# Native CPU foundation training backend

Dated implementation/evidence report, updated 2026-10-20. This report covers
PR110 on `feat/cpu-foundation-backend`; it is not architecture authority.
The user's 19 October 2026, 23:17:19 UTC instruction authorized “create a cpu
backend that we can offload some training to the cloud with” alongside larger
groups. The root and crate guardrails now record this explicit offline scope.
AOA-ADM-007 makes processor placement an implementation mechanism and
AOA-TIME-006 describes GPU placement as a reference mechanism. The controlling
architecture is unchanged. Normal gameplay retains its GPU default and rejects
CPU policy saves.

## Executable boundary

`alife_training/cpu-backend` executes the compiled N2048 sparse graph directly
in Rust. It creates no graphics device and needs no Vulkan ICD. It does not call
the legacy simplified CPU reference controller or software Vulkan. Founder
weights and optimizers remain outside private organism state.

```sh
cargo run --locked -p alife_training --features cpu-backend \
  --bin train_cpu_foundation -- \
  --previous /path/to/sealed-native-or-cpu-cycle \
  --output /path/to/new-output --seed EXPLICIT_NEW_SEED --ticks 64
```

A serial `cycle.json` directory must first pass the existing native serial,
terrain and critic-correction continuity verifier and export a hash-bound typed
package. The export command reads the original directory without changing it:

```sh
cargo run --locked -p alife_game_app --features foundation-training \
  --bin export_native_training_handoff -- /path/to/sealed-serial /path/to/new-portable
```

Use `/path/to/new-portable` as the CPU command's `--previous`. Its
`native-training-handoff.json` pins the original cycle, asset and complete
actor/value checkpoint bytes, plus `training-handoff.json`. A missing binding or
changed finite moments, ages, stage mask or critic parameters fails before CPU
trainer/world/output construction. Native correction proof is checked at export;
the typed package retains the actual corrected critic age without rewriting the
historical cycle receipt or resetting either optimizer. The package declares
biological objective2 / residual critic objective1 and fresh-cohort continuation.
CPU and group50 sources retain their existing sealed hash/receipt checks.

The export is the trust boundary for ordinary historical serial checkpoints:
the existing native verifier checks their declared continuity but cannot
retroactively authenticate finite moment/mask edits made before export when the
old receipt carried no checkpoint hashes. The portable binding protects the exact
selected state after export. Declared critic-correction packages additionally
retain their existing full source/target snapshot proof. Export output cannot
reside inside either source tree, including through parent-directory symlinks.
This is integrity/continuity evidence, not accepted-policy or GPU qualification.

`--fixture` replaces `--previous` only for deterministic development evidence;
its fixture label propagates to descendants. The CLI uses a new homogeneous
cohort with the inherited founder and retains the old individual archive. It
never claims a terminal GPU individual restore. A CPU cohort's own full private
checkpoint can be restored under its collection policy. Heterogeneous compiler
graphs require per-row compilation and are rejected at this boundary. The
separate group50 worker owns population placement/vectorized execution.

Seed allocation remains caller-owned. The CPU receipt records historical
freshness as **Unknown** and preserves the complete source receipt and exact
source hashes. It does not reserve seeds or claim accepted mixed-care ancestry.
The source must be a completed native handoff (`next-cohort-admitted`) or a
completed CPU handoff (`complete-fresh-cohort-cpu`), or complete group50 native receipt, with matching asset, full
checkpoints, objective versions and ages. Specialized critic-correction sources pass that existing native admission at
portable export; this CLI does not bypass it or repair/reset an optimizer.

## Native computation and canonical collection

- Sparse recurrent routes, cadence, activity masks, detached lifetime plus
  alpha-times-fast offsets, structural edges, nonlinear dendrites, receptor
  projection/threshold effects, intrinsic leak/activation, EMA and metabolic
  state follow the production shader equations.
- Candidate/memory/cognitive decoder channels, native joint sampling and
  six-step speech decoding use actual canonical perception and legality.
  Shared frame preparation, attention, context and construction development
  are extracted without changing the GPU helper bodies. GPU uploads remain
  in their existing wrapper.
- The collector runs ordinary world motor/biology transactions, neural and
  recognized social emissions, successor prediction, language grounding,
  memory/topology updates and sealed measured physiology. Distinct CPU neural
  evidence survives credit and packed logging; GPU credit rejects CPU patches.
- Elapsed-tick eligibility, selected-candidate deduplication, receptor-projected
  Hebbian/Oja updates and developmental ceilings update private somatic state.
  CPU organism schema2 preserves active and inactive activation banks, including
  lower-level zero-step collection eligibility. Production ATP policy has a
  two-step minimum. The host uses its exact native route pruning, work counts
  and Q24/Q16 affordability, with neutral hardware pressure. Unaffordable work
  enters canonical recovery sleep; it never forces extra neural steps.
- Native sleep control, bounded replay, structural discovery, consolidation,
  fast/lifetime promotion and actual sleep work are causal. Waking resumes only
  after the normal phase/physiology boundary. Complete checkpoints bind world,
  biology, both neural banks, lifetime/fast weights, traces, sidecar metadata,
  memory compaction/staging, predictor, sleep, language, sequence and RNG state.
  Failed steps roll back the whole private/world transaction.
- PPO uses joint representative/six-channel likelihood, exact conditional
  entropy, clipping and receipt temperature. GAE follows actual transitions,
  death and sleep gaps; unobserved final successors are not invented. Burn-in
  has no actor loss. Bounded microbatch tapes accumulate sample-weighted BPTT.
  The linear critic uses the full sampled Bellman residual derivative through
  current/actual next features and bias, with native Adam.
- `FoundationTrainerCheckpoint` and `PpoValueHeadCheckpoint` retain all weights,
  moments and exact integer ages. Frozen coordinates remain unchanged. Fresh
  founder rebind validates all non-genetic graph/ABI coordinates. Neither actor
  nor critic is reset at continuation or export.

## Portable GPU return seam

`FoundationTrainingHandoffV1` is available without the CPU feature. It contains
exact native compiler inputs, canonical candidate asset, full actor/value
checkpoints, policy version, objective versions 2/1, execution origin and explicit
fresh-cohort continuation. `validate()` / `admit()` are device-free.
`into_gpu_training_session()` restores the existing real native GPU Training
session and both optimizers. CPU decisions are never relabeled GPU decisions.
The exported `.alife-foundation` uses the existing gameplay asset codec.

The CLI writes `training-handoff.json`, `trained.alife-foundation`,
`actor-checkpoint.json`, `value-checkpoint.json` and `cpu-cycle.json`.
The CPU receipt has `complete=true`, `mode=native_cpu_fresh_cohort`,
`receipt_filename=cpu-cycle.json`, `backend=native-cpu`, exact output SHA256s,
the original receipt once under `seed_provenance.native_source_receipt`, and `seed_provenance.source_sha256` plus
`seed_provenance.native_source_receipt`. The ancestry pin set uses the actual
source receipt filename and the three native model files; all consumed source
bytes, including phase/handoff files, are separately retained and rehashed.

Group50's existing read-only source/admission reader recognizes this receipt
filename/domain and validates the typed handoff, native checkpoints and SHA256s
before any reservation or device access. Immutable source copies include the CPU
phase marker and are themselves validated. Latest-known-lane admission verifies
intermediate receipt pins and compares the native terminal receipt with the exact
surviving mixed checkpoint; older roots and edited intermediate links are rejected.
Historical freshness remains Unknown under known-exclusions. CPU continuation also
accepts exact complete group50 native output, preserving weights, moments, ages and
fixture ancestry in either direction. These are source-admission changes; group
population, collection kernels and GPU evidence domains retain their existing path.
Real hardware execution of the GPU import remains **Unknown** on this CPU host.

## Validation and measured work

```sh
VK_ICD_FILENAMES=/no-vulkan-icd.json cargo test --locked -p alife_training --features cpu-backend --lib cpu::
VK_ICD_FILENAMES=/no-vulkan-icd.json cargo test --locked -p alife_training --features cpu-backend --test cpu_backend --test cpu_collector --test cpu_cli --test backend_handoff --test checkpoint_validation --test blocked_legacy_pipelines -- --nocapture
cargo clippy --locked -p alife_training --features cpu-backend --lib --bins --test cpu_backend --test cpu_collector --test cpu_cli --test backend_handoff -- -D warnings
cargo check --locked -p alife_training --features cpu-backend,gpu-tests --test cpu_backend --test backend_handoff
```

Tests cover recurrence/plasticity formulas, finite-difference recurrent/decoder/
speech and PPO entropy gradients, forced probabilities, masks, burn-in, private
isolation, tamper rejection, sleep-to-wake, minimum ATP work and unaffordable
sleep, full residual critic Adam, native asset return and exact checkpoints at
actor age 16684 and critic age 19800. Coverage includes age 16777217 and overflow
rejection. CPU CLI tests run with an invalid ICD path, continue learned state
without resets, preserve immutable source bytes and reject incomplete output.
The GPU differential/import tests are compiled here, not executed on llvmpipe.
Independent source review cleared the numerical and canonical integration;
fixes bind all private sidecars/RNG and use exact executed work counters.

Unoptimized cloud fixtures on AMD EPYC 9V74, four-core quota, 16 GiB RAM, no GPU:
five sealed canonical decisions plus a serialized exact restore completed in
1.002632 s after integrating current founder/group50 main. A separate seven-decision
trajectory produced six real training rows, advancing actor 16684→16685 and critic
19800→19801 in 0.120602 s of update work.
The earlier executable 12-tick fixture at integration commit b21d266d collected 12 valid canonical decisions
and 11 rows in 0.991735 s (update 0.292100 s); its continuation collected another
12 decisions in 0.952347 s (update 0.276023 s), preserving ages 2→4.
[Structured fixture results](20261020-native-cpu-fixture-results.json) pin the
earlier receipts and native outputs. These executable timings precede the current
founder corrections; current-main throughput is not inferred from them.
These bounded fixture measurements do not establish a production speedup or
accepted-policy behavior. Updated exact-head CI is recorded on PR110.

Core evidence/packed compatibility regressions pass 28 tests. CPU policy config
and gameplay boundary tests enforce the explicit offline mode. Documentation
checks pass 77/77. Existing legacy training/evaluation gates remain blocked.

## Accepted policy qualification: Blocked

The pinned Library package
`alife-policy7400-accepted-checkpoint-57files-20261019.zip` was requested through
the supported materializer, initially and once more under the authorized bounded
retry. Both returned no workspace path; downloading each returned URL failed
with `Tunnel connection failed: 403 Forbidden`. No bytes were materialized,
no payload hash was verified, and no accepted source was trained or changed.
No alternative executor/service or additional download retry was attempted.

The accepted policy's 32768 actor weights and actor/value ages 16684/19800 remain
unmodified. Actual accepted-package CPU continuation, latest-known-lane group50
admission and terminal GPU individual restore are **Unknown**, pending source
access and their respective qualification. Integer-age fixtures are numerical
regressions, not substitutes for the unavailable package. PR113 founder corrections
are included: CPU Adam applies inherited sign restrictions only to recurrent
synapses, preserving signed action/speech readouts. PR112's credit-weighted Oja
change remains unmerged at this report revision; CPU waking plasticity follows the
current production shader and must follow that merge before new qualification.

## Requirement trace

| Requirement | Scope/evidence |
| --- | --- |
| AOA-ADM-007, AOA-TIME-006 | Explicit offline CPU placement and receipt; GPU gameplay default/boundary retained. |
| AOA-BRAIN-006/007/009/011/014 | Native sparse/recurrent/somatic equations, gradients and formula tolerances. |
| AOA-LEARN-002/003/005/010 | Sealed CPU physiology/receptor credit, private plasticity, actual PPO/residual critic and sleep replay. |
| AOA-AUTH-003, AOA-INV-009 | Ordinary organism-owned world biology; score-free enumeration; no synthetic physiology in training. |
| AOA-PERSIST-002/004 | Exact native weights/moments/integer ages and full CPU private archive; accepted package/hardware restore qualification Unknown. |
