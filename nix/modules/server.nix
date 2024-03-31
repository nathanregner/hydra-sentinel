{ config, lib, pkgs }:
with lib;
let
  toml = pkgs.formats.toml { };
  hydraCfg = config.services.hydra;
  cfg = config.services.hydra-sentinel;
  baseDir = cfg.baseDir;
in {
  options.services.hydra-sentinel-server = {
    enable = lib.mkEnableOption "Hydra Sentinel daemon";

    settings = lib.mkOption {
      type = types.submodule {
        freeformType = toml.type;
        options = {
          listenAddr = mkOption {
            type = types.str;
            default = "0.0.0.0:3002";
            description = mdDoc ''
              An internet socket address to listen on, either IPv4 or IPv6.
            '';
          };

          githubWebhookSecretFile = mkOption {
            type = types.path;
            description = mdDoc ''
              TODO
            '';
          };

          hydraBaseUrl = mkOption {
            type = types.string;
            default = "http://${hydraCfg.listenHost}:${hydraCfg.listenPort}";
            description = mdDoc ''
              TODO
            '';
          };

          hydraMachinesFile = mkOption {
            type = types.path;
            default = "http://${hydraCfg.listenHost}:${hydraCfg.listenPort}";
            description = mdDoc ''
              TODO
            '';
          };

          allowedIps = mkOption {
            type = types.listOf types.string;
            default = [ ];
            example = [ "192.168.0.0/16" ];
            description = mdDoc ''
              CIDR notation
            '';
          };

          replyTimeout = mkOption {
            type = types.string;
            default = "30s";
            description = mdDoc ''
              TODO
            '';
          };

          buildMachines = mkOption {
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
                hostName = mkOption {
                  type = types.str;
                  example = "nixbuilder.example.org";
                  description = lib.mdDoc ''
                    The hostname of the build machine.
                  '';
                };
                system = mkOption {
                  type = types.nullOr types.str;
                  default = null;
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
  };

  config = mkIf cfg.enable {
    assertions = [{
      assertion = builtins.elem cfg.hydraMachinesFile hydraCfg.buildMachines;
      message =
        "services.hydra-sentinel.hydraMachinesFile must be a member of services.hydra.buildMachinesFiles";
    }];

    users.users.hydra-sentinel = {
      description = "Hydra Sentinel";
      group = "hydra";
      home = baseDir;
      # We don't enable `createHome` here because the creation of the home directory is handled by the hydra-init service below.
      uid = config.ids.uids.hydra;
    };

    systemd.services.hydra-sentinel-server = {
      wantedBy = [ "multi-user.target" ];
      requires = [ "hydra-server.service" ];
      after = [ "hydra-server.service" ];
      restartTriggers = [ sentinelConf ];
      serviceConfig =
        let confFile = tomlFormat.generate "config.toml" cfg.settings;
        in {
          ExecStart = "${pkgs.goeland}/bin/goeland run -c ${confFile}";
          User = "hydra-sentinel";
          Group = "hydra";
          Restart = "always";
        };
    };
  };
}
