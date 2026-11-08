# Detached multi-drive routing assay V1

This is a new source-only/fixed-context experiment, not a rollout or alteration of earlier weight-space evidence. No private life, acquired checkpoint, world step, learning step or GPU/device execution is involved. The production CPU replay operator executes the compiled graph, inherited activation dynamics, route cadence and inherited dendrites. Default ABI and existing assets are unchanged.

## Construction and intervention

Base source: accepted `10eecff46329d74405a8a8ab25f0e4a4c5fb60b1`. Three explicit arms use seed 7: canonical default N2048, experimental repaired graph with its genetic weights, and the repaired graph with frozen whole-care6 weights. This separates graph repair from the existing engineered weight intervention.

`examples/support/multidrive_frozen.rs` copies only construction functions from `examples/probe_weight_space_care.rs`, not rollout functions. The original file SHA-256 is `8c4a7ccd988895513c8b16beac7d68a298f969e1f281cf27d5d5989de1624cba`. Exact constructor text from `fn whole_care(` through before `struct TrialSpec` has SHA-256 `2c0c0c0478dc120cad1a232da2f7d35dd67d92a750b85428ae083fccc3d664bb`; the copy differs only in making the entry point public. Earlier constructor, asset and receipt files remain untouched. Result JSON binds base compiled asset and evaluated weight-vector digests separately: restoring a fresh experiment's weights does not rename the base asset as whole-care6.

The assay reports actual expressed canonical newborn drive receptors and emitters. Drive 9 is boredom under this binding; hormone 9 is distinct praise. Extension1 has no canonical receptor and is probed as an artificial electrical input, not an unmet biological need. Curiosity and reproductive readiness are not automatically welfare deficits. ATP is availability, so the perturbation decreases it.

Each synthetic context runs 32 baseline frames and 32 sustained-perturbation frames. Perturbations comprise the unchanged baseline, all 11 lanes separately, and hunger+fatigue, hunger+pain and fatigue+boredom conflicts. Inputs use inherited homeostatic encoder scales/biases/clamps. Other sensory and hormone inputs are zero, receptor gains are 1, threshold shift is zero, and every compiled microstep is executed. This intentionally does not execute actual ATP/fatigue-dependent biological scheduling. There is no biochemical feedback, body energy debit, acquired offset, structural growth or biological advancement. Consequently these results characterize neural routing under fixed conditions, not integrated behavior or cost.

All eight candidate families remain present, including Idle and Rest. Their intrinsic feature vectors use the production helper, including the Rest posture lane. Three explicit feature fixtures cover nutritious contact, aversive contact and distant bearing. They are decoder probes, not claims that arbitrary fixtures are legal, reachable meals or validated world actions. Innate scores are reported separately from recurrent neural scores, along with their sum. No best-action quota or forced activity objective is applied.

## Measurements and controls

- Structural directed reachability, nonzero evaluated-weight recurrent reachability, and nonzero reachability including inherited dendrites are separate outputs. Dendritic threshold gating means a nonzero path is still not evidence of effective transmission.
- Full family logits, baseline logits, signed changes, individual motor absolute activations/baselines/deltas and direct innate contributions are recorded at perturbation frames 1, 2, 4, 8, 16 and 32. Decoder feature sign, motor sign and weight all matter.
- Every input-cut control injects baseline input and must equal the baseline replay sequence exactly, permitting the same deterministic baseline evaluation to be reused. The preliminary run additionally evaluated every cut and confirmed exact logits. Every arm checks that weights and all optimizer fields remain byte-identical.
- The same inherited dendrites are used in each arm's baseline and perturbations. Repaired genetic and whole-care6 use the same graph and dynamics.
- No learned-policy, GPU, terrain/heard-language qualification or multi-seed generalization claim is made.

Run: `cargo run --locked --offline -p alife_training --features cpu-backend --example probe_multidrive_routing`. Build/run receipts and findings are recorded separately after execution.
