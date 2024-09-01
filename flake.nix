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
      nixpkgs,
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
          system,
          inputs',
          pkgs,
          lib,
          ...
        }@perSystem:
        (
          let
            commonArgs = {
              version = "0.1.0";
              src =
                let
                  inherit (lib) fileset;
                in
                fileset.toSource {
                  root = ./.;
                  fileset = fileset.unions [
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
              buildInputs =
                with pkgs;
                [ openssl ]
                ++ (lib.optionals stdenv.isDarwin (
                  with darwin.apple_sdk.frameworks;
                  [
                    CoreFoundation
                    SystemConfiguration
                    IOKit
                    libiconv
                  ]
                ));

              cargoLock.lockFile = ./Cargo.lock;
            };

            client = pkgs.rustPlatform.buildRustPackage (
              commonArgs
              // rec {
                pname = "hydra-sentinel-client";
                cargoBuildFlags = [ "--package ${pname}" ];
              }
            );

            server = pkgs.rustPlatform.buildRustPackage (
              commonArgs
              // rec {
                pname = "hydra-sentinel-server";
                cargoBuildFlags = [ "--package ${pname}" ];
              }
            );

          in
          {
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
                buildInputs =
                  with pkgs;
                  [ openssl ]
                  ++ (lib.optionals stdenv.isDarwin (
                    with darwin.apple_sdk.frameworks;
                    [
                      clang
                      SystemConfiguration
                      IOKit
                      libiconv
                    ]
                  ));
                buildPhase = ''
                  xcrun --sdk macosx --show-sdk-path >$out
                '';
              });

            };

            devShells.default = pkgs.mkShell {
              inherit (commonArgs) nativeBuildInputs buildInputs;
              packages = (
                with pkgs;
                [
                  cargo
                  cargo-nextest
                  cargo-watch
                  clippy
                  config.treefmt.build.wrapper
                  rust-analyzer
                  rust-bindgen
                  rustc
                  rustfmt
                ]
              );

              LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
              RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
            };

            treefmt = import ./treefmt.nix;

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
        overlays.default = _: prev: {
          hydra-sentinel-client = self.packages.${prev.system}.client;
          hydra-sentinel-server = self.packages.${prev.system}.server;
        };

        nixosModules = {
          server = import ./nix/modules/server.nix { inherit (self) packages; };
          client = import ./nix/modules/client/nixos.nix { inherit (self) packages; };
        };

        darwinModules = {
          client = import ./nix/modules/client/darwin.nix { inherit (self) packages; };
        };
      };
    };

}
