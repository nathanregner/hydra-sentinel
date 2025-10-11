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
  options.services.hydra-sentinel-client =
    import ./options.nix {
      inherit
        self
        config
        pkgs
        lib
        cfg
        ;
    }
    // {
      logFile = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        # TODO: write somewhere sane
        # https://github.com/LnL7/nix-darwin/issues/460
        default = "/tmp/hydra-sentinel-client.log";
      };
    };

  config = lib.mkIf cfg.enable (
    let
      user = config.users.users._hydra-sentinel-client;
      configFile = json.generate "config.json" (lib.filterAttrs (_: v: v != null) cfg.settings);
    in
    {
      users = lib.mkIf cfg.headless {
        users._hydra-sentinel-client = {
          description = "Hydra Sentinel client service user";
          uid = 3002;
        };
        knownUsers = [ user.name ];
      };

      system.activationScripts.preActivation.text = ''
        touch '${cfg.logFile}'
        ${lib.optionalString (pkgs.stdenv.isDarwin && cfg.headless) ''
          chown ${toString user.uid} '${cfg.logFile}'
        ''}
        chmod 0644 '${cfg.logFile}'
      '';

      launchd.daemons.hydra-sentinel-client = lib.mkIf cfg.headless {
        script = ''
          "${cfg.package}/bin/hydra-sentinel-client" ${toString configFile}
        '';
        serviceConfig = {
          UserName = user.name;
          KeepAlive = true;
          RunAtLoad = true;
          StandardOutPath = cfg.logFile;
          StandardErrorPath = cfg.logFile;
        };
      };

      launchd.user.agents.hydra-sentinel-client = lib.mkIf (!cfg.headless) {
        script = ''
          "${cfg.package}/bin/hydra-sentinel-client" ${toString configFile}
        '';
        serviceConfig = {
          KeepAlive = true;
          RunAtLoad = true;
          StandardOutPath = cfg.logFile;
          StandardErrorPath = cfg.logFile;
        };
      };
    }

  );
}
