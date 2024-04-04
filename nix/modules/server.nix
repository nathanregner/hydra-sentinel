self:
{ config, lib, pkgs, ... }:
let
  json = pkgs.formats.json { };
  hydraCfg = config.services.hydra;
  cfg = config.services.hydra-sentinel-server;
in {
  options.services.hydra-sentinel-server =
    let inherit (lib) types mkOption mdDoc;
    in {
      enable = lib.mkEnableOption "Hydra Sentinel server daemon";

      package = lib.mkOption {
        type = types.package;
        default = self.packages."${pkgs.system}".server;
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
            github_webhook_secret_file = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = mdDoc ''
                TODO
              '';
            };

            hydra_base_url = mkOption {
              type = types.str;
              default = "http://127.0.0.1:${toString hydraCfg.port}";
              description = mdDoc ''
                TODO
              '';
            };

            hydra_machines_file = mkOption {
              type = types.path;
              default = "/var/lib/hydra/machines";
              description = mdDoc ''
                TODO
              '';
            };

            allowed_ips = mkOption {
              type = types.listOf types.str;
              default = [ ];
              example = [ "192.168.0.0/16" ];
              description = mdDoc ''
                CIDR notation
              '';
            };

            heartbeat_timeout = mkOption {
              type = types.str;
              default = "30s";
              description = mdDoc ''
                TODO
              '';
            };

            build_machines = mkOption {
              type = types.listOf (types.submodule {
                options = {
                  sshUser = mkOption {
                    type = types.nullOr types.str;
                    default = null;
                    example = "builder";
                    description = lib.mdDoc ''
                      The username to log in as on the remote host. This user must be
                      able to log in and run nix commands non-interactively. It must
                      also be privileged to build derivations, so must be included in
                      {option}`nix.settings.trusted-users`.
                    '';
                  };
                  hostname = mkOption {
                    type = types.str;
                    example = "nixbuilder.example.org";
                    description = lib.mdDoc ''
                      The hostname of the build machine.
                    '';
                  };
                  system = mkOption {
                    type = types.str;
                    example = "x86_64-linux";
                    description = lib.mdDoc ''
                      The system type the build machine can execute derivations on.
                      Either this attribute or {var}`systems` must be
                      present, where {var}`system` takes precedence if
                      both are set.
                    '';
                  };
                  maxJobs = mkOption {
                    type = types.int;
                    default = 1;
                    description = lib.mdDoc ''
                      The number of concurrent jobs the build machine supports. The
                      build machine will enforce its own limits, but this allows hydra
                      to schedule better since there is no work-stealing between build
                      machines.
                    '';
                  };
                  speedFactor = mkOption {
                    type = types.int;
                    default = 1;
                    description = lib.mdDoc ''
                      The relative speed of this builder. This is an arbitrary integer
                      that indicates the speed of this builder, relative to other
                      builders. Higher is faster.
                    '';
                  };
                  mandatoryFeatures = mkOption {
                    type = types.listOf types.str;
                    default = [ ];
                    example = [ "big-parallel" ];
                    description = lib.mdDoc ''
                      A list of features mandatory for this builder. The builder will
                      be ignored for derivations that don't require all features in
                      this list. All mandatory features are automatically included in
                      {var}`supportedFeatures`.
                    '';
                  };
                  supportedFeatures = mkOption {
                    type = types.listOf types.str;
                    default = [ ];
                    example = [ "kvm" "big-parallel" ];
                    description = lib.mdDoc ''
                      A list of features supported by this builder. The builder will
                      be ignored for derivations that require features not in this
                      list.
                    '';
                  };
                };
              });
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
    assertions = [{
      assertion = builtins.elem cfg.settings.hydra_machines_file
        hydraCfg.buildMachinesFiles;
      message =
        "services.hydra-sentinel.hydra_machines_file must be a member of services.hydra.buildMachinesFiles";
    }];

    users.users.hydra-sentinel-server = {
      description = "Hydra Sentinel Server";
      group = "hydra";
      isSystemUser = true;
      # home = "/var/lib/hydra-sentinel-server";
      # createHome = true;
    };

    systemd.tmpfiles.rules = [
      "f+ ${cfg.settings.hydra_machines_file} 0660 ${config.users.users.hydra-sentinel-server.name} ${config.users.users.hydra-sentinel-server.group} -"
    ];

    systemd.services.hydra-sentinel-server = {
      wantedBy = [ "multi-user.target" ];
      requires = [ "hydra-server.service" ];
      after = [ "hydra-server.service" "hydra-server.service" ];
      serviceConfig = let
        confFile = json.generate "config.json"
          ((lib.filterAttrs (_: v: v != null)
            (cfg.extraSettings // cfg.settings)) // {
              listen_addr = "${cfg.listenHost}:${toString cfg.listenPort}";
            });
      in {
        ExecStart = "${cfg.package}/bin/hydra-sentinel-server ${confFile}";
        User = "hydra-sentinel-server";
        Group = "hydra";
        Restart = "always";
      };
    };
  };
}
