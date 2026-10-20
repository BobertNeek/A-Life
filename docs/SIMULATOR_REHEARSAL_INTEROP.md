# Rehearsal founder and optimizer handoff

This is a follow-on to the bounded PC candidate `b7a67384075223be24e5375a7fd3bfe96995042f`
on PR109. The bounded candidate and its qualification commands remain pinned to
that commit. Do not switch a running qualification to this follow-on.

## Existing gameplay path

The exported `trained.alife-foundation` is the ordinary canonical founder asset.
An explicit experimental New Game selection can use:

```text
--new-game --founder n2048:OUTPUT/trained.alife-foundation
```

`NewGameFounderSelection::N2048Candidate` decodes that asset and the canonical
New Game path records the founder in the genome/save before GPU insertion. Later
load uses the saved genome. This starts new individuals with the genetic founder;
it does not transfer a rehearsing individual's acquired weights, memory or private
state. It does not promote the candidate or change the default founder.

Gameplay does not use the offline actor/value Adam optimizer. Keep the complete
training package separately, with its exact ancestry and optimizer checkpoints,
for later founder optimization. Asset loading alone does not qualify learning
benefit, trainer-removed competence or gameplay performance.

## Bounded candidate package: schema 1

At `b7a67384`, the real behavioral receipt is `rehearsal.json` and the separate
admission marker is `rehearsal-continuation.json` with `schema_version: 1`.
The marker binds four files by exact BLAKE3-256 byte digest:

- `trained.alife-foundation`: canonical founder asset.
- `actor-checkpoint.json`: full `FoundationTrainerCheckpoint`, including Adam
  moments, per-weight update ages, stage mask and optimizer configuration.
- `value-checkpoint.json`: full `PpoValueHeadCheckpoint`, including value Adam
  parameters/age and `last_updated_policy_version`.
- `critic-residual-update.json`: actual full one-step residual critic evidence.

The marker includes policy, founder seed base, biological objective 2, critic
objective 1, final actor/value optimizer ages, verified next-cohort rebind and
decision flags, and `optimizer_reset: false`. The complete behavioral receipt
contains the exact inspected parent file hashes, collection seed/configuration,
attempt/holdout lineages, measured budgets and before/after optimizer ages.
Schema 1 supports this runner's own exact continuation; its marker does not bind
the behavioral receipt or the parent continuation marker. It must not be presented
as the stronger schema 2 contract or as a native `cycle.json` package.

## Separate schema 2 follow-on

The follow-on publishes its marker last, after a complete `rehearsal.json` exists.
It requires `complete: true`, `receipt_file: "rehearsal.json"`,
`execution_domain: "gpu-wgsl"`, the actual collection `seed`, and
`historical_freshness: "unknown"`. It adds `rehearsal.json` to the five bound
output files and binds the complete immediate source identity:

| Source identity field | Meaning |
| --- | --- |
| `policy_version` | Exact source policy before this cohort |
| `founder_seed_base` | Original canonical founder construction identity |
| `foundation_digest` | Canonical source asset digest |
| `actor_optimizer_step`, `value_optimizer_step` | Exact input optimizer ages |
| `file_digests[]` | Actual source filename, encoded byte count, BLAKE3-256 digest |

A native-cycle parent has its four inspected native files, including its actual
`cycle.json`. A schema 2 rehearsal parent has the five output files plus its own
`rehearsal-continuation.json`. Thus the continuation chain preserves the exact
parent marker hash; the parent marker binds its own parent. Keep ancestor packages
or exact retained receipts available when a downstream admission policy needs to
resolve that chain. A missing ancestor is unresolved evidence, not proof of an
accepted root. This does not replace seed-history imports, known exclusions or
recovery reconciliation owned by the ordinary collectors.

The inspector validates the receipt's parent identity, policies, nonzero loss-row
count, all-attempt coverage and optimizer ages against the marker, and validates
the critic evidence's objective/ages/no-reset fields. This runner performs one
actor and one critic update, so each saved age must equal its parent's age plus
one. The value checkpoint must belong to the exported policy and the actor's
weights must match the asset bit for bit. All previously established shape,
phenotype and finite-state checks remain in force. Schema 1 is not silently
upgraded; it needs its own explicit admission decision.

The device-free adapter is:

```text
simulator_rehearsal::admit_simulator_rehearsal_training_source(path, expected_policy)
    -> SimulatorRehearsalTrainingSource
```

It returns the full asset/actor/value, exact `FoundationCheckpointInspection`,
actual receipt/manifest filenames and verified JSON contents, objective versions
and GPU execution domain. It rereads required files against the inspected hashes.
It creates no output, world, device, optimizer or seed reservation. It does not
restore a lifetime or attest historical freshness. The real compiler scene seed
is `manifest.seed`/`receipt.config.seed`; retain `founder_seed_base`, brain class,
sensor profile, development and ABI identities from the canonical asset/compiler.

## Shared training admission still required

The parent is coordinating `FoundationTrainingHandoffV1` in `alife_training` with
the CPU owner after its package schema settles. The rehearsal adapter above must
map to that common type before a shared caller claims cross-backend integration.
It must preserve canonical founder compiler inputs, current policy/objectives,
both complete checkpoints, real receipt filename and exact source ancestry, and
label this package's actual GPU origin. No CPU collector is implemented here.

The group owner's concrete seams are read-only `group_source()` for native model
state and `admission::admit()` for seed/provenance admission. Both need an explicit
rehearsal receipt route: source selection must recognize the actual rehearsal
marker, and provenance must use the actual receipt plus its hash chain and
collection/holdout seeds. Rehearsal retries stay correlated historical practice;
`unknown` historical freshness must survive conversion. No snapshot or old PPO
probability becomes a fresh allocation. A CPU-origin handoff also needs explicit
origin/receipt handling, without relabeling it as GPU-produced evidence.

Ordinary `train_n2048_care --resume` still calls its native-cycle loader and
expects a real `cycle.json`. It cannot resume a rehearsal package with both Adam
histories today. Its owner must add an explicit shared-admission adapter and
verify source-to-care update/continuation on actual hardware before claiming that
route works. Never copy or fabricate native cycle receipts, reset an optimizer,
reuse an old policy's PPO records, or use a founder-only import to conceal this
gap. The strict legacy synthetic-physiology gates remain unchanged.

For policy7400's accepted ages 16684/19800, one qualified rehearsal update must
produce 16685/19801 and a second must produce 16686/19802. An admission-only
conversion leaves all ages, moments, per-weight ages, policy and source bytes
unchanged. Any subsequent ordinary trainer must advance them by its actual
updates and retain the original root ancestry.

Controlling requirements: AOA-FOUND-002, AOA-FOUND-006, AOA-FOUND-007,
AOA-EVO-003 and AOA-ART-004. Gameplay loading is an available mechanism; causal
deployment competence and cross-backend source continuation remain separate
acceptance evidence.
