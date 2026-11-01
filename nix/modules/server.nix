self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  json = pkgs.formats.json { };
  hydraCfg = config.services.hydra;
  cfg = config.services.hydra-sentinel-server;
in
{
  options.services.hydra-sentinel-server =
    let
      inherit (lib) types mkOption mdDoc;
    in
    {
      enable = lib.mkEnableOption "Hydra Sentinel server daemon";

      package = lib.mkOption {
        type = types.package;
        default = self.packages."${pkgs.stdenv.hostPlatform.system}".server;
      };

      listenHost = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = mdDoc "Host to listen on.";
      };

      listenPort = mkOption {
        type = types.int;
        default = 3001;
        description = mdDoc "Port to listen on.";
      };

      settings = lib.mkOption {
        type = types.submodule {
          freeformType = json.type;
          options = {
            githubWebhookSecretFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = mdDoc ''
                TODO
              '';
            };

            hydraBaseUrl = mkOption {
              type = types.str;
              default = "http://127.0.0.1:${toString hydraCfg.port}";
              description = mdDoc ''
                TODO
              '';
            };

            allowedIps = mkOption {
              type = types.listOf types.str;
              default = [ ];
              example = [ "192.168.0.0/16" ];
              description = mdDoc ''
                CIDR notation
              '';
            };

            heartbeatTimeout = mkOption {
              type = types.str;
              default = "60s";
              description = mdDoc ''
                TODO
              '';
            };

            buildMachines =
              let
                base = {
                  hostName = mkOption {
                    type = types.str;
                    example = "nixbuilder.example.org";
                    description = lib.mdDoc ''
                      The hostname of the build machine.
                    '';
                  };
                  systems = mkOption {
                    type = types.listOf types.str;
                    example = [
                      "x86_64-linux"
                      "aarch64-linux"
                    ];
                    description = lib.mdDoc ''
                      The system types the build machine can execute derivations on.
                    '';
                  };
                };
              in
              mkOption {
                type = types.listOf (
                  types.submodule {
                    options = base // {
                      vms = mkOption {
                        type = types.listOf (types.submodule { options = base; });
                        default = [ ];
                        description = lib.mdDoc ''
                          VMs hosted by this machine. The machine is woken when jobs are scheduled for their systems.
                        '';
                      };
                      macAddress = mkOption {
                        type = types.nullOr types.str;
                        default = null;
                        example = "00:11:22:33:44:55";
                        description = lib.mdDoc ''
                          If present, wake-on-lan will be attempted for this machine when matching jobs are scheduled.
                        '';
                      };
                    };
                  }
                );
                default = [ ];
                description = lib.mdDoc ''
                  TODO
                '';
              };
          };
        };
        default = { };
      };

      extraSettings = lib.mkOption {
        type = types.submodule { freeformType = json.type; };
        default = { };
      };
    };

  config = lib.mkIf cfg.enable {
    users.users.hydra-sentinel-server = {
      description = "Hydra Sentinel Server";
      group = "hydra";
      isSystemUser = true;
      # home = "/var/lib/hydra-sentinel-server";
      # createHome = true;
    };

    systemd.services.hydra-sentinel-server = {
      wantedBy = [ "multi-user.target" ];
      requires = [ "hydra-server.service" ];
      after = [
        "hydra-server.service"
        "hydra-server.service"
      ];
      serviceConfig =
        let
          confFile = json.generate "config.json" (
            (lib.filterAttrs (_: v: v != null) (cfg.extraSettings // cfg.settings))
            // {
              listenAddr = "${cfg.listenHost}:${toString cfg.listenPort}";
            }
          );
        in
        {
          ExecStart = "${cfg.package}/bin/hydra-sentinel-server ${confFile}";
          User = "hydra-sentinel-server";
          Group = "hydra";
          Restart = "always";
        };
    };
  };
}
