# The service

The daemon does nothing useful on its own: it has to run from boot so the limits the SMC
publishes are applied before anything loads the machine. There is one unit for it, kept in two
places.

| Place | Who uses it |
| ----- | ----------- |
| `nixosModules.macsmc-zonepower` in `flake.nix` | NixOS |
| `dist/macsmc-zonepower.service` | everything else |

Both carry the same config, and `nix flake check` fails if they drift apart.

## NixOS

```nix
{
  services.macsmc-zonepower.enable = true;
}
```

The package the module runs defaults to the one this flake builds, so
`services.macsmc-zonepower.package` is there when you want a different one.

## Other distros

```sh
cargo build --release
sudo install -m 755 target/release/macsmc-zonepower /usr/local/bin/
sudo install -m 644 dist/macsmc-zonepower.service /etc/systemd/system/
sudo systemctl enable --now macsmc-zonepower.service
```

`ExecStart=` in `dist/macsmc-zonepower.service` assumes the binary is in
`/usr/local/bin`.

## What the unit needs

The unit only starts when the macsmc driver has registered its platform device, so it stays
off on machines that have nothing to read limits from. What it reads and writes once it runs:

| Path                                                       | Access |
| ---------------------------------------------------------- | ------ |
| `/sys/bus/platform/devices/APP0001:00/plimit`              | read   |
| `/sys/module/intel_powerclamp/parameters/max_idle`         | read   |
| `/sys/devices/system/cpu/cpufreq/policy*/scaling_max_freq` | write  |
| `/sys/class/drm/card*/gt_max_freq_mhz`                     | write  |
| `/sys/class/drm/card*/gt_boost_freq_mhz`                   | write  |
| `/sys/class/thermal/cooling_device*/cur_state`             | write  |

Those attributes are mode 0644 and owned by root, and the kernel lets the owner write them
without any capability. So the daemon runs as root but with an empty
`CapabilityBoundingSet=`: it can write exactly what a root process may write, and it can do
nothing that would need a capability, no modules, no network, no mounting, no clock changes.
`User=`, `DynamicUser=` and `PrivateUsers=` are off for the same reason that keeps it root, they
would take away the ownership the writes depend on.

`ProtectKernelTunables=` is the one hardening option that stays off, it would make all of
`/sys` read only. `ProtectSystem=strict` still makes the rest of the file system read only and
leaves `/sys` alone. Everything else is on: private `/tmp`, a private `/dev` with a closed
device policy, no network and no socket families at all, other people's processes hidden from
`/proc`, module loading blocked, personality and clock and hostname frozen, no new privileges,
no setuid or setgid files, a seccomp allow list of `@system-service` minus the groups the
daemon never uses, native syscalls only, and `0077` as the umask.

`@io-event` is the exception, it stays allowed even though the daemon has no event loop. The
Rust runtime calls `poll()` before `main()` to check that file descriptors 0, 1 and 2 are
open, and aborts on an error it does not expect. Blacklisting the group makes that probe fail
with `EPERM`, and the daemon then dies at startup with an abort and a core dump without ever
reaching its own code.

The daemon exits if it cannot read or write any of those paths, and the unit restarts it. The
restart rate is capped, so a machine that never grows the paths it needs ends up with the unit
in `failed` instead of a daemon that spins.

## Stopping

`SIGTERM` and `SIGINT` stop the daemon. Before it goes it writes a zero limit to every zone, so
`systemctl stop`, `systemctl restart` or Ctrl+C leave the CPU, the iGPU and idle CPU time
uncapped instead of throttled at whatever the last limits were.

The signal only raises a flag and the daemon checks it between polls, so it stops within one
poll of the signal. Installing the handler is a `rt_sigaction` call, which is in `@signal`, and
`@system-service` already allows that group, so the filter needs nothing new.
