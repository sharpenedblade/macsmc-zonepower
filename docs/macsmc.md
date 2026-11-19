# Zone power on Intel Macs

macsmc is the kernel stack that talks to the SMC on Intel Macs. It is where all zone power
data comes from. There are three interfaces, and they split cleanly by direction:

| Interface     | Direction | What you get or send                                                  |
| ------------- | --------- | --------------------------------------------------------------------- |
| `plimit` file | read      | the power limit step of every zone, as one packed line                |
| hwmon device  | read      | power, temperature, current, voltage, and the limits where they are keys |
| `/dev/smc`    | both      | one key and up to 32 raw bytes at a time, writes only for six keys     |

A T2 Mac expects the operating system to act on those requests and lower the CPU and the GPUs
for it. When nothing does, the SMC keeps escalating until it asserts PROCHOT, which drops the
CPU to its lowest clock under a modest combined CPU and GPU load while the temperatures are
still normal.

`macsmc-power` is the battery and AC adapter driver. Despite the name it has nothing to do
with power zones.

## Keys

SMC keys are four character codes. Zone power keys are `PZ<zone><role>`:

| Zone                                       | What it covers                            |
| ------------------------------------------ | ----------------------------------------- |
| `PZ0x`                                     | heatpipe, the whole machine budget        |
| `PZ1x`                                     | CPU                                       |
| `PZ2x`                                     | discrete GPU                              |
| `PZ3x`                                     | battery auxiliary                         |
| `PZ4x`                                     | battery                                   |
| `PZ5x`                                     | "abstract" zone, not well understood      |
| `PZAP` / `PZBL` / `PZHD` / `PZDM` / `PZOD` | wireless, backlight, storage, memory, ODD |

Those names come from Apple's own key tables and they are not steady: `PZHD` is labelled HDD
power in some of them and DIMM power in others.

| Role | Meaning                                             |
| ---- | --------------------------------------------------- |
| `G`  | measured power                                      |
| `F`  | filtered power                                      |
| `E`  | target power                                        |
| `T`  | throttle level, a `flt` from 0 to 100, not watts and not a step |

Which zones a machine has is a mask in `ZPEN` (`hex_`, one byte): `0x3f` on a MacBookPro16,1,
which is zones 0 to 5, and `0x1f` on a MacBookPro15,1.

The steps themselves are keys on the older machines: `MSTc` CPU, `MSTg` iGPU, `MSTi` dGPU,
`MSTm` memory and `MSTf` forced idle, all `ui8`, and they are in the macsmc-hwmon label table.
The dumps say less than the table does: `MSTc`, `MSTi` and `MSTf` are there on the pre-T2
machines and on no T2 machine, and `MSTg` and `MSTm` are in no dump at all. On a T2 Mac the
record is only in the I/O window, and the mirrors are `MSLC` CPU and `MSLG` GPU (`ui8`, read
only), with `MSAc`, `MSAg` and `MSAi` (`fp88`) which look like the smoothed versions.

Three keys describe the limits instead of measuring them. `RGEN` and `DPLM` give the data
version of the limit record, `HI0N`, `HE0N` and `HE2N` hold how many steps the GPUs have and
are written by the operating system, see [step counts](step-counts.md).

Which zones and keys exist depends on the model. Only the ones the machine actually has are
exposed, so nothing here can be assumed present. `MSTi` is missing on the pre-T2 machines that
have no discrete GPU, and no T2 machine has the `MSTx` keys at all. The other way round also
happens: a key can sit on a machine with no hardware behind it.

## Reading the limits: `plimit`

A read-only sysfs file on the SMC device, not in the hwmon directory. The device is the
platform device the ACPI core makes for the `APP0001` node, so the path does not move:

```sh
find /sys -name plimit
/sys/bus/platform/devices/APP0001:00/plimit
2e001627 0f
```

Two space-separated fields, no header, one line:

| Field  | Format                           | Contains                                   |
| ------ | -------------------------------- | ------------------------------------------ |
| first  | 8 hex digits, read little-endian | four bytes: CPU, iGPU, memory, forced idle |
| second | 2 hex digits                     | dGPU                                       |

Within each byte, bit 7 says the limit is a hard limit and the other 7 bits are the step. So
`0x8f` is step 15 with the hard limit flag, and `0x0f` is step 15 without it.

Reading `2e001627 0f` gives:

| Zone        | Byte   | Step | Hard limit |
| ----------- | ------ | ---- | ---------- |
| CPU         | `0x27` | 39   | no         |
| iGPU        | `0x16` | 22   | no         |
| memory      | `0x00` | 0    | no         |
| forced idle | `0x2e` | 46   | no         |
| dGPU        | `0x0f` | 15   | no         |

The bytes sit at fixed offsets in the I/O window the SMC uses for its events: `+0x10` the
word, `+0x14` the byte. The driver reads them straight out of the window, which it always
requests from ACPI even when it also has the MMIO window for key commands, so the file works
whichever way key reads are done.

Two things to watch:

- The window has no version field. The version is in the key table: `RGEN` at 3 means the
  layout above, and where `RGEN` is missing the size of `DPLM` decides, 5, 4 or 3 bytes for
  version 3, 2 and 1, and no `DPLM` at all means version 1. `DPLM` is a `{lim` key that the SMC
  will not read back, so the size it has in the key table is all you get out of it. Every
  machine in the dumps has `RGEN` 3, or no `RGEN` and a 5 byte `DPLM`, so all of them are
  version 3. Only the version 3 layout is understood.
- A record with a step in it is a request, not a state. The SMC can ask for a step deeper
  than the ladder the operating system has.

## What a step is worth

A step is an index into a ladder the operating system owns, so it means something different
for every zone:

| Zone        | One step is                              | Linux control                                |
| ----------- | ---------------------------------------- | -------------------------------------------- |
| CPU         | one P-state step, 100 MHz                | `scaling_max_freq` of each cpufreq policy    |
| iGPU        | one P-state step, 50 MHz                 | `gt_max_freq_mhz` and `gt_boost_freq_mhz`     |
| dGPU        | one entry of the driver power table      | `power1_cap` of the amdgpu hwmon device       |
| forced idle | a share of forced idle time              | `cur_state` of the intel_powerclamp cooling device |
| memory      | not known                                | nothing yet                                  |

The ladders have to be described back to the SMC, see [step counts](step-counts.md). How to
turn the iGPU step into a clock is in [iGPU power limits](igpu-power-limits.md).

Forced idle is the one limit that does not move a clock. It says what share of CPU time has to
be idle: the step leaves the CPU `100 - step` percent of the time to run and holds the cores in
a C-state for the rest, so it caps average CPU power without touching frequency or voltage.
macOS does this in XCPM and never goes below 30 % run time, which is a floor of 70 % idle.

There is one idle step for the machine and no step per core. `intel_powerclamp` takes the same
percentage in its single `cur_state`, and one timer wakes an injection thread on every target
core, so they all get the same duty cycle. Which cores are targets is a mask, the `cpumask`
module parameter, and by default it is every logical CPU, both siblings of a hyperthread
included. The mask only changes while injection is off, so it is not something a daemon can
adjust as it goes.

The depth is capped by the mask as much as by the driver. A `cur_state` above `max_idle - 1` is
quietly clamped down, `max_idle` is 50 by default, and it accepts up to 100 only where the mask
leaves some cores out, 75 where it covers every present CPU. `max_state` answers 100 and does
not tell you the ceiling, `max_idle` does. The driver measures what it achieved through the
package C-state counters, and only when the mask is every present CPU of a single package and
die, so on a partial mask it injects without feedback.

## Reading sensors and limits: hwmon

```
/sys/class/hwmon/hwmonN/name          ->  macsmc_hwmon
/sys/class/hwmon/hwmonN/power1_label  ->  CPU Target (PZ1E)
/sys/class/hwmon/hwmonN/power1_input  ->  45000000
```

Find the device by reading `name`, which says `macsmc_hwmon`. Never pick a channel by
number: the numbers follow the driver's key table filtered by the machine, so they move
between models and even between boots. Find the channel by its label instead, and read the
key out of the parentheses.

Values are scaled up, so a channel is only useful after dividing:

| Channel                        | Type  | As read     | Divide by | Result                |
| ------------------------------ | ----- | ----------- | --------- | --------------------- |
| power (`PZxG`, `PZxF`, `PZxE`) | `flt` | microW      | 1e6       | W                     |
| throttle (`PZxT`)              | `flt` | level x 1e6 | 1e6       | a level from 0 to 100 |
| limits (`MSTx`)                | `ui8` | step x 1e6  | 1e6       | step                  |
| temperature, current, voltage  | `flt`, fixed point | milli | 1e3  | degrees, amps, volts |

To list what a live machine has:

```sh
h=$(find /sys/class/hwmon -name name -exec grep -l macsmc_hwmon {} \; | head -1 | xargs dirname)
paste <(cat $h/power*_label) <(cat $h/power*_input)
```

Where the `MSTx` keys exist they also appear here as power channels, which avoids the I/O
window, but a hwmon limit carries no hard limit flag, so if you need the flag, use `plimit`.
On a T2 Mac they are not there at all and `plimit` is the only way to the steps.

## Writing: `/dev/smc`

All writes go through this device. hwmon is read-only apart from fan targets, which need an
unsafe module parameter. The device is mode `0600`, so this is root only.

You pass a key, a size and a buffer of up to 32 bytes, and get success or an error back. The
key is the four characters packed into a `u32` in reading order, so `HI0N` is `0x4849304E`. The
size has to be exactly the size the SMC reports for that key, or the ioctl answers `EINVAL`.

The ioctl accepts six keys and nothing else, because keys owned by other kernel drivers must
not have two writers, and everything else is `EPERM`:

| Key            | Effect                                                  |
| -------------- | ------------------------------------------------------- |
| `HE0N`         | dGPU step count, or the iGPU count where `HE2N` exists  |
| `HE2N`         | dGPU step count, on the machines that have it            |
| `HI0N`         | iGPU step count, on the machines without `HE2N`          |
| `MSTS`, `MSPC` | CPU P-state range, that is, the CPU floor                |
| `QENA`         | SMC throttling on and off, `flag`, and 1 in every dump   |

The limit steps are not in that list and the SMC does not let anyone write them either.

`IOCTL_SMC_GET_KEY_INFO` says whether a key exists on the machine at all, and gives its size,
its type and its flags, where bit 7 is readable and bit 6 writable. `IOCTL_SMC_READ` reads a
key back. `IOCTL_SMC_GET_KEY_INDEX` walks the full key table, which is how you find out what a
machine has instead of trusting a static list.

The counts and their ranges are in [step counts](step-counts.md). The SMC forgets them over
sleep and nothing puts them back, so whoever writes them has to write them again after
resume.

The KaiT2en `t2smc` driver is a different stack that does the same writes behind three sysfs
files instead of an ioctl: `smc_igpu_steps`, `smc_dgpu_steps` and `smc_cpu_floor`.

Raw bytes come with a type code and you decode them yourself. Plain integers and fixed point
come back big-endian, and the fixed point suffix says how many fraction bits there are in hex,
so `fp2e` is 2.14 and you divide by 2^14. The 32-bit floats are the odd ones out and come back
little-endian: the bytes `63596740` of a `flt` key are `0x40675963`, which is 7.23.

## Nothing tells you when limits change

The ACPI backend does request an interrupt and does see event `0x80` when the limits change,
but that ends in a notifier chain inside the kernel and macsmc-hwmon does not listen to it.
There is no event and no poll() support in user space, so a daemon has to re-read on a timer.
Around 250 ms works. The KaiT2en `t2smc` driver is the one that wired the event through: its
`smc_plimit_raw` file wakes readers, which is what a poll() on `plimit` would look like.

## Where to look

The macsmc patches live in [t2linux/linux-t2-patches][patches]. `plimit` and the ACPI backend
are in patch `3002`, the extra key types in `3012`, the key tables and labels in `3013`, the
ioctl in `3014`, the core in `3001`. The mainline `drivers/hwmon/macsmc-hwmon.c` covers the
label and scaling behaviour.

For everything that is not in the kernel: the [VirtualSMC SMC dumps][dumps] list the keys and
values of real machines, [t2-cruise][cruise] is the reverse engineering of the controller, and
the [KaiT2en `t2smc` driver][t2smc] is the stack that already writes the step counts.

[patches]: https://github.com/t2linux/linux-t2-patches
[dumps]: https://github.com/acidanthera/VirtualSMC/tree/master/Docs/SMCDumps
[cruise]: https://github.com/flo-coast/t2-cruise
[t2smc]: https://github.com/kaiT2en/KaiT2en-Fedora/tree/main/modules/t2smc
