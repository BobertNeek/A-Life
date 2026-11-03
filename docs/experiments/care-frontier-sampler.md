# Bounded care frontier experiment

`scripts/care_frontier_sampler.py` is an external, per-batch curriculum selector.
It consumes completed native CPU care assessments from one frozen actor/value
source. It changes task selection only: native biology, rewards, PPO, Adam,
topology, semantic-prior implementation and accepted checkpoints are unchanged.
It is inspired by adaptive problem generation, not an implementation of Dr. Zero
or HRPO. No LLM is called and no training-speed benefit is claimed.

The task bank explicitly lists prewritten lesson/need-condition templates,
native seeded placements, a host difficulty ladder, future training seeds and
separate holdout seeds. Unknown fields, arbitrary/custom placements, reward
overrides, unsupported lessons, duplicate seeds and discovery/holdout overlap
reject. Native scene construction, need admission and action legality remain
authoritative. A legal template does not guarantee a particular solver can solve
it. Zero observed successes are unresolved/too hard, not proof of impossibility.

For a batch with at least three current-tier observations, success at or above
75% raises difficulty one tier; success at or below 25% lowers it one tier;
otherwise it holds. Bounds always apply. A success requires native exact-target
nourishment and positive health throughout the evaluated window, with full
bootstrap capture. Mixed-success target-tier templates receive weight
`4*p*(1-p)*survival_rate`. Observed all-success templates are excluded from the
frontier but can provide a lower-tier retention dose of 25% rounded down. No
dispatch is fabricated when evidence cannot fill the requested batch. The
default held/reachable/aged-feeding/newborn-feeding order is a host hypothesis;
monotonic difficulty has not been established.

Baseline selection is deterministic balanced round-robin and independent of
outcomes. Both learning arms request the same decision budget and unchanged
native training settings. Actual optimizer updates/early KL stops must also be
compared before claiming a matched learning result. A ready adaptive learning
run has not yet been demonstrated by the native pilot below.

Every public assessment import requires an explicit native evaluator. Both the
standalone CLI (`--native-evaluator`) and fixture runner invoke the existing
native completed-output loader on the exact selected, bounded archive. Native
checks its typed V1 checksum, source/history admission and full private recovery;
Python does not emulate Rust path ordering or receipt serialization. The returned
result must equal the sealed result, and archive/executable hashes must remain
unchanged. This reuses a completed assessment without another live dispatch.
Python also checks archive hashes, full-dose trace, offline-head identity and
teacher/prior/bootstrap boundaries before selection. Every future batch must use
fresh discovery seeds and one current frozen source. Holdout results never enter
the selector.

Build the disposable CPU harness and run a fresh output (never resume):

```sh
cargo build --locked -p alife_game_app --no-default-features --features cpu-training --bin train_cpu_care --bin train_cpu_care_round
python3 -B scripts/run_care_frontier_fixture.py --output NEW_DISPOSABLE_DIRECTORY --care-binary PATH_TO_TRAIN_CPU_CARE --round-binary PATH_TO_TRAIN_CPU_CARE_ROUND --decisions 64 --discovery-per-family 4 --slots 4
python3 -B -m unittest discover -s scripts -p 'test_care_frontier_sampler.py'
```

The ordinary Python tests mock native admission while exercising the structural
and refusal guards. To also run the genuine sibling-path regression with two
negative tamper controls, point it at the just-built CPU executables:

```sh
ALIFE_FRONTIER_TEST_CARE=PATH_TO_TRAIN_CPU_CARE ALIFE_FRONTIER_TEST_ROUND=PATH_TO_TRAIN_CPU_CARE_ROUND python3 -B -m unittest discover -s scripts -p 'test_care_frontier_sampler.py' -v
```

That test generates a fresh disposable founder, places `source-history.json`
beside the `source` directory, creates one two-decision native assessment, and
requires unchanged read-only reuse. It then rejects receipt-only and artifact
byte tampering. This layout exercises the difference between native path
component ordering and whole-string sorting without inventing another checksum
implementation. Without both explicit binaries the native test is skipped.

The runner has no accepted-source input. It creates a new founder, preserves raw
commands/results/diagnostics, pins its binaries and completed sources, and never
retries interrupted lives. Discovery and holdouts use native `evaluate`, which
removes teacher/prior and performs no offline actor/value updates. An empirical
frontier permits one matched-budget learning batch and paired holdouts. Without
a frontier it records a frozen fixed-budget reference and stops training.

The actual public-fixture pilot used Rust backend source `b2f7a81f`, dev/debug0,
temperature 1 and a two-decision disposable bootstrap. Its 16 discovery cases,
eight holdouts and four frozen-reference cases each evaluated 64 decisions:
**zero nourishing meals in all 28 cases**, positive health throughout every
evaluated window. The reviewed controller lowered tier 1→0 and returned
`no_sufficient_empirical_frontier`; both learning arms remained undispatched.
The four-case reference covers 256 evaluated decisions, not training. Raw compact
trial metrics, native receipt hashes, binary hashes and process costs are in
`care-frontier-fixture.json`. Original native archives/logs remain local; no
trained/checkpoint artifacts are published. No efficacy or speedup follows from
this result. More useful solver competence or finer validated templates are
needed before a learning comparison can run.

## Optional recipe authors and ASCII maps

An LLM/Luna can propose template IDs or help author new templates off the hot
path. Its JSON is untrusted input. Host code owns the allowed template IDs,
budget, difficulty bounds, seed allocator and holdout ledger; model text cannot
choose rewards, executables, file paths, code or acceptance. The cheap first
model comparison is a fixed number of proposed allowlisted template IDs versus
the programmatic selector, charged for proposal/validation/assessment cost.
There is no current evidence of an LLM benefit.

ASCII maps are a future optional front end to versioned structured scene JSON,
not supported by this sampler. A grid/legend can define roles (`A` learner,
`F` food, `#` obstacle, `.` ground), cell size/origin and finite seeded rolls for
allowed variants. A deterministic compiler should emit ordinary engine
coordinates plus the recipe/seed/generated-scene hashes. Validate rectangular
bounded dimensions, unique required roles, finite physical sizes, valid terrain,
clearance/collisions, supported goal predicates and traversable routes using the
engine's movement authority. Reject invalid/unreachable proposals with bounded
work; do not silently reroll until a favorable scene appears. Goals are scored
from world outcomes, never model declarations. Existing continuous non-grid
scenarios remain supported. Custom map compilation/reachability and additional
goals need their own implementation and physical positive controls.
