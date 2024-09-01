self:
{ pkgs, ... }:
{
  name = "client-server-connect";

  nodes.server =
    { config, ... }:
    {
      imports = [ self.outputs.nixosModules.server ];
      services.hydra = {
        enable = true;
        buildMachinesFiles = [
          config.services.hydra-sentinel-server.settings.hydraMachinesFile
        ];
        hydraURL = "http://localhost:${toString config.services.hydra.port}";
        notificationSender = "";
      };
      services.hydra-sentinel-server = {
        enable = true;
        listenHost = "0.0.0.0";
        listenPort = 3001;
        settings = {
          allowedIps = [ "192.168.0.0/16" ];
          githubWebhookSecretFile = pkgs.writeText "github_webhook_secret_file" "hocus pocus";
          buildMachines = [
            {
              hostName = "client";
              systems = [ "x86_64-linux" ];
              supportedFeatures = [
                "nixos-test"
                "benchmark"
                "big-parallel"
              ];
            }
          ];
        };
      };
      networking.firewall.allowedTCPPorts = [ config.services.hydra-sentinel-server.listenPort ];
    };

  nodes.client =
    { config, ... }:
    {
      imports = [ self.outputs.nixosModules.client ];
      services.hydra-sentinel-client = {
        enable = true;
        settings = {
          hostName = "client";
          serverAddr = "server:3001";
        };
      };
    };

  testScript =
    # python
    ''
      server.start()
      client.start()

      server.wait_for_unit("hydra-sentinel-server.service")
      client.wait_for_unit("hydra-sentinel-client.service")

      server.wait_until_succeeds("wc -l /var/lib/hydra/machines | gawk '{ if (! strtonum($1) > 0) { exit 1 } }'")

      expected = "ssh://client x86_64-linux - 1 1 benchmark,big-parallel,nixos-test - -"
      actual = server.succeed("cat /var/lib/hydra/machines").strip()
      print(f"got {actual!r}, expected {expected!r}")
      assert expected == actual
    '';
}
