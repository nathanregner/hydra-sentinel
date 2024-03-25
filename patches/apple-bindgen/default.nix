{ stdenv, fetchFromGitHub }:
stdenv.mkDerivation rec {
  pname = "apple-sys";
  version = src.rev;
  src = fetchFromGitHub {
    owner = "youknowone";
    repo = "apple-sys";
    rev = "master";
    hash = "sha256-NrF8JlO0PDOT1xMPVMMcm4kC6zQGc5UtZtBBcdiZhJY=";
  };
  patches = [ ./0001-disable-include-path-detection.patch ];
  dontBuild = true;
  installPhase = ''
    mkdir -p $out
    cp -r . $out
  '';
}
