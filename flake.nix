{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (nixpkgs) lib;
      in rec {
        packages.default = pkgs.callPackage ({ lib, rustPlatform, pkg-config
          , openssl, darwin, stdenv, fetchFromGitHub, fetchgit, cargo-typify
          , clang }:
          rustPlatform.buildRustPackage {
            pname = "hydra-hooks";
            version = "1.0.0";

            # TODO
            # src = lib.sources.sourceFilesBySuffices (lib.cleanSource ./.) [ ".nix" ];
            src = lib.cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkg-config ]
              ++ lib.optionals stdenv.isDarwin [
                darwin.apple_sdk.frameworks.SystemConfiguration
                clang
              ];
            buildInputs = [ openssl ];
          }) { };

        devShells.default = pkgs.mkShell {
          buildInputs = packages.default.nativeBuildInputs
            ++ packages.default.buildInputs ++ [ pkgs.rustfmt ];

          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          CARGO_CONFIG = (pkgs.formats.toml { }).generate "config.toml" {
            paths = [ (pkgs.callPackage ./patches/apple-bindgen { }) ];
          };
        };
      });
}
