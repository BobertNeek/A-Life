# Thermal and reproductive drive audit

## Scope and source

Read-only production audit of ordinary main b0b4e5ab85473f1ace63f0629ecf4704e7b0fb81, represented locally by commit 7a15a2f8ceffb871947ca7df819095d2cab99327 with identical tree 0a34d588488c09f9a7fd4d8bc6115be61bdcd750. Separate disposable worktree. No learned checkpoint, private bank, save migration, or reset read or changed. The only added code is a fresh-state core integration fixture. No production weight or physiology change is proposed for immediate promotion.

## Findings

### Temperature: measured input is present, but ordinary exposure physics is missing

* `alife_world/src/grounded_sensing.rs:60-98,250-269`: surface temperature is a dimensionless signed [-1,1] descriptor. It is pseudorandom by spawn sequence, including ordinary food, agents, and toys. Object-slot temperature is multiplied by physical contact before encoding.
* `alife_world/src/headless.rs:4080-4113`: reachable nearby contacts expose hot and cold separately through tactile lanes 3 and 4. There is no distant refuge-temperature field. Object temperature is not emitted as a thermal body event anywhere in ordinary world physics; production occurrences are perception, hashing, validation, and event combination.
* `alife_core/src/biochemistry.rs:153-170,394-443`: supplied BodyEventDelta.temperature_stress is unsigned [0,1]. Body load increases by dose*(1-temperature_tolerance), decreases by sleep_recovery*metabolic_turnover*0.20, and clamps to [0,1]. Rest has a real recovery route; a cooler location does not currently have an independent thermal-relief route.
* `alife_core/src/biochemical_graph.rs:1139-1140,1804`: body thermal stress emits TEMPERATURE at gain 0.30 and CORTISOL at gain 0.20; the receptor projects the temperature drive. These are biochemical gains, not directly comparable to decoder logit weights.
* `alife_core/src/phenotype/io_compile.rs:661-714` and `phenotype/decoder.rs:84-131`: temperature is part of innate DANGER. Avoid gain is 24*reflex_strength*hazard_aversion above urgency 0.55. Idle/Inspect/Contact/Other receive -8*reflex_strength above the same threshold. Avoid is bound to adverse contact chemistry, or noncontact bearing fallback, not hot/cold polarity. Rest's innate route is fatigue only. This supports nonspecific alarm; it does not implement thermoregulation or a justified safe-temperature target.
* Default Nano inventory supplied by the primary drive audit: thermal lane7 reaches neuron66; no compiled recurrent route reaches candidate readout. This audit does not claim a new native neural test of that already-inventoried route.

**Conclusion:** A larger thermal action weight cannot create missing world exposure or refuge information. The existing unsigned drive cannot tell whether moving left, moving right, approaching a cool surface, or approaching warmth would help. Avoiding an arbitrary visual object while hot is not evidence of thermoregulation.

**Separate future world factor:** define a neutral surface baseline, explicit harmful hot/cold threshold, interval-scaled contact dose, tolerance/insulation behavior, and passive recovery outside exposure. Do not map abs(current random descriptor) directly into harm. Keep body events unsigned unless a separately versioned physiology representation requires polarity; observations can retain signed physical temperature. Cover neutral/hot/cold/contact/no-contact/occlusion/tolerance/relocation, simultaneous action and world interval dose, atomic biology commit, and save roundtrip. No hidden refuge choice or semantic target oracle is needed. AOA-BODY-001/005 supports this physical pathway, but does not specify numeric temperature units or a threshold.

### Reproduction: physiology is wired, but it is not an explicit motivated mate action

* `alife_world/src/headless.rs:1775-1825,2113-2272`: eligible nearby compatible organisms in an authorized habitat generate measured mating opportunity using phenotype mate response and the same pre-update biology snapshot. Solids block mating reach. Habitat membership, foundation compatibility, and brain class compatibility remain real constraints.
* `alife_core/src/biochemical_graph.rs:1141-1145,1808`: mating opportunity emits reproductive chemistry at gain 0.85. The homeostatic reproductive receptor is wired. Emitter strength is scaled by developmental biochemical expression (`biochemical_graph.rs:1418`); puberty independently gates readiness. The fresh age-four fixture has zero projected reproductive drive, not an adult-scale motive.
* `alife_core/src/biochemistry.rs:603-648,949-970`: readiness requires puberty, health>=0.55, injury<=0.45, body energy>=0.35, brain ATP>=0.25, reproductive drive>=the phenotype threshold, developmental hormone>=0.20, opportunity>=0.50, and fertility>0. Cached readiness is rechecked against current biology, age and bounded cadence freshness.
* `alife_world/src/headless.rs:1940-1975,2068-2094`: births require the reproduction-cadence boundary, a fertility trial, both live parents ready, and valid placement. Birth transfers real parent reserves into the newborn. There is no explicit Mate action and no post-birth reproductive-chemical satiation/refractory pulse. Reserve costs can subsequently block readiness; they are not hormonal satiation.
* Default Nano inventory supplied by primary drive audit: reproductive lane8 reaches neuron72 but has no compiled recurrent route to candidate readout; no innate reproductive mask exists. This audit does not rerun the earlier neural inventory.

**Conclusion:** There is a functioning proximity-triggered reproductive physiology path, not a verified end-to-end reproductive motivation policy. Raising a reproductive weight without a grounded mate-related action/cue and measured consequence would invent behavior. AOA-REP-001 explicitly requires genotype, physiological readiness, world interaction, and lifecycle authority; proximity and physical eligibility satisfy the implemented interaction model. No controlling requirement mandates a mating command, gestation, or refractory interval, so their absence alone does not justify silently altering ordinary reproduction here. Satiation would be a separate biological design factor, not a neural-weight patch.

## Fresh bounded verification

`crates/alife_core/tests/thermal_reproductive_drive_audit.rs` has two disposable source-to-physiology tests:

1. A supplied thermal exposure increases body load and projected drive versus matched control; a recovery event reduces body thermal load versus matched passive continuation.
2. Juvenile opportunity cannot reproduce; a fresh adult encounter can; current ATP 0.249 blocks readiness and 0.25 admits it while other conditions are unchanged.

These test actual core implementation, not a copied formula. They do not prove ordinary-world thermal exposure, native GPU behavior, autonomous mate-seeking, reproductive satisfaction, or full-suite compatibility. The ATP boundary fixture deliberately changes only its disposable in-memory snapshot to test readiness; it is not a production state mutation.

### Results

Both fresh tests passed (2 passed, 0 failed). Thermal body load was 0.495 without recovery versus 0.29500002 with recovery; projected thermal drive was 1.0 versus 0.94386005 on the next step. Adult reproductive drive was 0.95000005, ATP 0.64920074; juvenile drive was 0.0. Build used Rust 1.99.1, offline Cargo, one job, no incremental compilation, debug symbols disabled, and a distinct target. Log: `../thermal-reproductive-test-v1.log`. Native core fixtures only; no GPU/ordinary-world closed loop is claimed.
