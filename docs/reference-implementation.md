# The reference implementation

[t2-zone-power][zonepower] is the daemon this project is built on top of, and
[iurjscsi1101500/t2-zone-power][adaptation] is its port from the KaiT2en
`t2smc` driver to macsmc. Both are Rust, both are GPL-3.0-or-later, and the
same copyright notice in [NOTICES.md](../NOTICES.md) covers them. Reading them
is the cheapest way to learn what a T2 Mac expects, because they are the only
code that has run on one.

Same author, two names: the KaiT2en app is the original, the macsmc port drops
its GUI and swaps the sysfs files of `t2smc` for the macsmc interfaces.

## Two directions, two pipes

| Direction | Path | What moves |
| --------- | ---- | ---------- |
| SMC to daemon | `plimit` in sysfs | the limit steps, polled every 250 ms |
| daemon to SMC | `/dev/smc` ioctl | the ladder lengths, written once at start |

Everything that actually slows the machine down is a sysfs write: the CPU, both
GPUs and idle CPU time. Only four keys go back into the SMC.

## What it writes

| Key | Value | Written when |
| --- | ----- | ------------ |
| `MSTS` | turbo P-state count | at start |
| `MSPC` | non turbo P-state count above the CPU floor | at start, after `MSTS` |
| `HI0N` | iGPU state count | at start, if it found an iGPU |
| `HE0N` | dGPU state count, 16 | at start, and every 10 s until a dGPU shows up |
| `HE2N` | dGPU state count, 16 | on the machines that have the key |

It picks the GPU key by reading `HE2N` first: if the read fails the machine
does not have it, and `HE0N` is the dGPU slot instead of the iGPU slot. Same
rule as the KaiT2en driver, see [step counts](step-counts.md).

The counts are clamped to 1 to 127, and the port never writes `QENA`.

## How it writes them

One ioctl per key on `/dev/smc`: `IOCTL_SMC_WRITE` is number 2 with the magic
`'S'`, and it takes the key as four characters packed into a `u32` in reading
order (`HI0N` is `0x4849304E`), the size, and a buffer of 32 bytes. `size` is 1,
the count is the first byte of the buffer.

The payload has to match the driver byte for byte, and the two drivers do not
agree:

| Driver | Payload | Size |
| ------ | ------- | ---- |
| macsmc, patch `3014` | `{ u32 key; u32 size; u8 buf[32]; }` | 40 |
| KaiT2en `t2smc` | `{ u32 key; size_t size; u8 buf[32]; }` | 48 |

The size is part of the ioctl number, so a payload of the wrong size does not
just fail, the command is not even recognised and the driver answers `EINVAL`.
The macsmc port kept the `t2smc` layout while reading `plimit` from macsmc, so
on a t2-patches kernel none of its writes reach the SMC, and it prints one
error per attempt. Use the 40 byte layout with macsmc.

## How it counts

The dGPU count is not measured. It is a constant 16, because AGPM drives every
Mac dGPU through PowerTune states 0 to 15.

The iGPU count comes from i915:

```
steps = (RP0 - RPn) / 50 + 1
```

The CPU counts come from cpufreq and from a table of the `CPUFloor` of each
board plist, keyed by `product_name`. Four numbers go in: the lowest clock, the
top of turbo, the base clock, and the floor.

```
states    = (turbo max - lowest) / 100 + 1
turbo     = (turbo max - base) / 100
non turbo = states - turbo
floor     = (CPUFloor - lowest) / 100, the rungs the floor makes unreachable
```

Then, the way `X86PlatformPlugin::setCPUFloor` is read:

| Case | `MSTS` | `MSPC` |
| ---- | ------ | ------ |
| floor below the lowest clock, or no non turbo rung | nothing is written | |
| exactly one non turbo rung | `turbo - floor` | 1 |
| floor inside the non turbo rungs | `turbo` | `non turbo - floor` |
| floor inside the turbo rungs | `states - floor - 1` | 1 |

On a MacBookPro16,1 with 800 MHz to 4.6 GHz, base 2.6 GHz and a floor of
1000 MHz: 39 rungs, 20 of them turbo, 19 below, and the floor takes 2. That
gives `MSTS` 20 and `MSPC` 17, and that machine holds 19 and 17. It is one of
the few derivations that gets a real pair almost right, and `MSTS` is still one
over, see [step counts](step-counts.md).

A `Macmini8,1` is not in the floor table, which is right: that plist has no
`CPUFloor`, and macOS reports no floor there either.

## Where the numbers come from

| Input | Path |
| ----- | ---- |
| board model, for the CPU floor | `/sys/class/dmi/id/product_name` |
| lowest clock, top of turbo, base clock | `cpuinfo_min_freq`, `cpuinfo_max_freq`, `base_frequency` under `policy0` |
| iGPU ladder | `gt_RP0_freq_mhz`, `gt_RPn_freq_mhz` under a `card*` |
| dGPU ladder | nothing, it is a constant |
| dGPU floor | `device/device` of the amdgpu device, a PCI id: Navi 1x floor is 40 % of TDP, Polaris is 0 |
| dGPU TDP | `power1_cap_default` of the amdgpu hwmon device, in microW |
| whether a key exists | `IOCTL_SMC_GET_KEY_INFO` |

The model name match is a prefix match, and `base_frequency` only exists on
intel_pstate and amd_pstate.

## Where it is weak

It is worth knowing before copying it.

- The counts are written once, at start. The SMC loses them over sleep and
  nothing in the daemon puts them back, so after a suspend the machine is in
  the state [step counts](step-counts.md) describes as "Linux today": the SMC
  balances the heatpipe on the CPU alone.
- It never reads a count before overwriting it. The SMC ships values that look
  right for the machine, and the port replaces them with a computed guess,
  including the `MSTS` that is one over.
- The dGPU cap follows the PowerTune table linearly from the TDP to the floor,
  which is what the macOS drivers fill in, but the floor table only knows Navi
  1x and Polaris. A Vega or an RDNA 2 dGPU is left alone entirely.
- A runtime suspended dGPU is skipped, and the cap is not re-applied until the
  step changes, so a dGPU that wakes between two changes runs uncapped.

[zonepower]: https://github.com/kaiT2en/KaiT2en-Fedora/tree/main/apps/t2-zone-power
[adaptation]: https://github.com/iurjscsi1101500/t2-zone-power
