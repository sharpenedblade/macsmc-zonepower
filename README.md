# macsmc-zonepower

Daemon implementing the userspace side of smc power limits on t2 macs.

## Installation

1. Build with `cargo build --release` and copy `./target/release/macsmc-zonepower` to `/usr/local/bin/macsmc-zonepower`.
2. Copy `dist/macsmc-zonepower.service` to `/etc/systemd/system/`.
3. Run `systemctl enable --now macsmc-zonepower`.

### On NixOS

1. Add this repo as a flake input.
2. Add `services.macsmc-zonepower.enabled = true;` to your config.

## Credits

The algorithm and significant parts of the code were copied from or
heavily influenced by https://github.com/kaiT2en/KaiT2en-Fedora/tree/main/apps/t2-zone-power
