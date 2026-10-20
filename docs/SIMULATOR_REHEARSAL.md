# Simulator based varied rehearsal

This is an offline experiment over the ordinary causal runtime, not a learned
world model. It does not change the game policy, promote a founder, or certify
unseen-scene generalization.

The runner snapshots the full portable simulator at a sealed boundary every eight
collection decisions. After a useful, measured food acquisition or nourishing
meal, the latest preceding snapshot is eligible for the success pool. Snapshots
contain the actual world and biological state, both acquired weight banks,
recurrent state, eligibility, memory, private prior, sleep, tracking and cognitive
sidecars through the existing exact codec. No private state is made sensory input.

The next attempt can move the same food left, right or farther away, or insert a
small obstacle on either side. Placements use body/object radii and terrain
walkability, reject held/consumed targets and solid intersections, and mutate a
clone atomically. Invalid variations are counted and fall back to a newly
constructed scene. Heading and hunger edits are deliberately unsupported until
there is an appropriate validated scene API; no drives or reward labels are
written by this experiment.

A cohort freezes the current genetic actor and value head. Every retry samples
new actions through ordinary perception and executes actual world consequences.
The success and difficult-failure pools are bounded separately and stratified by
source scene and variation. Every fourth slot reserves a new scene, and every
fourth offset slot reserves a difficult failure when available. Successes come
from exact target acquisition/consumption facts. Failures require actual blocked,
collision, aversive or terminal consequences. Empty pools fall back to new scenes.

All trainable attempts enter the first actor accumulation batch before any policy
update, including failures. Burn-in rows have no actor/value loss. Truncated
attempts bootstrap their actual final decision; death uses a terminal boundary
and canonical terminal biological credit. A sleep interruption trains the valid
prefix rather than discarding the cohort. The critic retains the source's full
one-step Bellman residual objective on captured GPU features. Both Adam histories
continue; the source checkpoint is read-only. A verified next-cohort rebind and
`rehearsal-continuation.json` support another invocation without resetting either
head. The in-memory snapshot pools expire at the policy update: older actions and
probabilities are never admitted as current-policy PPO. Persisted snapshots remain
historical simulator evidence, not automatically admissible next-policy branches.

## Run

Requires the training runtime restore constructor, a hardware GPU supported by the
canonical runtime, and a verified native objective2 source checkpoint (or an exact
simulator continuation produced by this runner). No CPU fallback is selected.

Fresh scenes use the inspected source asset and its original founder seed. The
seeded continuity fixture uses the shared procedural bootstrap factory, which
first constructs a canonical newborn and then compiles its expressed genome.
A raw scaffold is only a provisional New Game construction token: its decoder
ABI can differ from inherited founder expression. Admitted checkpoint assets are
never passed through that factory or rebound to new metadata. Scene preparation
and production resident admission have device-free regressions; setup errors
identify the stage, founder, scene and rollout while retaining the original cause.
New procedural bootstrap weights and metadata now derive from the expressed
conception-seed genome, so they can differ from historical raw-scaffold bootstraps.

```sh
cargo run -p alife_game_app --features foundation-training --bin rehearse_n2048_care -- \
  SOURCE_CYCLE NEW_OUTPUT EXPECTED_SOURCE_POLICY SEED
```

Defaults: 16 attempts, at most 32 decisions per attempt including two burn-in rows
and a bootstrap decision, 600 seconds of collection admission time, 16 success
snapshots and eight difficult-failure snapshots. Source admission, in-flight
captures/ticks, optimizer work and holdout evaluation all have measured wall
costs; the collection ceiling stops new decision admission, not an in-flight
atomic operation. Update and holdout costs are reported separately. Snapshot
assets have a 512 MiB ceiling with conservative per-capture headroom; ordinary
replay uses the existing host/disk ceilings.

The setup path returns its exact save only after durable publication, reopen
validation and rebind. Decision zero consumes that save for the pool snapshot;
serialization still validates assets, restores the world and records the same
BLAKE3 lineage. No simulator progress occurs between setup and that consumption.
Unused setup state is discarded before the first tick, and later eight-decision
boundaries still export current GPU state. No live runtime/device cache is added.

`pool_snapshots` counts serialized pool IDs separately from `snapshot_captures`
and `snapshot-capture-ledger.json`, which count actual explicit GPU save exports
(including setup and holdouts). With full attempts and every requested capture
admitted, qualification needs 6 exports instead of 10, with 4 pool snapshots;
default mode needs 66 instead of 82, with 64 pool snapshots. These are source-
derived counts, not measured runs or speedups. A previous capture measured
9.6569 seconds; multiplying it by removed exports does not measure saved time.

For the parent's short hardware qualification, append `--qualify`: four attempts
of eight decisions and two eight-decision holdouts. Collection admits work for
240 seconds from source entry, including setup/captures; a separate 360-second
whole-run allowance covers updates, holdouts, release and finalization. These are
planning allowances, not measured completion guarantees. Follow the serial execution and
external safety-stop instructions in
[Short qualification handoff](SIMULATOR_REHEARSAL_QUALIFICATION.md); do not run the
default comparison during the PC's Minecraft workload.

Append `--control-fresh` for newly constructed scenes only. Append `--compare` to
run that control and varied rehearsal from the same source, with the same
configured ceilings and disjoint within-run holdouts. `comparison.json` reports
actual decision counts, discovery, successful rehearsal, holdout outcomes and
all stage times. Unequal counts and time-limit exits remain explicit. Arm order
is recorded because GPU/cache warm-up can affect wall time. A single run does not
establish a learning or throughput advantage.

`rehearsal.json`, per-attempt receipts, full `snapshot-*.json` files and their
BLAKE3-bound lineage records distinguish parent scene, snapshot, perturbation,
current policy and sampling seed. `stage-timing.json` persists actual entry-clock
stage boundaries, failure status and decision counts, including on unsuccessful
runs. Its synced append-only `stage-timing.jsonl` retains earlier complete records
if an external stop interrupts the latest summary. Nested stage durations are inclusive and must not be added together. The
behavioral receipt’s `elapsed_seconds` ends after resource release; timing JSON
also covers final publication/verification. Compact replay records preserve the actual
perceptions, sampled commands, behavior probabilities and sealed outcomes. The
schema 2 continuation manifest binds exported weights, complete actor/value
states, residual critic evidence and the actual `rehearsal.json` receipt. It is
prevalidated before publication and checks the source policy, exact parent hashes (including a
parent rehearsal manifest), founder identity and one-step optimizer-age advance.
Qualification-source admission also requires completed in-budget finalization
timing; an interrupted publication cannot qualify by marker presence alone.
An incomplete or altered package cannot become a continuation. Earlier schema 1
packages are not silently upgraded. Native cycle files are never fabricated.

`training_reward_sum` contains only actual training-loss-row rewards. It is null
for untrainable attempts and holdouts; holdout outcomes report real useful-success
counts, decisions and times, rather than inventing a zero reward measurement.
Parent packages or retained receipts remain necessary to resolve the ancestry
chain back to a separately accepted root; this manifest does not certify missing
ancestors or campaign-wide freshness.

Newly constructed scenes are new within this experiment; this runner has no
campaign-wide seed ledger. All historical freshness remains **unknown**. Holdout
seeds are disjoint from every planned collection scene, but historical unseen-scene
generalization remains **unknown**. Rehearsal outcomes are correlated evidence.
The strict defaults and recovery evidence of the separate mixed-care runner are
unchanged.

## Verification and limits

CPU tests exercise actual canonical world branching/metabolism, near-food geometry,
invalid-state rejection, pool diversity, policy-version rejection, seed partitioning
and sequence masks. They do not certify neural gameplay. GPU restore continuity,
end-to-end source continuation and matched learning comparisons require a supported
hardware session. The private policy7400 Library identity exists and the PC
verified the upload/manifest. Cloud consumer byte validation is blocked by proxy
CONNECT 403. The cloud has no hardware GPU. The parent owns coordination of any
short PC execution; this document starts no workload or provisioning.

Standalone delivery requires the current-main checks, R2 review, final-head CI,
exact-restore hardware continuity and accepted policy7400 source-to-update-to-own-
continuation qualification without resetting either Adam state. A matched
discovery/retention comparison is required before claiming benefit; it is not an
additional merge gate for this opt-in experiment. No founder is promoted here.

The existing gameplay choice `--new-game --founder
n2048:OUTPUT/trained.alife-foundation` loads the canonical genetic asset. Gameplay
does not import the offline Adam optimizers or a rehearsing individual's private
state. Ordinary care resume and CPU/group common admission remain separate,
unimplemented routes and do not gate standalone delivery. The full ancestry
follow-on remains preserved at `691ba2c84524ccaf110e97875d19afa016791847`;
only its own-package validation is integrated here.

Relevant controlling requirements: AOA-INV-009 (same causal step), AOA-BIO-026
(measured biology), AOA-CTX-002 (canonical interoception), AOA-FOUND-004 (world
variation), AOA-FOUND-006 (deployment mechanisms), AOA-SLEEP-007 (replay identity).
Implementation evidence belongs in the dated verification receipt, not the
controlling architecture.
