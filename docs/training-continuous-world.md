# Resident continuous-life training

`scripts/run_continuous_care.py` defaults to restoring **the living world and
individual in Current**, collecting 512 trained decisions per PPO update, and
installing the updated genetic policy into that same runtime. Its default total
budget is one million trained decisions, bounded by the dated cutoff and
temporary-storage quota. It neither concatenates fresh episodes nor changes
simulation time. Without `--run`, it only prints a plan: no checkpoint inspection,
native command, seed reservation or world execution occurs.

Actor/value weights and full Adam history, the saved body/world/ecology,
recurrent memory and ordinary lifetime plasticity are retained. Restored-life
mode rejects food/need preconditioning; it does not reset hunger, move food or
prepare a lesson again. New policies use an explicit compiler overlay over
unchanged biological inputs. Mutable neural state and the runtime sampling
stream must survive policy installation. Legacy missing sampling provenance must
be reported explicitly; a declared new stream is not reconstructed history.

Each rollout uses one frozen policy version. PPO updates on scratch after that
rollout, then the checked new policy is installed before collection continues.
Snapshots must pair those installed heads and optimizer ages with the exact same
live private state. Natural death terminates that individual; this host does not
resurrect it or silently substitute a new body. Unfinished targeted care lessons
remain in the curriculum handoff and are not credited by continuous exposure.

`--fresh-world` explicitly selects the older diagnostic: one new Feeding world,
frozen actor throughout collection, final **pre-PPO** private capture and
post-PPO global heads. Only that mode accepts `--food-need-condition`, with
`newborn_v1` and `ordinary_healthy_aging_v1` kept separate. It is not a fallback
when a binary lacks the live-policy continuation contract.

The living collection limit is an artificial truncation, not death. The final
successor value is retained. The ordinary fresh-world cycle zeroes it on confirmed
death; the new same-individual MVP instead discards the pending terminal window
before PPO and retains the last valid snapshot. It does not yet learn a terminal
PPO target. Retaining such an update requires a typed terminal-head handoff,
separate from the last exact living-private snapshot; it cannot install learned
heads into a dead body or pair them with stale living private state. Obtaining a
living successor record currently executes one more **ordinary waking decision**, plus
any intervening normal non-capture ticks. Its cognition, recurrent/lifetime
learning, motor action and biology are real. It is excluded from PPO loss but
included in the subsequent final private-life capture. The later frozen value
pass reads recorded states on offline trainer scratch and does not advance the
authoritative world. Do not call the whole bootstrap acquisition an observational
peek or claim that its action had no physical effect.

The 512-row value reads and 256-row PPO spans are offline slices within an update;
they do not recreate the live body. Each replay window starts from its captured
recurrent/homeostatic/dendritic state and per-row acquired-weight offsets. Four
explicit `--fresh-world` 128-decision sessions instead create four fresh private
lives and, when all survive, capture 516 decisions rather than one fresh
512-decision session's 513. They
cannot establish chunk invariance. A future 1x512 versus 4x128 continuity assay
must keep one world, complete initial state/RNG and lifetime learning, disable
PPO updates in both arms, match actual ticks, and reuse a boundary successor
record or provide a genuinely observational acquisition seam. That resumable
same-world no-update chunk-equality assay remains distinct from comparing
different PPO update schedules. Hardware equivalence/speed needs actual evidence.

Live requests stream up to ten million total trained decisions in windows of
1..36000. The older fresh Feeding cycle keeps its 1..36000 horizon and other
fresh resident care lessons keep their 1..128 limits. Longer
requests do not change logical time: the authoritative loop is still 20 world
ticks per simulated second and runs without deliberate real-time pacing. Sleep
and no-capture ticks still execute ordinary causality; captured segments and
sleep-gap physiology accounting remain in use. A row budget is not elapsed
simulation time. The fresh-cycle absolute world guard, `8 * decisions + 4096`,
includes setup; the raw replay cap is 4 GiB. A request can hit world, RAM, VRAM or
temporary-storage limits before reaching its decision horizon. In particular,
allowing 512 decisions does not prove the default 2 GiB producer quota suffices
for a particular founder. Failure must reconcile the consumed seed and retained
baseline, rather than bypass the quota.

## Admission and lifecycle

Use a verified exact-source native build containing the finite-row bridge,
causal metrics, bound cleanup and completed shared active-write quota repair.
The launcher rejects old binaries lacking the contract flag
`temporary_budget_enforced_during_writes=true` or
`recoverable_predispatch_initialization=true`. Inspect the build receipt and
binary/inspector SHA-256 before supplying their explicit hashes. A caller's hash
alone does not authenticate a binary's source revision. The source checkout must
be clean and the current checkpoint already fully admitted. The initial GPU
telemetry implementation uses `nvidia-smi`; other adapters need a supported
telemetry implementation before this host can admit them. Host adapter telemetry
alone does not establish the native runtime's selected adapter/backend.

`train_n2048_care --build-info` reports build-time Cargo `PROFILE`, `OPT_LEVEL`
and target values, the executable's actual `debug_assertions` cfg and selected
package feature flags. It requires no GPU, checkpoint or training startup and
does not change `--resident-contract`. Cargo-reported values describe this
package; they do not certify every dependency's optimization, effective custom
compiler flags, source revision or hardware qualification. Cargo `PROFILE` is
the debug/release inheritance bucket, not the name of a custom profile. Runtime environment
variables cannot change the embedded values. This diagnostic does not change
launcher defaults or establish the profile of older binaries lacking the flag.

The default also requires `same_individual_ppo`, `restore_current_private_world`,
`full_sampling_state_checkpoint` and `in_world_cutoff`, plus sufficient advertised
total/update limits. Missing support is a refusal, not permission to run a fresh
world. `export_native_training_handoff.exe` supplies full Current admission and
allocation checks; `inspect_archived_success.exe` is a different tool.

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
  --checkpoint-tool C:\A-Life\qualified\export_native_training_handoff.exe `
  --binary-sha256 EXACT_NATIVE_SHA256 --checkpoint-tool-sha256 EXACT_INSPECTOR_SHA256 `
  --auto-seed --night-date FRESH_ALBERTA_DATE `
  --decisions 512 --total-decisions 1000000 --snapshot-every-updates 32 `
  --final-reserve-seconds 180
```

Add `--run` only after admission. `--auto-seed` selects the admitted pending tail,
or one nonce after the highest known exclusion if no allocation is pending. A
dry plan leaves this unresolved and never opens Current. Unknown historical
freshness remains unknown. A restored session consumes one reservation nonce;
that nonce does not replace the retained physical world seed or, when present,
the saved sampling stream. An existing pending allocation must be resumed
exactly, including its unused tail; the host does not silently allocate over an
exclusion or drop unfinished curriculum slots. An
outer curriculum controller remains responsible for its full remaining lesson
schedule; replacing one targeted block with a continuous session must be explicit.

### Dated Alberta night window

For automatic-night admission, use `--night-date YYYY-MM-DD` for the intended
Alberta local date instead of converting the end through host timezone rules.
For example, `--night-date 2026-11-02` derives a 07:00 UTC opening, 14:57 UTC
collection cutoff with the default 180-second final-save reserve, and 15:00 UTC
hard end. An explicitly supplied hard end must agree. The dated policy covers
2026-10-27 onward: current MDT and [permanent Alberta time (ABT)](https://www.alberta.ca/albertas-new-time-system-abt)
from November 1, 2026 are both UTC-6, so the local 01:00-09:00 block remains eight
hours. Stale `America/Edmonton` tzdata that predicts a fall-back must not extend
it to 16:00 UTC. This application guard neither changes OS timezone settings nor
rewrites historical Alberta clock rules.

A plan may be prepared before opening, subject to the existing fresh-end bound;
`--run` refuses early or expired admission before accessing store/owner paths or
invoking Native. The live resource guard also rechecks the dated collection
window. Recovery-only `--reconcile --run` remains available after the window so
an interrupted session can be admitted and retired without starting another
world. The compact owner/final receipt retains the exact window policy. Without
`--night-date`, independently authorized idle engineering work keeps its explicit
aware UTC deadline and existing activity/resource guards.

Keep the user's scheduler DTSTART in `America/Edmonton`; this runtime guard does
not prove or repair future wake resolution in that service. If stale scheduler
rules wake an hour late, only the remainder before the same 15:00 UTC end is
available. Future scheduler occurrences/tzdata must be verified separately when
exposed. An expired October 27 launcher cannot be reused for a later date.

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

The default native path checks the soft collection cutoff inside the world and
has until `hard-end - 60 seconds` to finish its snapshot. The host then reserves
60 seconds for stopped recovery/admission/publication; it never extends the hard
end. With the default 180-second reserve, this gives 120 seconds for native
finalization. Measure actual save/recovery costs before relying on these reserves.
Input/resource guards remain active during finalization. The explicit fresh-world
path retains its earlier forced collection-cutoff stop contract.

The default cadence saves every 32 updates (normally 16,384 trained rows), at
controlled completion and final shutdown. That is a decision bound, not a fixed
wall-clock interval. A crash or guard stop may lose unsnapshotted actor/value and
private-life progress since the last complete snapshot; the reservation remains
consumed and the previous validated baseline is retained. No per-decision durable
save or zero-recent-loss guarantee is imposed.

Native temporary data stays inside its declared aggregate quota. Host control
metadata and two logs are separate, bounded sidecars (each log at most 8 MiB at
guard observations, with possible between-observation overshoot). After admitted
publication, Native cleanup removes the run, and the host clears those sidecars.
One bounded `last-continuous-session.json` remains with compact outcomes and
identities; no episode archive or additional learned checkpoint is retained.
That receipt also commits the stopped owner and exact identity/digest of each
temporary sidecar before removal. Retirement checks the admitted published
current under its writer lease, validates every surviving sidecar, and removes
the owner state last. A retry can use the committed receipt even if the owner
state is absent, without rereading deleted logs or replacing known outcomes with
unknowns. Recreated/conflicting files, changed current and unknown lane entries
are preserved. Historical receipts without this commitment cannot authorize
cleanup. A changed owner from the same committed session is a conflict, even if
the remaining logs are readable. A genuinely new admission records a fresh
session ID and the exact previous-receipt digest, allowing the prior completed
receipt to coexist without granting it authority over the new session's files.
Pending Native `resident-retirement.bin` or `resident-retirement.next` is finished by Native
before Python Store admission while the stopped run owner is still available;
an old final receipt cannot retire a different Native run.
The same pre-Store routing handles Native `resident-initialization.bin` and
`resident-initialization.next` under the stopped startup owner. Native admits the
bound current and aborts only that known zero-dispatch initialization; the host
does not publish a replacement or consume a seed. A fresh launch and a completed
old receipt both refuse those markers rather than adopting another startup.
For a torn first stage, Native may honestly report
`initialization_binding_verified=false`; retain that distinction instead of
claiming a complete request binding. The host still checks the unchanged
admitted baseline and canonical candidate after the zero-dispatch abort.

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

The first hardware slice uses a disposable copy of one admitted living Current:
at least two small PPO windows, one installed-policy boundary, a snapshot and
exact restore. Verify stable world/body identity, monotonic real ticks, retained
private/sampling state and actor/value optimizer ages before a longer timed run.
Keep the accepted baseline immutable and experimental reservations separate.
Record trained versus bootstrap decisions, whole-world unique ticks, collection
and whole-host wall time, PPO/save costs, death/truncation, real nourishing events
and actual prior delivery. A source/mock pass alone is not hardware evidence.

A later one-world versus fresh-world comparison changes world age, food
encounters, setup frequency and update cadence; it is not a controlled claim of
better learning. Equal decision exposure does not imply equal PPO updates.
Follow with retained care competence and repeated need-driven cycles before
claiming benefit. No simulation multiplier is established by this documentation.
