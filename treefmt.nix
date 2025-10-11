{ rustfmt }:
{
  projectRootFile = "flake.nix";
  programs = {
    nixfmt.enable = true;
    rustfmt = {
      enable = true;
      package = rustfmt;
    };
    statix.enable = true;
    taplo.enable = true;
  };
}
