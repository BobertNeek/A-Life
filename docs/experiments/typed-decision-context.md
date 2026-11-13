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
