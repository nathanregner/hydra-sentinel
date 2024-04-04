{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (nixpkgs) lib;
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.lib.${system};

        inherit (pkgs) stdenv openssl pkg-config darwin clang;
        commonArgs = {
          version = "0.0.0";
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
            ];
          };

          nativeBuildInputs = [ pkg-config ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.SystemConfiguration
            clang
          ];
          buildInputs = [ openssl ];
          useNextest = true;
        };

        cargoArtifacts =
          craneLib.buildDepsOnly (commonArgs // { pname = "hydra-sentinel"; });

        client = craneLib.buildPackage (commonArgs // rec {
          inherit cargoArtifacts;
          pname = "hydra-sentinel-client";
          cargoExtraArgs = "-p ${pname}";
        });

        server = craneLib.buildPackage (commonArgs // rec {
          inherit cargoArtifacts;
          pname = "hydra-sentinel-server";
          cargoExtraArgs = "-p ${pname}";
        });

      in {
        packages = { inherit client server; };

        devShells.default = craneLib.devShell {
          buildInputs = (with pkgs;
            [ rustfmt cargo-watch ] ++ commonArgs.buildInputs
            ++ commonArgs.nativeBuildInputs);

          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          CARGO_CONFIG = (pkgs.formats.toml { }).generate "config.toml" {
            # Janky workaround to fix duplicate SDK include paths from impure environments.
            # Even removing darwin.apple_sdk.frameworks.SystemConfiguration from the shell
            # doesn't seem to fix it
            paths = (lib.optionals pkgs.stdenv.isDarwin
              (pkgs.callPackage ./patches/apple-bindgen { }));
          };
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
                  hostname = "client";
                  system = "x86_64-linux";
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
                hostname = "client";
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

            expected = "ssh://client x86_64-linux 1 1 benchmark,big-parallel,nixos-test -"
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
      };
}
