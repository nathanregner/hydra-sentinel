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
    flake-utils.lib.eachDefaultSystem (system:
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
              ./protocol
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
          buildInputs = (with pkgs; [ rustfmt cargo-watch ]);

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
      });
}
