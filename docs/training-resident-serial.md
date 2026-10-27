# Experimental finite resident serial care

The opt-in [finite continuous-world host](training-continuous-world.md) runs one
longer Feeding world through the same producer, atomic publisher and bound Native
cleanup. It does not install a new PPO policy into a living world or concatenate
separate private lives. Its initial 512-decision horizon remains hardware-unqualified.

This opt-in producer does not replace the qualified training launcher. No GPU
qualification or throughput result is implied by this source change.

`train_n2048_care --resident-contract` reports capability. Use
`--resident-serial STORE REQUEST.json` only after exclusive training/resource
admission. STORE must contain the admitted `current.alife-checkpoint`, its stable
writer lock, and no pending publication. An existing `.run` is refused.

Example request (replace the seeds and cutoff after checking current metadata):

```json
{"schema":1,"seed_range":[100,103],"rows":[{"lesson":"feeding","decisions":16},{"lesson":"relocated_food","decisions":64}],"checkpoint_every_episodes":2,"sampling_temperature":1.0,"collection_cutoff_unix_seconds":1800000000,"max_temporary_bytes":2147483648}
```

The request must resume the exact pending allocation or reserve a wholly fresh
range. The entire allocation remains excluded; a checksummed, flushed small WAL
consumes each world before dispatch. Historical freshness is never upgraded.
The parent owner must enforce the hard deadline, activity/thermal guards and
process cleanup. `.run/STOP` is checked between episodes. This CLI never closes
other applications or sends desktop input.

The same native trainer retains actor weights, both Adam moments, coordinate
ages, stage mask and global age across cohorts. Critic/value checkpoint state is
carried in RAM through the existing critic objective; this is not a claim that
all critic operations stay on GPU. Every lesson still initializes its ordinary
fresh world, sensory setup, recurrent/eligibility/lifetime buffers. Private
individual state is not transferred across policies. The existing first exact
capture, replay/log-probability identity, bootstrap handling, PPO and biological
consequences remain in use. Asset export/readback and next-cohort admission also
remain; the speed benefit must be measured.

Updates are **provisional in RAM** until a configured full snapshot commits.
Cadence is explicit, not forced per episode. The planned final row always saves.
A boundary stop, crash, power loss or external hard stop can lose all updates
since the last snapshot (possibly the entire run). The baseline current remains
byte-identical throughout. Seeds already dispatched remain consumed even when
their provisional updates are lost.

Before a scheduled saving episode begins, final private capture is enabled at
the unchanged collection-complete **pre-PPO** capture/publish/rebind point. After
PPO, full actor/value heads are paired with that private component, typed native
frontier admission runs, and one self-contained checkpoint is encoded, flushed,
fully readmitted, then committed by a checksummed manifest. At most two committed
temporary snapshots and the active episode are retained within the declared byte
budget. No late private capture after PPO is substituted. No complete private
packet is required for intermediate non-saving episodes.

After the producer is definitely stopped, `--recover-resident STORE` acquires
the same writer lease. It validates baseline/request/WAL/snapshot binding and
fully admits the latest committed snapshot (or the original baseline), then
applies the latest consumed seed cursor. It writes and fully admits
`.run/recovered-current.alife-checkpoint` and an idempotent recovery intent.
It never overwrites canonical current, starts a world, or claims lost RAM updates
were restored. The existing owner must publish that exact candidate through its
qualified atomic-current publisher and verify it before removing `.run`.
Legacy host runners refuse the new native run kind; they must not clean it or
silently fall back to the unchanged baseline. Cleanup/next-launch integration
remains an owner responsibility until separately qualified.

Device-free tests cover loss-window seed exclusion, corrupt/partial metadata,
exclusive writer admission, and complete baseline recovery without historical
source dependencies. An ignored hardware test covers two native resident
episodes, final private-state snapshot and actor/value integer-age continuation;
ordinary CI compiles it but does not execute it.
