self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  json = pkgs.formats.json { };
  cfg = config.services.hydra-sentinel-client;
in
{
  options.services.hydra-sentinel-client = import ./options.nix {
    inherit
      self
      config
      pkgs
      lib
      ;
  };

  config = lib.mkIf cfg.enable {
    users = {
      users.hydra-sentinel-client = {
        description = "Hydra Sentinel client";
        group = "hydra-sentinel-client";
        isSystemUser = true;
      };
      groups.hydra-sentinel-client = { };
    };

    systemd.services.hydra-sentinel-client = {
      wantedBy = [ "multi-user.target" ];
      bindsTo = [ "network-online.target" ];
      after = [ "network-online.target" ];
      serviceConfig =
        let
          confFile = json.generate "config.json" (lib.filterAttrs (_: v: v != null) cfg.settings);
        in
        {
          ExecStart = "${cfg.package}/bin/hydra-sentinel-client ${confFile}";
          # TODO: is there any way to run this as a system-level, non-root service with DBus access?
          # User = "hydra-sentinel-client";
          Restart = "always";
          RestartSec = 1;
          RestartSteps = 10;
          RestartMaxDelaySec = 60;
        };
    };
  };
}
