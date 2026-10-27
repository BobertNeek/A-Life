# Finite continuous-world care sessions

`scripts/run_continuous_care.py` is an opt-in host for **one ordinary fresh
Feeding world/body/runtime**, normally 512 waking training decisions. It reuses
the resident producer, recurrent PPO and atomic current-checkpoint publisher.
It does not concatenate ended episodes, schedule an overnight run, replace
targeted lessons, or claim a measured speedup. The default invocation only prints
a plan and does not inspect a checkpoint, run a native command or reserve seeds.

The accepted founder's learned actor, value state, compiler state and both Adam
histories initialize the session. This is a **fresh cohort**, consistent with the
existing care curriculum. Recurrent memory, ordinary lifetime plasticity,
biology, physics and food ecology then continue within that one world. World
setup and the optional need-aging preparation happen once. Food regrowth uses
the existing world ecology; this host adds no food, hunger, reward or action
override. `newborn_v1` and `ordinary_healthy_aging_v1` remain distinct conditions.

The genetic actor is frozen during collection. PPO runs after collection, using
the ordinary real bootstrap on a living horizon and terminal handling on death.
There is no supported installation of the newly trained global actor into the
same living individual. Final private capture remains **pre-PPO**, paired with
post-PPO global heads under the existing fresh-cohort handoff contract. The next
session therefore starts a new ordinary private life; this is not a solution to
continuous same-individual online policy installation.

The native bridge permits explicit Feeding horizons 1..36000, the existing
single-cycle bound. Other resident care lessons keep their 1..128 limits. Longer
requests do not change logical time: the authoritative loop is still 20 world
ticks per simulated second and runs without deliberate real-time pacing. Sleep
and no-capture ticks still execute ordinary causality; captured segments and
sleep-gap physiology accounting remain in use. A row budget is not elapsed
simulation time. The existing absolute world guard, `8 * decisions + 4096`,
includes setup; the raw replay cap is 4 GiB. A request can hit world, RAM, VRAM or
temporary-storage limits before reaching its decision horizon. In particular,
allowing 512 decisions does not prove the default 2 GiB producer quota suffices
for a particular founder. Failure must reconcile the consumed seed and retained
baseline, rather than bypass the quota.

## Admission and lifecycle

Use a verified exact-source native build containing the finite-row bridge,
causal metrics, bound cleanup and completed shared active-write quota repair.
The launcher rejects old binaries lacking the contract flag
`temporary_budget_enforced_during_writes=true`. Inspect the build receipt and
binary/inspector SHA-256 before supplying their explicit hashes. A caller's hash
alone does not authenticate a binary's source revision. The source checkout must
be clean and the current checkpoint already fully admitted. The initial GPU
telemetry implementation uses `nvidia-smi`; other adapters need a supported
telemetry implementation before this host can admit them. Host adapter telemetry
alone does not establish the native runtime's selected adapter/backend.

Create a dedicated empty owner directory **outside** the source checkout and
checkpoint store. All cooperating launchers must use that same owner directory.
Its stable OS lease prevents duplicate host owners, while the checkpoint store's
existing Native/Python lease prevents simultaneous state writers. Check actual
process ownership and resources before dispatch; no application is closed.
Outside the automatic safe window, retain the default 300-second input-idle
guard. `--idle-seconds 0` is only appropriate under independently established
safe-window/user authority. Telemetry calls are read-only and issue no input.

Example plan; replace every placeholder using the current handoff:

```powershell
$env:ALIFE_SLM_PRIOR = 'on'
$env:ALIFE_SLM_PRIOR_BACKEND = 'deterministic'
python -B scripts/run_continuous_care.py `
  --store C:\A-Life\current --lane C:\A-Life\continuous-owner `
  --binary C:\A-Life\qualified\train_n2048_care.exe `
  --checkpoint-tool C:\A-Life\qualified\inspect_archived_success.exe `
  --binary-sha256 EXACT_NATIVE_SHA256 --checkpoint-tool-sha256 EXACT_INSPECTOR_SHA256 `
  --seed FIRST_UNUSED --last-seed OWNED_ALLOCATION_END `
  --decisions 512 --hard-end FRESH_UTC_END --final-reserve-seconds 180
```

Add `--run` only after admission. An existing pending allocation must be resumed
exactly, including its unused tail; the host does not silently allocate over an
exclusion or drop unfinished curriculum slots. One world consumes one seed. An
outer curriculum controller remains responsible for its full remaining lesson
schedule; replacing one targeted block with a continuous session must be explicit.

Lease order is: Python Store admission/publication recovery, **release**, one
Native producer with its WAL, confirm owned child stopped, Native recovery with
its own lease, **release**, Python atomic publication, **release**, Native bound
cleanup. Python never holds the store lease while running producer/recover/finish.
The canonical current remains unchanged until full typed recovery-candidate
admission. Native finish checks exact recovery-intent/current/candidate/WAL
binding and owned paths; it does not relabel a Native run as a legacy host run.

The host pins source, helpers, executables and explicit deterministic-prior
environment. The Native seed WAL is authoritative. `--reconcile --run` with the
same store/lane/binary/inspector resumes **recovery only**, never another world.
An unknown process-launch outcome fails closed and retains the owner record.
Do not delete that record or launch another worker to bypass it.

The host stops its own producer at `hard-end - final-reserve-seconds` and reserves
at least 180 seconds for recovery, admission/publication and cleanup. Measure the
actual final-save cost before reducing or relying on that reserve. Native STOP
and collection cutoff are checked between rows, so the host deadline can abort
a long active world. One row saves only at its end: a crash, guard stop or forced
deadline can lose **all** new updates in the session, with its seed still
consumed. No periodic mid-world full snapshot is claimed. The previously validated
accepted baseline remains recoverable. This implements the accepted recent-work
risk rather than demanding full persistence on every waking decision.

Native temporary data stays inside its declared aggregate quota. Host control
metadata and two logs are separate, bounded sidecars (each log at most 8 MiB at
guard observations, with possible between-observation overshoot). After admitted
publication, Native cleanup removes the run, and the host clears those sidecars.
One bounded `last-continuous-session.json` remains with compact outcomes and
identities; no episode archive or additional learned checkpoint is retained.

## Measurement and first comparison

The retained receipt separates provisional trajectory outcomes, admitted final
policy/Adam ages, retained decision rows, whole-host wall time and exact nourishing
events from generic `Consumed`. Missing old instrumentation is unknown. Bound
target nourishment can include regrowth of the same target; it is not proof of a
complete varied self-care loop. Prior counters/provider IDs must show actual
delivery; setting an environment variable is not delivery evidence.

`causal_collection` measures committed control ticks **after setup/first prime**
through collection end, including any real bootstrap tick. Its wall interval
includes intervening trainer/replay/host work, but excludes setup, the first prime,
final save and PPO. Its capture counters cover foundation training-state
snapshot/readback traffic, not all GPU traffic or total GPU time. Do not substitute
the older `collection_seconds` denominator. Require the actual object before
reporting simulated seconds/wall second. Whole-session retained rows/hour also
includes restore, setup, optimizer, save, native recovery and publication.
Its exact endpoint follows Native cleanup, final admission and event reading;
the final compact-receipt write and host-sidecar housekeeping are excluded.
If a crashed host is reconciled later, that downtime is included in this interval
and must be disclosed when comparing runs.

The first hardware assay is pending resource admission. On disposable clones of
one admitted baseline, use the same source/backend, explicit prior, need condition
and total requested decision exposure: one 512-decision world versus eight
64-decision fresh worlds. Reserve disjoint fresh seed sets and preserve both
baselines. Record actual trained/bootstrapped rows, committed world ticks, control
and whole-host wall time, PPO epochs and actor/value optimizer-step deltas,
death/truncation, exact nourishing events, physiology and actual prior delivery.
Equal decision exposure does **not** imply equal PPO updates (one long collection
may make two updates while eight short collections make sixteen). Different
world age, food encounters, setup frequency and update cadence are changed
workload/treatment, not a controlled claim of better learning. Follow with matched
held/reachable retention and repeated need-driven care before claiming benefit.
This source change has no measured simulation multiplier or hardware qualification.
