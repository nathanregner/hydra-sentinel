# Hydra Sentinel

This flake provides `NixOS` and `nix-darwin` modules to automatically wake
[Hydra](https://github.com/NixOS/hydra) builders when builds are queued for a
matching system type, and keep them awake until the queue drains.

Builders register themselves with Hydra by running `hydra-builder`, which
connects to the queue runner over gRPC; Sentinel only manages their power state.

Hydra aborts builds for a system once no connected builder has supported it for
`services.hydra.queueRunner.settings.maxUnsupportedTimeInS`. Set this longer than
your builders take to wake.

## Usage

Refer to [./nix/tests](./nix/tests/) for a full example.

## Security

No security is provided; nodes should communicate over the local network or a
VPN like Tailscale.

## TODO

- Tests for non-headless systems, darwin
- Better docs
