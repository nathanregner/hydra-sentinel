self:
{ config, lib, pkgs, ... }:
let
  toml = pkgs.formats.toml { };
  cfg = config.services.hydra-sentinel-client;
in {
  options.services.hydra-sentinel-client = let inherit (lib) types mkOption;
  in {
    enable = lib.mkEnableOption "Hydra Sentinel client daemon";

    package = lib.mkOption {
      type = types.package;
      default = self.packages."${pkgs.system}".client;
    };

    settings = lib.mkOption {
      type = types.submodule {
        freeformType = toml.type;
        options = {
          server_addr = mkOption {
            type = types.str;
            example = "example.com:3002";
            description = lib.mdDoc ''
              The address of the Hydra Sentinel server.
            '';
          };
          hostname = mkOption {
            type = types.str;
            example = "rpi4";
            description = lib.mdDoc ''
              The hostname of this build machine.
            '';
          };
        };
      };
    };
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
      serviceConfig = let
        confFile = toml.generate "config.toml"
          (lib.filterAttrs (_: v: v != null) cfg.settings);
      in {
        ExecStart = "${cfg.package}/bin/hydra-sentinel-client ${confFile}";
        User = "hydra-sentinel-client";
        Restart = "always";
        RestartSec = 1;
        RestartSteps = 10;
        RestartMaxDelaySec = 60;
      };
    };
  };
}
