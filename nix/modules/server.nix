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
              default = "60s";
              description = mdDoc ''
                TODO
              '';
            };

            build_machines = mkOption {
              type = types.listOf (types.submodule {
                options = {
                  hostName = mkOption {
                    type = types.str;
                    example = "nixbuilder.example.org";
                    description = lib.mdDoc ''
                      The hostname of the build machine.
                    '';
                  };
                  systems = mkOption {
                    type = types.listOf types.str;
                    example = [ "x86_64-linux" "aarch64-linux" ];
                    description = lib.mdDoc ''
                      The system types the build machine can execute derivations on.
                      Either this attribute or {var}`system` must be
                      present, where {var}`system` takes precedence if
                      both are set.
                    '';
                  };
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
                  sshKey = mkOption {
                    type = types.nullOr types.str;
                    default = null;
                    example = "/root/.ssh/id_buildhost_builduser";
                    description = lib.mdDoc ''
                      The path to the SSH private key with which to authenticate on
                      the build machine. The private key must not have a passphrase.
                      If null, the building user (root on NixOS machines) must have an
                      appropriate ssh configuration to log in non-interactively.

                      Note that for security reasons, this path must point to a file
                      in the local filesystem, *not* to the nix store.
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
                  publicHostKey = mkOption {
                    type = types.nullOr types.str;
                    default = null;
                    description = lib.mdDoc ''
                      The (base64-encoded) public host key of this builder. The field
                      is calculated via {command}`base64 -w0 /etc/ssh/ssh_host_type_key.pub`.
                      If null, SSH will use its regular known-hosts file when connecting.
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
