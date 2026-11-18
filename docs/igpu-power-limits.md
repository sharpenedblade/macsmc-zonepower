# The iGPU power limit

The SMC limits the integrated GPU the same way it limits the CPU. It publishes a step, and
the operating system turns that step into a clock. macOS asks its graphics driver for a
maximum P-state, Linux asks i915 for a maximum frequency. Unlike the dGPU, where a step means
watts, an iGPU step means a clock.

Everything below is about T2 machines with an Intel GPU. iMacPro1,1 and MacPro7,1 have no
iGPU, so there is nothing to cap there.

## Where the step comes from

| Place                                  | What                                                     |
| -------------------------------------- | -------------------------------------------------------- |
| `plimit`, the byte at `+0x11`          | the iGPU step, bits 0 to 6, bit 7 is the hard limit flag |
| `MSTg`                                 | the key Apple's own tables call the GPU limit, `ui8`, one byte |
| `MSLG`                                 | the mirror T2 machines do have, `ui8`, read only         |
| hwmon label `GPU Plimit (MSTg)`        | the same step times 1e6, but only where the key exists    |

Two traps here. `MSTg` and `MSTm` are in Apple's key tables but in no SMC dump we have, and
`MSTc`, `MSTi` and `MSTf` only appear on the pre-T2 machines. The hwmon driver drops keys the
machine does not have, so on a T2 Mac the I/O window is where the iGPU step is, and the steps
themselves are read only everywhere. `MSLG` is in every T2 dump we have, and what it follows
on a machine with two GPUs is not pinned down: t2-cruise applies it to the Radeon of a
MacBookPro16,1.

Step 0 is no limit. A higher step is a slower GPU. The step changes on its own, so a daemon
has to keep reading it, see [macsmc](macsmc.md).

On a hybrid machine the zone keys follow the discrete GPU (`PZ2x`), and the iGPU shows up in
the `PC*G` readings instead, `PC0G` is labelled CPU Integrated GPU. There is no iGPU zone.

## The ladder is reported, not fixed

The SMC does not know how many steps a machine has. The operating system tells it. The whole
story, including what real machines hold, is in [step counts](step-counts.md); the short
version is here.

| Key   | Machine                                       | Value                 |
| ----- | --------------------------------------------- | --------------------- |
| `HI0N` | has no `HE2N`, most or all T2 machines        | number of iGPU states |
| `HE0N` | has no `HE2N`                                 | number of dGPU states |
| `HE0N` | has `HE2N`                                    | number of iGPU states |
| `HE2N` | has it                                        | number of dGPU states |

The SMC holds 17 and 16 on a MacBookPro16,1, and the PLimit handlers in ioreg of that machine
read `IGPUPLimitDict` pMin 17 and `EGPUPLimitDict` pMin 15. They are related but they are not
the same number: an iMac19,1 has 20 in `HI0N` and pMin 16. Copying pMin into the key is not a
way to get the count.

The counts are lost over sleep. Whatever writes them has to write them again after resume.

## What a step is worth

i915 publishes the P-state range of the GPU and the cap it is currently holding:

```
/sys/class/drm/card0/gt_RP0_freq_mhz    fastest state, read only
/sys/class/drm/card0/gt_RP1_freq_mhz    read only
/sys/class/drm/card0/gt_RPn_freq_mhz    slowest state, read only
/sys/class/drm/card0/gt_max_freq_mhz    the cap, writable
/sys/class/drm/card0/gt_boost_freq_mhz  clock for a client that waits, writable
/sys/class/drm/card0/gt_min_freq_mhz    the floor, writable
```

Find the card by the file, not by the card number: only the Intel driver creates `gt_*`, and
on a hybrid machine the iGPU is not guaranteed to be card0. `/sys/class/drm/cardN/device/vendor`
is `0x8086` for an Intel card.

```sh
for f in RP0 RP1 RPn max boost min; do
        printf '%-6s %s\n' $f $(cat /sys/class/drm/card0/gt_${f}_freq_mhz)
done
```

macOS numbers the iGPU states in 50 MHz from RP0 down to RPn, so

```
cap = max(RP0 - step * 50, RPn)
```

Nobody has written down what a T2 iGPU reports for RP0 and RPn yet. If a MacBookPro16,1 runs
from 1200 down to 350, which is 18 states, the ladder is:

| Step | Cap          |
| ---- | ------------ |
| 0    | 1200, no cap |
| 1    | 1150         |
| 7    | 850          |
| 15   | 450          |
| 17   | 350          |

`(1200 - 350) / 50 + 1` is also the way to get the count to report, which gives 18 where the
SMC of that machine holds 17. It is close and it is the best guess so far.

`powermetrics` on a Mac prints the same steps: `CPU Plimit`, `GPU Plimit (Int)`, and on a
machine with a second GPU `GPU2 Plimit (Ext1)`. It has shown 15 for the iGPU of a
MacBookPro16,1 under load, which is a 450 MHz cap on the ladder above.

Clamp the step to the ladder. The example line in [macsmc](macsmc.md) asks for iGPU step 22,
which is above this ladder, and the SMC does ask for more than a ladder has.

Gen9 and Gen11 do not use GuC by default, so i915 takes the host side RPS path and turns a
MHz value into a hardware ratio in units of 50/3 MHz. The hardware ladder is 16.67 MHz wide,
the 50 MHz is only how macOS and i915 count the states. Anything in that range is accepted.

## Writing the cap

- i915 answers `EINVAL` for a cap below RPn, above the top of its ladder, or below the
  current `gt_min_freq_mhz`. Clamp before writing.
- The cap is a ceiling, not a target. The GPU stays free to sit under it.
- A write while the GPU is parked is stored and applied when it wakes. i915 only touches the
  registers when RPS is running, so the write does not wake the GPU, and a daemon does not
  have to worry about hybrid machines with the panel on the dGPU.
- `gt_boost_freq_mhz` is a second door to the top. When a client waits on a request, i915
  throws away the soft limit for that decision and jumps to the boost clock, which defaults
  to the top of the ladder. Capping only `gt_max_freq_mhz` leaves a hole. Cap both to the
  same value.
- The cap is a runtime value in driver memory. i915 sets it back to the top of the ladder when
  it works the GPU out again at load, and it reprograms the hardware after a GT reset, so keep
  re-applying it rather than trusting the last write.
- There are two names for the same files: `gt_*` next to `card0/device`, and `rps_*` under
  `card0/gt/gt0`. Both are there on any Gen6 or newer GPU. The `gt_*` names also show up under
  `gt/gt0`, and the two rate files `rps_up_threshold_pct` and `rps_down_threshold_pct` appear
  there only when GuC is not doing the clocking, which is the case here.

## Telling the SMC the count

`HI0N` is one of the six keys `/dev/smc` lets you write. Send it through `IOCTL_SMC_WRITE`
with the size the SMC reports for the key, which is 1 for a `ui8`, and the count as the
payload. Ask `IOCTL_SMC_GET_KEY_INFO` first: it says whether the key exists on this machine at
all, and `IOCTL_SMC_READ` reads the count back, which is how you see what the SMC had before.

Report the count before the first cap. Without it the SMC has no ladder for the iGPU and
balances the heatpipe on the CPU alone, which is what Linux does today. What to put in the
key, what range is sane and when to write it again are in [step counts](step-counts.md).

## Still unknown

- Nobody has said what the hard limit flag means for the iGPU byte.
- Nobody has written down which RP0 and RPn the T2 machines report. Worth collecting: a
  MacBookAir8,2 (Amber Lake), a MacBookPro15,2 (Iris Plus 655), a MacBookPro15,4 or 16,3
  (Coffee Lake plus AMD) and an iMac20,x (Comet Lake) are the interesting cases. With those
  numbers the iGPU count stops being a guess.
- Which GPU `MSLG` follows on a hybrid machine, and whether there is a mirror for the iGPU on
  a T2 Mac at all.
