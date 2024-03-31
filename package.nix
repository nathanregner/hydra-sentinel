{ lib, rustPlatform, pkg-config, openssl, darwin, stdenv, clang }:
let cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
in rustPlatform.buildRustPackage {
  pname = "hydra-hooks";
  version = "1.0.0";
  inherit cargoDeps;

  # TODO
  # src = lib.sources.sourceFilesBySuffices (lib.cleanSource ./.) [ ".nix" ];
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
}
