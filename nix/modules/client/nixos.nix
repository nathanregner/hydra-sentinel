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

  confFile = json.generate "config.json" (lib.filterAttrs (_: v: v != null) cfg.settings);
in
{
  options.services.hydra-sentinel-client = import ./options.nix {
    inherit
      self
      config
      pkgs
      lib
      cfg
      ;
  };

  config = lib.mkIf cfg.enable {
    users = lib.mkIf cfg.headless {
      users.hydra-sentinel-client = {
        description = "Hydra Sentinel client";
        group = "hydra-sentinel-client";
        isSystemUser = true;
      };
      groups.hydra-sentinel-client = { };
    };

    systemd.services.hydra-sentinel-client-headless = lib.mkIf cfg.headless {
      wantedBy = [ "multi-user.target" ];
      bindsTo = [ "network-online.target" ];
      after = [ "network-online.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/hydra-sentinel-client ${confFile}";
        Restart = "always";
        RestartSec = 1;
        RestartSteps = 10;
        RestartMaxDelaySec = 60;
      };
    };

    systemd.user.services.hydra-sentinel-client = lib.mkIf (!cfg.headless) {
      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/hydra-sentinel-client ${confFile}";
        Restart = "always";
        RestartSec = 1;
        RestartSteps = 10;
        RestartMaxDelaySec = 60;
      };
    };
  };
}
