# Detached native matched motor microcircuit, version 1

This is an intermediate component, not an admitted A-Life brain, a world experiment, a default-policy change, or proof of learned behavior. It cannot be attached to an existing creature. It never calls the world, action sampler, optimizer, or ordinary CPU/GPU brain executor.

## Why this step

The previous whole-care motor neurons had different, sometimes negative operating points. A hand-chosen reference coefficient could reverse a supposedly positive drive contribution. This component tests an alternative: matched drive/reference afferents, rectified pressure pairs, recurrent recovery/search/fear controllers, then distinct motor-execution pairs. Every new readout uses an execution pair rather than reading a controller directly.

The branch and readout lists are an exact lowering of the archived M0 proposal. Logical addresses 2048–2143 become local indices 0–95. There are 70 branches, 184 branch input products, 22 afferent assignments, 50 readout coefficients and 96 allocated cells, of which 92 have a functional connection. It is new-path mediation only. Existing action-proposal, innate, episodic and other channels are not included or suppressed here.

Reference: [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), especially experiment 4 targeted repair, experiment 5 whole-model construction, and experiment 6 iterative feedback. Those game controllers motivate the method; they do not establish this organism's competence.

## Actual implementation

`lib.rs` is a Rust component with private fixed parameters and explicit state. One call executes one synchronous previous-state microstep. It uses leaky tanh cells and the same expression order as the inspected CPU dendrite update: rectified weighted previous activation, tanh branch response, projection gain, local threshold, then leaky cell response. It has not been compared through the actual compiled CPU sparse-graph executor.

Both each sensed drive and its declared setpoint receive the same interoceptive gain, regional gain and afferent offset. New cell pairs have tanh activation, leak 0.5, bias zero and no metabolic-feedback subtraction. Metabolic load and EMA still update, with explicitly authored decay constants 0.99 and 0.98. These constants and the proposed setpoints are design choices, not experimentally optimized biology. ATP's setpoint is 0.5 in this frozen version. High ATP is not treated as a deficit. Unbound extension 1 is carried through a pressure pair but has no behavior-bearing downstream edge.

Parameters are fixed only inside this detached component. This does not establish that integrated A-Life somatic, replay, structural, consolidation or optimizer writers would preserve them. Actual integration needs its own versioned identity, capacity, matched encoder/dynamics validation, pricing, plasticity rules, admission and checkpoint qualification. Existing phenotype validators were not weakened.

The work counters count algorithmic operations, not elapsed time, ATP or GPU costs. Equal-work controls execute the same branch and readout loops and count the same operations. They are not a wall-clock performance comparison. No hidden settling steps are run.

## Executed qualification

Rust 1.99.1, standalone `rustc --edition=2021 --test`, no external dependencies.

The first compilation failed because generated `plan.rs` lacked its parent type imports. The correction added only `use super::{Branch, Readout};`. Failed source copies were preserved locally; the original complete compiler log was not captured to a file. No test ran on that source.

The first corrected executable SHA-256 was `bd149109e612f49b0246c62f66811f5bbde93d00cca92588383d563229c3150c`. Its 10 fixed tests passed in 0.23 seconds. The final source was then formatted and gained an explicit recovery-latency test; its 11 tests passed with no ignored tests in 0.15 seconds. The final test executable SHA-256 is `6ac62f646baf0cc43fd1b20f57ff6e6263e52865f79bd1035628fafd3c7584df`. The final library, test and scalar-probe builds all pass `rustc -D warnings`. Durations are observations, not performance benchmarks.

The tests cover:

- Exact branch/input/readout counts and execution-pair-only emission.
- Nonnegative pressure signs for all 11 drives across three equal initial operating points, five projection gains and four thresholds, with 24 microsteps per case. Positive gain must produce a nonzero driven differential.
- Zero added behavior at setpoints despite nonzero common cell activity and metabolic tracking.
- Explicit motor-stage latency: hunger-to-Ingest emission is zero for the first two microsteps and positive after the third. Fatigue-to-Rest emission is zero through the third microstep and positive after the fourth. A motor-bridge lesion removes the new emission while upstream need/controller activity remains.
- Positive versus negative observed chemistry effects, without equating taste with actual nutrition or harm.
- Fatigue/low-ATP recovery, object-directed movement suppression, no-op neutrality and no direct suppression of eating in place.
- Unbound-drive behavioral null and sensed-peer approach/vocal effects without a social-grab permission.
- Washout, matched counted-work sham, and an intentionally mismatched-reference control that produces a spurious low-ATP pressure.
- Atomic rejection of NaN input and exact cloned-state continuation. This is cloning, not save/load or cold restore.

No native world tick, creature life, training update, retained state, hardware GPU, dynamic ordinary-physics test or success claim is involved. `lower_plan.py` independently regenerates every branch/readout literal from the frozen recipe digest and verifies exact lowered text; this source check passed. An independent reviewer has not rechecked it. The actual compiled-graph candidate is a subsequent engineering task.

## Fixed native conflict probe

`probe.rs` executes eight declared constant-input contexts, each for exactly 96 previous-state microsteps. It prints only scalar controller differentials and added candidate logits at microsteps 1, 2, 3, 4, 6, 12, 24, 48 and 96. Thus `probe-v1.csv` has 72 rows from 768 executed component microsteps. There is no body, chemistry feedback, sampling, learned memory or reward. The 96-step endpoint is a long fixed-input probe, not one ordinary decision frame.

At microstep 96 with ordinary proposed projection gain 0.5 and threshold 0.25:

| Context | Added Rest | Added object-Approach | Added positive-chemistry Ingest |
| --- | ---: | ---: | ---: |
| All setpoints | 0 | 0 | 0 |
| Hunger 0.9 | -0.424020648 | 0.353350520 | 1.872423172 |
| Fatigue 0.9 | 1.526806593 | -1.526806474 | 0 |
| Hunger and fatigue 0.9 | 0.304960847 | -0.358177662 | 1.872423172 |
| ATP availability 0.1 | 1.334259510 | -1.334259510 | 0 |
| ATP availability 0.9 | 0 | 0 | 0 |
| Fear and fatigue 0.9 | -0.856909752 | -1.449739933 | 0 |
| Hunger 0.9, ATP availability 0.1 | 0.185030818 | -0.202594280 | 1.872423172 |

Unlisted drives remain at declared setpoints. Object and chemistry readouts use separate single-feature diagnostic vectors, not a claim that these describe a valid finalized perception frame. The conflict result is consistent with stationary recovery while retaining appetite for food in reach. It does not prove that a sampled joint motor bundle will be coherent, that a creature reaches or eats food, or that these provisional coefficients are optimal. No perpetual-activity oscillator is imposed; at setpoints the added outputs are zero, and washout is tested. Ordinary drive regeneration/relief must later close the biological loop.

Probe executable SHA-256: `b44c3640742b10bd8ab7bd8225ea33ef2264068d94a15404d5a9e3d5558bd44e`. Scalar CSV SHA-256: `37b67cd1c6ae2d3197a3a7334a165d8ca1d5d8924d817322c50a93c5ff7ca117`. The library SHA-256 is recorded in the manifest, not uploaded as a deliverable.
