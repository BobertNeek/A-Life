# Opt-in contact thermoregulation research factor

This is an unpromoted physical-world factor, not a founder policy or an ordinary-default bug fix. Source base is ordinary main tree 0a34d588488c09f9a7fd4d8bc6115be61bdcd750. All fixtures create fresh disposable organisms. No bank, checkpoint, acquired state, or autonomous life is read or run.

## Exact model

A caller explicitly supplies a map from surface entity identity to conductance in [0,1]. Identity binds a physical surface; no object-kind, food/hazard/social label, desired action, refuge coordinate, or reward is consulted. Ordinary world methods have no thermal configuration. Consequently existing random signed temperature descriptors do not become hazards by default.

Signed surface descriptors are dimensionless, not Celsius. The inclusive neutral range is [-0.25,+0.25]. Outside it severity is (abs(T)-0.25)/0.75, clamped to [0,1]. For each physically reachable touching surface multiply severity by configured conductance; take the maximum across contacts and action endpoints. Hot and cold therefore neither cancel nor add multiple full-body doses. Conductance zero represents an insulating contact material. There is no redundant actor-insulation field; inherited temperature tolerance already attenuates physiological response.

At maximal contact, exposure is 0.10 units/second before inherited tolerance. In a neutral/no-harmful-contact interval, passive thermal-load removal is 0.05 units/second. These are explicit conservative research timescales: ten seconds to unit load before tolerance, twenty seconds to remove unit load. They are not empirical thermodynamic calibration. Conductance attenuates external dose, not internal regulation. Fully insulating contact permits passive relief. Existing sleep-mediated relief remains in addition, but the new passive relief does not emit SleepRecovery or give ATP/fatigue benefits.

WORLD_TICKS_PER_SECOND is 20, so each admitted biological interval contributes at most 0.005 exposure and 0.0025 passive relief. Readiness admits current world tick+1, regardless of a command's duration field. No command duration multiplies dose.

## Contact geometry and pathways

Contact uses existing tactile reach: distance to surface center <=max(object radius, HEADLESS_CONTACT_RADIUS), excluding self/consumed objects. The existing solid-segment test rejects blocking object obstacles and terrain AABBs while ignoring the contacted surface itself. This is abstract point/center/radius contact geometry, not mesh heat transfer, contact-area integration, or a temperature field.

Three authoritative interval entry points share one event composition:

1. Staged world tick: stationary organisms at current tick receive one contact sample and one biology advance.
2. Registered single action: sample initial/final physical positions, combine max exposure with the action event, advance once.
3. Joint motor bundle, including neural emission: sample initial/final positions and combine once after all channel events.

The following world tick skips an organism whose biology already reached next tick, preserving existing once-only admission. The prototype inherits endpoint contact approximation; a surface crossed solely between endpoints is not integrated. This limitation is explicit rather than presented as swept/mesh thermal contact.

Positive exposure travels through existing unsigned BodyEventDelta.temperature_stress and inherited tolerance. Passive relief is a distinct explicitly named core advance input, applied inside BodyState integration before biochemical emitters/receptors. It is not encoded as negative temperature stress, sleep, nutrients, or reward. Existing entry points always supply zero relief.

## Transaction and persistence boundary

`HeadlessWorld::with_thermal_contact_candidate(parameters, operation)` clones the world, installs ephemeral configuration, runs the operation, clears configuration, and commits only on success. Nested candidate scopes fail. Existing record-level and action-level rollback remains in force. Parameter validation rejects NaN/out-of-range dose/conductance and missing surface IDs before the operation.

Configuration is not serialized or hashed into ordinary assets. Existing world-state serialization and biochemistry layout remain unchanged. A caller must explicitly re-supply identical external parameters after save/load. This is a research API contract, not a persistent material ABI; ordinary continued play after a load remains ordinary. A future promotion must decide persistent configuration/provenance separately.

## Evidence protocol

Before execution, source and fixture hashes were pinned in `../thermal-contact-candidate-pretest-v2.sha256` (v1 records a pre-compilation signature fix). The fixtures use deterministic world/genome seed 71512026 and source-visible operation closures. Focused tests cover neutral boundaries and default bytes, hot/cold symmetry, inherited tolerance scaling, insulating conductance, no-contact, obstruction, overlap max, action/joint/stationary one-dose equivalence including duration 99, relocation recovery without ATP/fatigue benefit, same-parameter save continuation, invalid parameters, and late rollback. Native core/world tests only; no GPU or autonomous ordinary-world closed loop.

### Expanded controls and prerepair outcomes

Expanded fixtures add genetically expressed tolerance 0/0.5/1, mature chemical projection, actual locomotion leaving the contact radius, the joint neural-emission branch, stationary late-failure injection, signed-zero passthrough, and one four-tick wrapper versus four one-tick wrappers. Zero relief follows the literal legacy arithmetic branch; zero exposure leaves the existing event bits untouched.

The first expanded test incorrectly required whole BiochemistryState equality between an empty neural frame and no neural frame. Both produced equal body/homeostasis but different legitimate neural-emitter work counts (6 versus 0). The test was corrected to assert equal physiology and preserve the work-count distinction. This was a fixture assertion repair, not a physiology change. Compile-only corrections are retained in earlier logs: entity IDs require validated raw numeric map keys; restore needs explicit inactive candidate configuration; biochemical work uses its public accessor. Each revised source was hashed before its run. Final evidence must identify the final source manifest and completed checks rather than treating earlier partial runs as final.

# V2 addendum: continued exposure and sleep are separate effects

This worktree is a distinct, unpromoted V2 factor. V1 source/rates/results are retained separately. V2 changes no temperature threshold, conductance rule, exposure rate, passive-relief rate, ordinary default, neural weight, or motion choice.

The core receives an explicit nonpersistent ThermalBalanceCandidate with passive_relief and suppress_sleep_thermal_relief. Ordinary entry points and the V1 relief API keep false/zero legacy behavior. Only a positive dose from explicitly configured external contact sets the V2 suppression flag. Generic externally supplied BodyEventDelta temperature, command type, object label, or desired action does not set it.

For that continuing-exposure interval only, the body thermal expression omits the inherited sleep-cooling subtraction. This is a separate expression branch, not compensating extra exposure, a second body mutation, or reordered default arithmetic. Sleep still supplies its original fatigue/ATP chemistry, actual sleeping state, repair input, and metabolic behavior. When no positive configured dose is present, ordinary sleep cooling remains; neutral/clear intervals also retain measured passive relief. The departure interval uses the same max-endpoint convention; the next clear interval cools. This is an abstract dimensionless thermal-load model, not realistic thermoregulation or a refuge policy.

Matched V2 controls pre-expose fresh mature bodies for ten intervals, then compare ongoing hot/cold Idle and Rest, neutral-contact Rest, and out-of-contact Rest. They check retained thermal load during ongoing exposure, preserved rest ATP/fatigue and physical sleeping, identical reserve/repair/health/injury, reproductive readiness and biochemical work counts, and exact continuation after portable save with identical external parameters. A zero configured-dose control checks that unrelated legacy thermal events cannot suppress ordinary sleep cooling. Existing V1 geometry/admission/rollback/signed-zero controls remain, with the new explicit balance type. No autonomous cohort is run.
