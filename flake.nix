{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-parts,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];
      imports = [ inputs.treefmt-nix.flakeModule ];

      perSystem =
        {
          config,
          pkgs,
          lib,
          ...
        }@perSystem:
        (
          let
            libs =
              with pkgs;
              (lib.optionals stdenv.isLinux [
                libGL
                libappindicator
                libayatana-appindicator
                libxkbcommon
                wayland
              ]);

            commonArgs = {
              version = "0.1.0";
              src = lib.fileset.toSource {
                root = ./.;
                fileset = lib.fileset.unions [
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

              nativeBuildInputs =
                with pkgs;
                [ pkg-config ]
                ++ lib.optionals stdenv.isDarwin [
                  xcbuild
                ]
                ++ lib.optionals stdenv.isLinux [
                  gtk3.dev
                  makeWrapper
                ];

              buildInputs =
                with pkgs;
                libs
                ++ [ openssl.dev ]
                ++ lib.optionals stdenv.isLinux [
                  gtk3.dev
                  xdotool
                ];

              cargoLock.lockFile = ./Cargo.lock;
            };

            client = pkgs.rustPlatform.buildRustPackage (
              commonArgs
              // rec {
                pname = "hydra-sentinel-client";
                cargoBuildFlags = [ "--package ${pname}" ];
                cargoTestFlags = cargoBuildFlags;
                postFixup = lib.optionalString pkgs.stdenv.isLinux ''
                  wrapProgram $out/bin/hydra-sentinel-client \
                    --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath libs}
                '';
              }
            );

            client-headless = pkgs.rustPlatform.buildRustPackage (
              commonArgs
              // rec {
                pname = "hydra-sentinel-client";
                cargoBuildFlags = [
                  "--package ${pname}"
                  "--no-default-features"
                ];
                cargoTestFlags = cargoBuildFlags;
              }
            );

            server = pkgs.rustPlatform.buildRustPackage (
              commonArgs
              // rec {
                pname = "hydra-sentinel-server";
                cargoBuildFlags = [ "--package ${pname}" ];
                cargoTestFlags = cargoBuildFlags;
              }
            );
          in
          {
            packages = {
              inherit client client-headless server;
            };

            devShells.default = pkgs.mkShell {
              inputsFrom = [
                config.treefmt.build.devShell
              ];
              packages =
                commonArgs.nativeBuildInputs
                ++ commonArgs.buildInputs
                ++ (with pkgs; [
                  cargo
                  cargo-nextest
                  cargo-watch
                  clippy
                  rust-analyzer
                  rustfmt
                ]);
              env = {
                RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
                LD_LIBRARY_PATH = lib.makeLibraryPath libs;
              };
            };

            treefmt = import ./treefmt.nix { inherit (pkgs) rustfmt; };

            checks = builtins.listToAttrs (
              builtins.map
                (
                  path:
                  let
                    args = (import path) self perSystem;
                  in
                  {
                    inherit (args) name;
                    value = pkgs.testers.runNixOSTest args;
                  }
                )
                [
                  ./nix/tests/connect.nix
                ]
            );
          }
        );

      flake = {
        overlays.default =
          _: prev:
          let
            inherit (prev.stdenv.hostPlatform) system;
          in
          {
            hydra-sentinel-client = self.packages.${system}.client;
            hydra-sentinel-client-headless = self.packages.${system}.client-headless;
            hydra-sentinel-server = self.packages.${system}.server;
          };

        nixosModules = {
          server = import ./nix/modules/server.nix { inherit (self) packages; };
          client = import ./nix/modules/client/nixos.nix { inherit (self) packages; };
        };

        darwinModules = {
          client = import ./nix/modules/client/darwin.nix { inherit (self) packages; };
        };

        hydraJobs = {
          checks = self.checks.x86_64-linux;
          inherit (self) packages;
        };
      };
    };
}
