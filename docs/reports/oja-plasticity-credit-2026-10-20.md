# Credit-gated production Oja stabilization — 2026-10-20

## Result and scope

The waking production shader applied Oja contraction on every accepted outcome,
including outcomes whose receptor-projected third factor was exactly zero.
It left genetic/lifetime arrays intact but trained bounded fast weights to
counteract their expressed strength. The correction multiplies that existing
effective-weight normalization by the **magnitude** of local credit. It retains
the normalization target, both rate controls, alpha placement, fast bounds,
eligibility timing, transactional guards, and all sleep/replay operations.

Isolated execution of the production WGSL helper and the original formula on
SwiftShader passed 181 paired vectors. At 100,000 zero-credit outcomes, a
representative alpha-1 decoder retained 0.00011920929 of its initial effective
0.95 before the change and all 0.95 after it. An alpha-0.25 decoder reached its
fast-clipping floor of 0.25 from 0.8 before the change and retained 0.8 after it.
These are controlled shader results, not a trained founder or gameplay score.

Source baseline: `bda3b65be5fa6f01865b850640adf2e8d01f4990` (fetched main).
The primary implementation is
`crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl`.
No checkpoint, learned asset, Adam history, CPU-backend file, digest contract,
or qualification candidate is modified. PR #109 at `b7a67384` has no
GPU-backend file overlap; PR #111 was observed at pinned `785d392c`.
Repository/crate/docs AGENTS instructions were read. No `.agents/skills`
directory exists in this checkout or the shared `/workspace/.agents` directory.

## Formula and actual gates

Let `G` be genetic weight, `L` lifetime weight, `F` fast weight, `a` alpha,
`p` postsynaptic activity, `e` credited eligibility, `m` local third factor,
`eta` effective learning rate, and `lambda` effective normalization rate.

```text
E = G + L + a*F
old_delta_F = eta*a*m*e - lambda*p*p*E
new_delta_F = eta*a*m*e - lambda*abs(m)*p*p*E
F_next = clamp(F + delta_F, receptor.fast_min, receptor.fast_max)
```

The evaluator now calls `fast_plasticity_delta`; it still loads current active
banks, stages the unchanged lifetime value, and uses the same clamp. There is
no save-format or arena-layout change and no history migration/reset.

* Credit is the signed dot product of eight outcome lanes and the heritable
  receptor vector, divided by the vector's L1 norm and clamped to `[-1,1]`.
  An all-zero receptor returns zero. Opposing nonzero lanes can also cancel.
* Lanes are prediction residual, pain, homeostatic improvement, frustration,
  novelty, social consequence, biochemical appetitive, biochemical aversive.
  `OutcomeCreditPacket::with_biochemical_receptors` gates the last two by
  measured improvement/aversive consequences. Baseline hormone tone alone is
  not credit. No teacher/answer labels are introduced.
* General default weights are `[.2,-1,1,-.5,.2,0,.5,-.5]` (L1=3.9).
  Signed-choice action/cognitive/memory profiles omit prediction residual
  (`[0,-1,1,-.5,.2,0,.5,-.5]`, L1=3.7). For example, isolated improvement
  1 projects to +0.25641024 / +0.27027026, isolated pain 1 to the negative
  counterparts, and improvement .01 to +0.0025641024 / +0.0027027025.
  Equal pain and improvement with other lanes zero project to zero.
* Compiler policies set inherited receptor rates. Fixed receptor rows have
  zero learning/normalization/replay rates. Enabled procedural recurrent
  routes use `mask_scale * .1`; the default TemporalPredictive→ActionPlanning
  mask is .5. N2048 uses its section slow/fast policy (.25/1) and projection
  scaling. Decoder rates use their explicit inherited head/family policy.
* Current developmental modulation multiplies recurrent rates by
  `1 + max(matching open critical-period bias)` and caps them at inherited
  global ceilings, independently for learning, replay, normalization. Fixed
  zero rates remain zero. Decoder rates bypass this transient multiplier.
  The existing `developmental_plasticity` integration tests compare transport
  against independently recompiled phenotype rates, including caps/overlaps.
* Alpha is independently inherited and normalized to `[0,1]`; the default .25
  becomes .125 on the default .5 mask. Some memory decoder rows use alpha 1.
  It multiplies the learning term in **fast** space and fast expression again
  in effective space, so the effective learning drive contains `a*a`.
  Disabling both genome Oja/Hebbian flags compiles alpha to zero; these flags
  are not separate runtime branches around the two terms in this shader.
  Alpha zero suppresses waking fast expression/learning drive, not the
  normalization term in fast-coefficient space. With nonzero local credit and
  nonzero normalization rate, latent fast changes can still enter lifetime
  weights during raw-coefficient sleep promotion. Only a zero inherited rate
  row is fully delta-disabled; this separate existing alpha/sleep semantic is
  not changed by the zero-credit fix.
* Activity-route masks gate new recurrent coactivation, not the normalization
  evaluator. A throttled route decays its old eligibility. Decoder eligibility
  follows selected motor/head semantics. The plasticity pass still visits all
  synapses after the sealed transaction passes identity/generation guards.
  Zero eligibility alone therefore did not stop the old Oja term.
* Eligibility is clamped to `[-1,1]` when accumulated; at outcome it is further
  decayed by `reference_decay^(outcome_tick-origin_tick)`. Default reference
  decay is .95 per .05-second simulation tick. Delayed outcomes attenuate the
  learning drive; the Oja term itself has no elapsed-time multiplier. It runs
  once per accepted outcome, not once per recurrent microstep or wall-clock
  frame. Counts below mean committed outcomes under maintained activity.

## Clamped and unclamped retention

At zero projected credit, the original **unclipped** recurrence is
`E_n = E_0 * (1-a*lambda*p*p)^n`. The fast coefficient is bounded; it is not
an unbounded weight process. For nonnegative alpha and a stable monotone step,
the effective trajectory stops at the nearest reachable endpoint of
`[G+L+a*fast_min, G+L+a*fast_max]` if that interval excludes zero.

For `G+L=.75`, `F_0=.2`, `p=.5`, and bounds `[-2,2]`, the closed-form old
effective values (including clipping) are:

| Compiled coefficient class | alpha | lambda | E_0 | 1,000 | 10,000 | 100,000 |
|---|---:|---:|---:|---:|---:|---:|
| Ordinary decoder / N2048 fast | .25 | .001 | .8 | .751529 | .428201 | .25 |
| Memory decoder | 1 | .001 | .95 | .739838 | .077956 | ~0 |
| Procedural masked slow | .125 | .00005 | .775 | .773790 | .762985 | .662893 |
| N2048 unmasked slow | .25 | .00025 | .8 | .787597 | .684275 | .25 |
| N2048 masked slow | .125 | .000125 | .775 | .771979 | .745310 | .524391 |
| N2048 masked fast | .125 | .0005 | .775 | .762985 | .662892 | .5 |

The new zero-credit result stays exactly `E_0` for every row and horizon when
the initial fast coefficient is within its existing bounds. Old unclipped
half-lives at `p=.5` are about 11,090 outcomes (ordinary decoder), 2,772
(alpha-1 memory), and 443,614 (procedural slow). For the ordinary decoder,
unclipped retained fractions at 1k/10k/100k are .939411/.535251/.001930;
clipping instead stops retention at .25/.8 = 31.25% in this example.
At `p=.1`, actual WGSL gives .798002/.780248/.623035 from .8 over those
horizons. At `p=1`, it gives .623021/.25/.25. Negative base -.75 at `p=1`
has the symmetric -.25 floor. Alpha zero leaves effective strength unchanged;
zero inherited rates disable both old and new updates.

The compiled default inventory (seed 42, age tick 0, maturation .35,
GroundedObjectSlotsV1) is
also printed by the regression test. N512/N1024 have respectively 123/431
enabled slow recurrent synapses and 431/1551 fixed ones, plus 984 alpha-.25
and 96 alpha-1 decoder rows. N2048 has 2048 masked-slow, 1024 masked-fast,
11008 unmasked-slow, 10496 unmasked-fast recurrent rows, and 4096 rows in
each decoder alpha class. These are scaffold defaults; trained assets can
express different genetics, rates, activity, and receptor profiles.

## Why this retains stabilization

For constant `p,e,m` and nonzero alpha, define
`h = a*lambda*abs(m)*p*p` and `d = eta*a*a*m*e`. Before clipping:

```text
E_next = (1-h)*E + d
E_star = sign(m)*eta*a*e/(lambda*p*p)       [m != 0, lambda*p*p != 0]
```

At `m=0`, both terms vanish. At `abs(m)=1`, the update is identical to the
original rule. Small credit slows the entire step; it no longer leaves an
unscaled cancellation term dominating a tiny learning drive. For `0<h<=1`,
the map is a contraction toward `E_star`, and clipping is nonexpansive and
selects the attainable endpoint if the fixed point is outside the fast range.
Changing the sign of credit changes the drive, not the contraction sign.
For founder tanh dynamics (`|p|<=1`) and validated alpha/rate/credit bounds,
`0<=h<=1`. At `p=0` or `lambda=0`, there is no Oja contraction; the existing
fast clamp still bounds learning. These are local fixed-activity guarantees,
not a proof of convergence of arbitrary recurrent networks. Custom unbounded
identity/ReLU activity can exceed this contraction regime; existing clamping
and finite-value transaction guards remain, with no newly claimed global
stability guarantee.

Using signed `m` on normalization would make punishment expansive
(`1 + a*lambda*abs(m)*p*p`). Fast-only normalization would regulate the fast
coefficient while leaving inherited/consolidated strength outside its target;
it would change the effective fixed point by a base-dependent offset. It also
would still decay existing fast knowledge at zero credit if left ungated.
Keeping effective-weight Oja with magnitude gating is the narrower correction.
Nonzero local credit with zero eligibility can still contract expressed
strength: this intentionally retains Oja stabilization during locally credited
events rather than adding a new eligibility/activity-route gate.

Actual paired WGSL learning outcomes for `G+L=0`, `F_0=.2`, `a=.25`,
`eta=.01`, `lambda=.001`, `p=.5`, `e=.01`, 100k outcomes:

| Local credit | Old E | Corrected E | Meaning |
|---|---:|---:|---|
| +1 | .09989677 | .09989677 | Same positive fixed-point behavior |
| -1 | -.09971043 | -.09971043 | Same stable negative learning |
| +.01 | .001094576 | .052980233 | Moves toward +.1 at the smaller step rate |
| -.01 | -.000901566 | .040900560 | Moves downward toward -.1 at the smaller step rate |

The exact real-valued fixed points are ±.1; f32 recurrences stagnate at a
small distance from them. Stronger sustained drives reach the existing ±.5
effective endpoints. Maximum `a=lambda=abs(m)=p=1` is covered as well.

CPU mirror vectors, in fast space: `eta=.01, lambda=.001, a=.25, G+L=.75,
F=.2, p=.5, e=.4` produce deltas `0, +.000008, -.000012, +.0008, -.0012`
for `m=0,+.01,-.01,+1,-1`. Only the normalization magnitude gate changes.

## Life, sleep, and retained history

Waking applies the same normalization pressure repeatedly over the lifetime;
the old decay is not reset by a quiet or zero-credit event. Sleep replay uses
stored outcome projections and eligibility at its own developmental replay
rate, adding bounded `replay_rate * alpha * replay_credit` to staged fast
weights. It does not apply this waking Oja expression.

Consolidation separately promotes `staging_rate * replayed_fast` into lifetime
weights (clamped to the lifetime limit), then multiplies fast by
`1-fast_decay_rate`. Defaults are .5, 4, and .5. Disabled inherited receptor
rows are preserved. This promotion operates in raw fast-coefficient space,
so effective strength is not generally conserved when alpha differs from 1.
No sleep formula is changed here.

The old cancellation coefficient can therefore become actual lifetime change
at sleep. For `G=.5,L=.25,F=-2,a=.25`, waking effective strength is .25;
without replay, one default consolidation gives `L=-.75,F=-1,E=-.5`.
This illustrates why retaining the raw immutable payload does not by itself
protect expressed or subsequently consolidated knowledge. The fix prevents
new zero-credit cancellation; it does not attempt to infer or reverse earlier
learning. Accepted backups, learned payloads, and optimizer history stay intact.

## Validation and remaining evidence

`oja_credit_stability` provides full production-module Naga parsing/validation,
evaluator/helper wiring checks, independent closed-form clamped oracles,
compiled default route inventories, profile projections, and an explicitly
invoked isolated device test. The device test extracts the exact production
helper text, executes it in WGSL, and runs the original formula as a paired
control. It covers 181 cases, 1/1k/10k/100k outcomes, both signs, small credit,
fixed routes, alpha 0/.125/.25/1, zero post, maximum contraction, and both fast
clipping endpoints. One-step tolerance is 2e-7 effective units; long f32
recurrences use 8e-4, with exact zero-credit retention asserted separately.
The final executed maximum absolute oracle error was 0.0001192092764,
dominated by the old alpha-1 cancellation trajectory's f32 stagnation near
zero. All six focused tests passed, including the explicitly invoked device
test. The readback loop uses `as_chunks::<2>()` to satisfy current strict
Clippy; this does not alter the production shader or any test vector.

```sh
cargo test --locked -p alife_gpu_backend --test oja_credit_stability -- --nocapture
# An isolated arithmetic test may explicitly use software Vulkan:
VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json \
  cargo test --locked -p alife_gpu_backend --test oja_credit_stability -- --ignored --nocapture
cargo test --locked -p alife_gpu_backend --features gpu-tests \
  --test closed_loop_fast_plasticity --no-run
```

The new `zero_projected_credit_commits_without_canceling_inherited_strength`
integration test uses nonzero rates, insensitive receptors, real measured
nutrition/sealed experiences, four normal transaction commits, unchanged
fast/lifetime arrays, and cleared pending eligibility. It compiles. Execution
in this saved cloud environment is **Blocked**: production returns
`NeuralBackendUnavailable` because there is no supported physical GPU. The
isolated device result identifies `SwiftShader Device (Subzero)`, Vulkan, CPU;
it is not counted as a hardware production acceptance run. No PC run was used.

Aggregate validation completed in the saved cloud environment:

* `cargo fmt --all -- --check`: passed.
* `cargo check --workspace --all-targets`: passed.
* `cargo test --locked --workspace --all-targets --no-fail-fast`: passed,
  1,941 tests across 216 completed targets, zero failures, 23 ignored.
* Final focused `oja_credit_stability --include-ignored`: six passed,
  including all 181 paired production/control WGSL vectors.
* `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed
  after the test-only chunk API lint fix; the focused suite was rerun after
  that edit. The broad workspace test run was not duplicated for this edit.
* `scripts/check_core_boundaries.sh`: passed.
* `scripts/docs_check.sh`: passed, 77/77 assertions.
* `git diff --check`: passed.

The first `scripts/check.sh --full` invocation passed formatting and workspace
compilation, then exhausted the cloud disk while linking test binaries with
debug symbols. Only compiler artifacts created during this task under
`target/debug` were removed. The remaining gates used the saved cloud compact
settings (`CARGO_INCREMENTAL=0`, dev/test debug symbols 0, two build jobs,
`target/cloud-cpu`, one test thread). Debug assertions stayed enabled.
No source, checkpoint, learned payload, or optimizer state was removed.
The implementation source commit is `2da160fe2d82a70545d96537cad1c22308cffd4c`;
the evidence/lint follow-up leaves the production shader identical to it.

Gameplay retention/behavior improvement on an accepted trained founder remains Unknown;
the demonstrated defect and its arithmetic correction do not require a perfect
founder or establish global learning failure.

Architecture trace: AOA-LEARN-001/006/007 (fast signed local learning),
AOA-LEARN-002/004 (unchanged causal sealed credit), AOA-LEARN-005 (distinct
learning/normalization rates and sleep schedule), AOA-FOUND-006 (the ordinary
production rule is fixed), AOA-SLEEP-002/003/004 (unchanged consolidation,
transaction, germline boundaries), AOA-PERSIST-001 (no acquired-state reset).
