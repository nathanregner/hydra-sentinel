{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (nixpkgs) lib;
        pkgs = nixpkgs.legacyPackages.${system};

        commonArgs = {
          version = "0.1.0";
          src = let inherit (lib) fileset;
          in fileset.toSource {
            root = ./.;
            fileset = fileset.unions [
              #
              ./Cargo.lock
              ./Cargo.toml
              ./client
              ./lib
              ./server
              ./vendor
            ];
          };

          env = lib.optionalAttrs pkgs.stdenv.isDarwin {
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            # RUST_BACKTRACE = "1";
            # CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG = "true";
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            xcbuild
            # rustPlatform.bindgenHook
          ];
          buildInputs = with pkgs;
            [ openssl ] ++ (lib.optionals stdenv.isDarwin
              (with darwin.apple_sdk.frameworks; [
                CoreFoundation
                SystemConfiguration
                IOKit
                libiconv
              ]));

          cargoLock.lockFile = ./Cargo.lock;
        };

        client = pkgs.rustPlatform.buildRustPackage (commonArgs // rec {
          pname = "hydra-sentinel-client";
          cargoBuildFlags = [ "--package ${pname}" ];
        });

        server = pkgs.rustPlatform.buildRustPackage (commonArgs // rec {
          pname = "hydra-sentinel-server";
          cargoBuildFlags = [ "--package ${pname}" ];
        });

      in {
        packages = {
          inherit client server;
          test = pkgs.stdenv.mkDerivation ({
            pname = "hydra-sentinel-client";
            version = "1.1.2";

            src = ./.;

            nativeBuildInputs = with pkgs; [
              pkg-config
              rustPlatform.bindgenHook
              xcbuild
            ];
            buildInputs = with pkgs;
              [ openssl ] ++ (lib.optionals stdenv.isDarwin
                (with darwin.apple_sdk.frameworks; [
                  clang
                  SystemConfiguration
                  IOKit
                  libiconv
                ]));
            buildPhase = ''
              xcrun --sdk macosx --show-sdk-path > $out
            '';
          });

        };

        devShells.default = pkgs.mkShell {
          inherit (commonArgs) buildInputs;
          packages = (with pkgs; [
            cargo
            cargo-watch
            clippy
            rust-bindgen
            rustc
            rustfmt
          ]);

          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        checks.vm = pkgs.testers.runNixOSTest {
          name = "client-server-connect";

          nodes.server = { config, ... }: {
            imports = [ self.outputs.nixosModules.server ];
            services.hydra = {
              enable = true;
              buildMachinesFiles = [
                config.services.hydra-sentinel-server.settings.hydra_machines_file
              ];
              hydraURL =
                "http://localhost:${toString config.services.hydra.port}";
              notificationSender = "";
            };
            services.hydra-sentinel-server = {
              enable = true;
              listenHost = "0.0.0.0";
              listenPort = 3001;
              settings = {
                allowed_ips = [ "192.168.0.0/16" ];
                github_webhook_secret_file =
                  pkgs.writeText "github_webhook_secret_file" "hocus pocus";
                build_machines = [{
                  hostName = "client";
                  systems = [ "x86_64-linux" ];
                  supportedFeatures =
                    [ "nixos-test" "benchmark" "big-parallel" ];
                }];
              };
            };
            networking.firewall.allowedTCPPorts =
              [ config.services.hydra-sentinel-server.listenPort ];
          };

          nodes.client = { config, ... }: {
            imports = [ self.outputs.nixosModules.client ];
            services.hydra-sentinel-client = {
              enable = true;
              settings = {
                hostName = "client";
                server_addr = "server:3001";
              };
            };
          };

          testScript = ''
            server.start()
            client.start()

            server.wait_for_unit("hydra-sentinel-server.service")
            client.wait_for_unit("hydra-sentinel-client.service")

            server.wait_until_succeeds("wc -l /var/lib/hydra/machines | gawk '{ if (! strtonum($1) > 0) { exit 1 } }'")

            expected = "ssh://client x86_64-linux - 1 1 benchmark,big-parallel,nixos-test - -"
            actual = server.succeed("cat /var/lib/hydra/machines").strip()
            print(f"got {actual!r}, expected {expected!r}")
            assert expected == actual
          '';
        };

      })) // {
        overlays.default = _: prev: {
          hydra-sentinel-client = self.packages.${prev.system}.client;
          hydra-sentinel-server = self.packages.${prev.system}.server;
        };

        nixosModules = {
          server = import ./nix/modules/server.nix { inherit (self) packages; };
          client =
            import ./nix/modules/client/nixos.nix { inherit (self) packages; };
        };

        darwinModules = {
          client =
            import ./nix/modules/client/darwin.nix { inherit (self) packages; };
        };
      };
}
