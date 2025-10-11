{
  self,
  config,
  pkgs,
  lib,
  cfg,
}:
let
  inherit (lib) types mkOption;
  json = pkgs.formats.json { };
in
{
  enable = lib.mkEnableOption "Hydra Sentinel client daemon";

  headless = lib.mkOption {
    type = types.bool;
    default = false;
  };

  package = lib.mkOption {
    type = types.package;
    default = self.packages."${pkgs.system}"."${if cfg.headless then "client-headless" else "client"}";
  };

  settings = lib.mkOption {
    type = types.submodule {
      freeformType = json.type;
      options = {
        serverAddr = mkOption {
          type = types.str;
          example = "example.com:3002";
          description = lib.mdDoc ''
            The address of the Hydra Sentinel server.
          '';
        };
        hostName = mkOption {
          type = types.str;
          example = "rpi4";
          default = config.networking.hostName;
          description = lib.mdDoc ''
            The hostname of this build machine.
          '';
        };
        heartbeatInterval = mkOption {
          type = types.str;
          default = "30s";
        };
      };
    };
  };
}
