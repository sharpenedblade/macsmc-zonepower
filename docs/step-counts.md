# Step counts

A power limit step is a position in a ladder, not a watt or a clock. The SMC has to know how
long each ladder is before it can use one, and it does not know that on its own: the
operating system tells it. These keys hold the counts. The macsmc stack hands you the keys and
writes none of them; the KaiT2en stack is the one that has a service doing it.

| Key          | Ladder                                                    | Type    | On macOS       |
| ------------ | --------------------------------------------------------- | ------- | -------------- |
| `HI0N`       | iGPU                                                       | `ui8`   | written by AGPM |
| `HE0N`       | first discrete GPU, or the iGPU where `HE2N` exists        | `ui8`   | written by AGPM |
| `HE2N`       | discrete GPU, on the machines that have this key           | `ui8`   | written by AGPM |
| `MSTS`       | CPU turbo P-states                                         | `ui8`   | X86PlatformPlugin |
| `MSPC`       | CPU non turbo P-states                                     | `ui8`   | X86PlatformPlugin |
| `QENA`       | not a count: turns SMC throttling on and off               | `flag`  | written by macOS |

The iGPU and the dGPU each get one key, and which one depends on `HE2N`:

| Machine    | iGPU count | dGPU count |
| ---------- | ---------- | ---------- |
| no `HE2N`  | `HI0N`     | `HE0N`     |
| has `HE2N` | `HE0N`     | `HE2N`     |

This is how the KaiT2en `t2smc` driver picks the key. `HE2N` is only in the MacPro7,1 dump
so far, and that machine has no `HI0N`, so the second row is one nobody has measured.

## What macOS leaves behind

Values the SMC holds with macOS running, from the VirtualSMC dumps, decimal:

| Machine        | iGPU | dGPU | `MSTS` | `MSPC` |
| -------------- | ---- | ---- | ------ | ------ |
| MacBook10,1    | 13   | -    | -      | -      |
| MacBookAir7,2  | 15   | -    | -      | -      |
| MacBookAir8,1  | 16   | -    | 20     | 9      |
| MacBookPro11,4 | 21   | -    | -      | -      |
| MacBookPro12,1 | 12   | -    | -      | -      |
| MacBookPro13,3 | 15   | 16   | -      | -      |
| MacBookPro14,1 | 15   | -    | -      | -      |
| MacBookPro14,2 | 16   | -    | -      | -      |
| MacBookPro15,1 | 16   | 16   | 19     | 13     |
| MacBookPro15,2 | 16   | -    | 15     | 14     |
| MacBookPro16,1 | 17   | 16   | 19     | 17     |
| MacBookPro16,3 | 16   | -    | 25     | 5      |
| Macmini7,1     | 17   | -    | -      | -      |
| Macmini8,1     | 18   | -    | 14     | 25     |
| MacPro6,1      | 16   | 16   | -      | -      |
| MacPro7,1      | -    | 16, 20  | 5      | 24     |
| iMac18,3       | 20   | 16   | -      | -      |
| iMac19,1       | 20   | 16   | 11     | 18     |
| iMacPro1,1     | 20   | 16   | 10     | 21     |

What to take from it:

- `HE0N` is 16 on every machine that has it. KaiT2en found that macOS drives every Mac dGPU
  through 16 PowerTune states, index 0 to 15, and the key agrees on every machine: 16 is the
  number to use for an AMD dGPU. The one machine with `HE2N` holds 20 there, which nobody has
  explained.
- The iGPU count moves with the machine, 12 to 21, and it is not simply the number of
  50 MHz states of the GPU: on a MacBookPro16,1 `HI0N` is 17 and the deepest iGPU limit in
  ioreg (`IGPUPLimitDict` pMin) is also 17, while on an iMac19,1 `HI0N` is 20 and pMin is 16.
  The two numbers are not the same thing and nobody has derived the first one.
- A key does not prove the hardware. iMacPro1,1 has no iGPU and still has `HI0N`. Ask the SMC
  which keys exist instead of guessing from the model.

## What to write

- **dGPU**: 16. The ladder is the PowerTune table of the driver, which KaiT2en found macOS
  fills with 16 entries from the TDP down to a floor. On Navi 1x that floor is 40 % of the TDP,
  on Polaris it is 0.
- **iGPU**: the number of states i915 has between the top and the bottom of its ladder. i915
  publishes both ends, and macOS counts in 50 MHz:

  ```
  steps = (RP0 - RPn) / 50 + 1
  ```

  A mobile Gen9 part with RP0 1200 and RPn 350 gives 18, where a MacBookPro16,1 holds 17.
  Close enough to work with, and the off by one is not explained. Nobody has read RP0 and RPn
  on a T2 machine yet, which is the missing piece.
- **CPU**: not known. See the next section.

Before writing anything, read the key. The SMC holds a per machine value of its own, the
values in the table above all look right for their machine, and keeping one is free. Write
only when the value you read is missing or plainly wrong.

Never write 0. `t2smc` never sends one: it reads its own 0 as "user space has not set
anything, leave what the SMC has", and its files answer `EINVAL` for 0 and for anything above
127. That is a good range to hold yourself to, 1 to 127.

## How to write it

The counts are plain SMC keys, so on the macsmc stack they go through `/dev/smc`, which is
mode `0600`. See [macsmc](macsmc.md).

- `IOCTL_SMC_GET_KEY_INFO` first. It fails if the machine does not have the key, and it gives
  the size and the flags: bit 7 readable, bit 6 writable.
- `IOCTL_SMC_WRITE` with `struct ioctl_smc_rw_payload`: the key as the four characters packed
  into a `u32` in reading order, so `HI0N` is `0x4849304E`; the size in bytes; the value in
  `buf`, which is 32 bytes at most.
- The size has to be exactly what the SMC reported for the key, 1 for these, or the ioctl
  answers `EINVAL`.
- The ioctl takes six keys and nothing else: `HE0N`, `HE2N`, `HI0N`, `MSTS`, `MSPC`, `QENA`.
  Every other key is `EPERM`, and that includes the limit steps themselves, which the SMC does
  not let anyone write.
- `IOCTL_SMC_READ` reads the count back. One byte.

The KaiT2en `t2smc` driver wraps the same keys in three files in the directory of its hwmon
device, which is named `t2smc`: `smc_igpu_steps`, `smc_dgpu_steps` and `smc_cpu_floor` (which
takes `<turbo> <floor>` and writes `MSTS` then `MSPC` in that order). If a machine has that
driver, use the files.

There is a fourth GPU key, `HI1N`, `ui8`, writable in the SMC and 0 on every laptop dump we
have. It has been described as "send p-state to iGPU". Nothing has confirmed that, and the
ioctl does not let you write it.

## When to write

- Once at start, before applying any step. The SMC has no ladder for a GPU until the count is
  in, and until then it balances the heatpipe through the CPU alone.
- Again after every resume. The SMC forgets the counts over sleep and nothing puts them back.
  macOS rewrites them on wake, and `t2smc` rewrites whatever user space had set from its
  resume path. A daemon that survives suspend has to write them again, and it has to
  remember what it wrote.
- Report a count only for a GPU you actually drive, which is what macOS does. On a hybrid
  machine the discrete GPU may be off or not bound at start, so look for it again every ten
  seconds or so rather than giving up.

## What this daemon writes

- It reads a key before it writes one. A key the machine does not have does not answer, and
  nothing is written there.
- It keeps a count the SMC already holds when it is between 1 and 127. Every machine in the
  table holds a value of its own that looks right for that machine, and replacing it with a
  computed one is usually a downgrade.
- Counts are clamped to 1 to 127 on the way out.
- The iGPU count is `(RP0 - RPn) / 50 + 1`. No dGPU count is written yet, because the daemon
  does not cap a discrete GPU, and a count for a GPU nothing limits is a lie the SMC believes.
- The CPU counts are the 100 MHz P-states of the ladder split at the base frequency. They come
  from the first cpufreq policy: the policies of a machine share its frequency limits, so the
  first one counts for all of them. A driver other than intel_pstate or amd_pstate publishes no
  base frequency, and the ladder is then taken to have no turbo range. The `CPUFloor` of the
  board plist is not applied: no derivation fits the dumps, and the machine's own value is the
  better guess, see [the reference implementation](reference-implementation.md) for the one that
  comes closest.
- The counts are worked out once, before the first cap, and they go in again every ten seconds
  without being measured again.

## What it changes

t2-cruise measured the read only version of this on a MacBookPro16,1: with the steps applied
but the counts never reported, the SMC still brings the machine under its budget, but it does
all of it through the CPU. KaiT2en report roughly 10 °C off the peak temperature once the
counts and the CPU floor are reported as well.

The counts also decide how deep a step can go. The SMC can ask for a step beyond the count you
gave it, and the example record in [macsmc](macsmc.md) does exactly that with iGPU step 22, so
clamp the step to your ladder before you turn it into a clock or a watt.

## The CPU counts

`MSTS` and `MSPC` describe the CPU ladder, and macOS writes `MSTS` before `MSPC`. The pair on
a MacBookPro16,1 is 19 and 17. The obvious reading is a count of 100 MHz P-state steps: 19 of
them between the base clock and the top of turbo, and 17 between the base clock and the floor
the board plist of that machine sets, counting both ends. That reading fits the second number
on an iMac19,1 (11, 18) but not its first, and the first number on a MacBookPro15,1 (19, 13)
but not its second. Nobody has a derivation that fits every machine.

You do not need these to apply the CPU steps: 100 MHz per step is enough for that. macOS
reports them, and a machine that reports them behaves differently, so they belong on the list
of things to write.

## Still open

- Whether the SMC ships its own counts or whether every value in the table was written by
  macOS. Read the keys on a machine that has never run macOS to find out.
- Where the iGPU count actually comes from, and what the SMC does with a count that does not
  match the GPU in front of it.
- `MSTS` and `MSPC`.
- `HE2N`. One machine, one dump, and no iGPU to argue about.

## Where this comes from

- The [KaiT2en `t2smc` driver][t2smc]: the `HE2N` rule, the sysfs files, the resume rewrite,
  and the 1 to 127 range.
- [t2-cruise][cruise], in particular `docs/FINDINGS.md`: the controller, the PowerTune ladder,
  and what the counts do to the machine.
- The [VirtualSMC SMC dumps][dumps]: every value in the table.
- Patch `3014` of [linux-t2-patches][patches]: the ioctl and its six key allowlist.

[dumps]: https://github.com/acidanthera/VirtualSMC/tree/master/Docs/SMCDumps
[t2smc]: https://github.com/kaiT2en/KaiT2en-Fedora/tree/main/modules/t2smc
[cruise]: https://github.com/flo-coast/t2-cruise
[patches]: https://github.com/t2linux/linux-t2-patches
