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
        ;
    }
    // {
      logFile = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = "/var/log/hydra-sentinel-client.log";
      };
    };

  config = lib.mkIf cfg.enable (
    let
      user = config.users.users._hydra-sentinel-client;
    in
    {
      users = {
        users._hydra-sentinel-client = {
          description = "Hydra Sentinel client service user";
          uid = 3002;
        };
        knownUsers = [ user.name ];
      };

      system.activationScripts.preActivation.text = ''
        touch '${cfg.logFile}'
        chmod 0644 '${cfg.logFile}'
        chown ${toString user.uid} '${cfg.logFile}'
      '';

      launchd.daemons.hydra-sentinel-client =
        let
          configFile = json.generate "config.json" (lib.filterAttrs (_: v: v != null) cfg.settings);
        in
        {
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
    }
  );
}
