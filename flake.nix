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
          , openssl, darwin, stdenv, fetchFromGitHub, fetchgit, cargo-typify }:
          rustPlatform.buildRustPackage {
            pname = "hydra-hooks";
            version = "1.0.0";

            # TODO
            # src = lib.sources.sourceFilesBySuffices (lib.cleanSource ./.) [ ".nix" ];
            src = lib.cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkg-config openssl ];
            buildInputs = [ openssl ] ++ lib.optionals stdenv.isDarwin
              [ darwin.apple_sdk.frameworks.SystemConfiguration ];

          }) { };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = packages.default.nativeBuildInputs or [ ]
            ++ [ pkgs.rustfmt ];

          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
}
