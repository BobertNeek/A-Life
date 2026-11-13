# Detached typed decision-context primitives

Enable `alife_training/experimental-decision-context` explicitly. This CPU-only
library is a first, deliberately detached integration of the retained-memory and
current-self numerical kernels. It does not alter the live runtime, ordinary CPU
replay, New Game, any creature checkpoint, production decoder lanes, or defaults.
It does not admit a body candidate or provide GPU execution. A caller cannot use
these functions as evidence of full runtime integration or recurrent BPTT.

## Semantics

`RetainedFrame1992` carries remembered-source provenance, confidence, age and
nominal candidate-own effects, separately from the replay tick's 54 current and
existing context inputs. `CurrentSelfFrame2000` carries typed Hold intention,
not a contact feature or an observed-object Look. Both bind the whole replay
tick by digest. They do not modify sensory inputs. Detached binding validates
internal consistency, not the truth of a source claim: collector provenance and
the actual post-neural endpoint still need independent binding before live use.

The retained 48-parameter head reads eight explicitly named Reserve endpoints,
forms six own-effect/confidence/freshness features, and bounds its contribution
with tanh. The current-self scalar reads endpoint 1791 only for typed Hold. This
neuron is **not intrinsically a forward-intention sensor**. `fresh_zero` is the
only convenience initializer here; copying the old Forward coefficient 192
produced near-tonic Hold and degraded the tested wall behavior.

Exact-zero contributions preserve the base score's bits, including negative
zero. Local gradients differentiate the underlying addition even at coefficient
zero. They return head-parameter and source-endpoint adjoints only; callers must
not describe them as gradients through the full recurrent graph. Head Adam state
is separate and updates transactionally. No old optimizer or save is migrated.
Existing experimental head/frame serialization identities and arithmetic are
retained; the new detached adapter does not restore a full experimental host.

`DecisionContextWorkV1` reports kernel logical work, including zero-coefficient
work. It does not debit ATP. A future host must meter both heads, feature work,
bounds and merges under the established combined-rounding policy before
admission. The adapter's validation/hash overhead is not a hardware work metric.

## Provenance and limits

Extracted from frozen `current-self-hold-native-2000/handoff-v1`, handoff SHA-256
`ea5d43b28e520c6ee7443957459a99d65bbb6cbc8e42cd82bd1e9f680b99cdd8`.
The later body-control handoff is
`a4f1b4964dc5c9353232703e5b50b365b4dcad9fdee5773c472de29f45077ab5`.
These are provenance hashes, not dependencies on private workspace files.
The integration uses current main's Adam implementation, adds checked detached
adapters and focused tests, and deliberately omits the experimental collector,
asset construction, full replay/host checkpoints, and body intervention chain.

Historical CPU pilots motivate caution, not candidate promotion: in eight wall
lives the zero-self arm ate 8/8, legacy 4/8, and copied Hold192 0/8. A separate
fresh three-arm, zero-self cohort ate 8/8 with retained head active versus 7/8
with head zero or source masked, but active memory delayed six of seven mutual
successes. Mirrored lives are not independent samples. This is neither a full
factorial nor general navigation/reliability/efficiency evidence. Neutral and
bitter fixture nonconsumption does not establish general diet competence.

The stored directional circuit is a coarse world ray, not a translated finite
location. Translation, quantization, settling, joint-action ordering and
postmeal residual influence remain research questions. This patch does not
adopt the unqualified location-transport prototype or a new physical behavior.

## Checks

```
cargo test -p alife_training --features experimental-decision-context --lib decision_context
```

Tests cover zero/off semantics, typed negative controls, all 48 parameter and
eight endpoint finite differences, malformed input rejection, atomic Adam
failure, head cold round-trips, bounds and logical work. Full native replay,
GPU, live body and full-host compatibility are outside this detached layer.


## Detached command-conditional joint numerical primitives

`JointPolicy2054` is an explicit new two-channel policy kernel. It accepts a validated base vector, disjoint nonempty locomotion/orientation masks, forced-slot metadata and a rectangular pair-potential table. It does not install a production sampler, change motor-channel count, or authenticate live observations. Public candidate/category/pair indices are checked. `JointSample2054` separates the observable representative and selected locomotion/orientation from the diagnostic discarded base-global index. Likelihood credits the marginalized motor category and selected pair, never the discarded index.

At fixed base, base-global f32 operation ordering and every Other representative are independent of pair potential. Pair probabilities and analytic logprob/entropy adjoints use stable f64 math. Pair normalization centers contrasts of the original base and potential terms before temperature scaling; an error-free sum expansion preserves small residuals across large finite common offsets and cancellation. The expansion collapse uses f64 arithmetic and can differ from the correctly rounded exact contrast by an ulp. This can correct pair probabilities and last-bit rounding without changing the base-only global draw. These are ideal normalized probabilities, not exact counts of a finite uniform-word grid. Zero pair potential does not restore the old motor-tuple tape. Stop/Hold remain ordinary candidates; no null calibration or physical safety is implied.

`JointHead2056` supplies a detached bounded hidden feature followed by a signed output amplitude, with3 signed effect coefficients and separately aged4-coordinate transactional Adam. Inputs are positional context4 and nominal effects6 (translation, body delta, gaze delta), not raw vision or privileged future coordinates. Caller-supplied values are not collector-authenticated. The private tape is bound to the exact forward parameter bits. Zero amplitude leaves the amplitude gradient live; updates do not touch existing heads or learned state. Standalone cold bytes are not a host checkpoint migration.

Both remain under the existing default-off `experimental-decision-context` feature. No host/collector/PPO/World/GPU route, model-binding revision, ATP/work admission, or full recurrent training is provided. Physical outcomes, learned desirability and null calibration require separate qualification. No existing candidate or trained life is promoted.
