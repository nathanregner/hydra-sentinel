{
  self,
  config,
  lib,
  pkgs,
}:
let
  inherit (lib) types mkOption;
  json = pkgs.formats.json { };
in
{
  enable = lib.mkEnableOption "Hydra Sentinel client daemon";

  package = lib.mkOption {
    type = types.package;
    default = self.packages."${pkgs.system}".client;
  };

  settings = lib.mkOption {
    type = types.submodule {
      freeformType = json.type;
      options = {
        server_addr = mkOption {
          type = types.str;
          example = "example.com:3002";
          description = lib.mdDoc ''
            The address of the Hydra Sentinel server.
          '';
        };
        host_name = mkOption {
          type = types.str;
          example = "rpi4";
          default = config.networking.hostName;
          description = lib.mdDoc ''
            The hostname of this build machine.
          '';
        };
        heartbeat_interval = mkOption {
          type = types.str;
          default = "30s";
        };
      };
    };
  };
}
