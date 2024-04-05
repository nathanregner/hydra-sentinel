self:
{ config, lib, pkgs, ... }:
let
  json = pkgs.formats.json { };
  cfg = config.services.hydra-sentinel-client;
in {
  options.services.hydra-sentinel-client =
    import ./options.nix { inherit self config pkgs lib; };

  config = lib.mkIf cfg.enable {
    users = {
      users.hydra-sentinel-client = { description = "Hydra Sentinel client"; };
    };

    launchd.daemons.hydra-sentinel-client = let
      configFile = json.generate "config.json"
        (lib.filterAttrs (_: v: v != null) cfg.settings);
    in {
      serviceConfig = {
        ProgramArguments = [
          "/bin/sh"
          "-c"
          "${cfg.package}/bin/hydra-sentinel-client"
          (toString configFile)
        ];
        UserName = "hydra-sentinel-client";
        RunAtLoad = true;
        KeepAlive = true;
      };
    };
  };
}
