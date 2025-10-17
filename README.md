# Hydra Sentinel

This flake provides `NixOS` and `nix-darwin` modules to automatically wake
[nix builders](https://nix.dev/manual/nix/2.25/advanced-topics/distributed-builds)
when [Hydra](https://github.com/NixOS/hydra) has builds queued for a matching
system type.

## Usage

Refer to [./nix/tests](./nix/tests/) for a full example.

## Security

No security is provided; nodes should communicate over the local network or a
VPN like Tailscale.

## TODO

- Tests for non-headless systems, darwin
- Better docs
- Check newest queued, ignore if stalled
