# Native Nano operating point, diagnostic only

Based on frozen V3 commit `27c01b6f258604a307c5c359ad41c7e91ae3ae0b` and closed local fixture artifact SHA256 `6f0e21260fec2885bee195cd32b9b7955579fd96e4fd68a49106aa7e27e98a92`.

## What was verified

- All 384 saved frames: consecutive ticks, actual before/after biology validated against the same seed-41 inherited phenotype, matching genome/source and frame homeostasis, exact compiled source/phenotype/input digests, and recorded dendrites matching birth branches.
- Shared native input encoder extraction: two tests passed, including bitwise equivalence to the frozen pre-extraction implementation under native and non-neutral receptor effects. The full frame encoder uses the helper unchanged.
- No new world interval, action, host/cohort, neural replay, optimizer update, or private-state restoration. Deterministic birth-record construction was used only to recover expressed chemistry identity.

## Mechanism

At the last frame, native interoceptive and regional gains are both 0.5, projection gain is 0.5, and local threshold subtraction is 0.25. The global encoded offset is -0.025 for the toy fixture and -0.0247334 for the peer fixture. The peer's small difference comes from the recorded serotonin/consolidation receptor effect, not an assumed neutral context.

All eleven drive-source neurons (64, 67, 68, 65, 69, 70, 71, 66, 72, 73, 74) have zero compiled recurrent inputs and zero recorded dendritic inputs. Their bias is zero. With these recorded effects, the preactivation upper bound before nonnegative metabolic subtraction is:

- Toy: 0.25 * pressure - 0.275.
- Peer: 0.25 * pressure - 0.2747334.

At raw pressures 0, 0.1, 0.8, and 1, all bounds remain negative. Across the actual 384 rows, the largest corresponding drive-source base was -0.075. Thus the positive-input first-hop branches cannot respond from the documented fresh zero neural state in these fixed contexts. This is a sign/operating-point failure shared by the eleven new relay inputs, not evidence that the drives lack biological meaning or all other drive paths are absent.

The loneliness path is source 69 -> reserve 500 -> reader 344. Reserve 500 also retains an older recurrent input from source 64 (hunger), genetic weight +0.0676881224, alpha 0.25, Hot10To30Hz cadence. Its acquired weight and instantaneous activation were not captured, so this diagnostic does not invent them or claim the reserve's complete current is known. The new first-hop branch contribution is bounded at zero under the conditions above. Dendritic branches evaluate every executed microstep; the recurrent route has its own cadence.

Reader 344 has no compiled recurrent input. Its new +0.25 bias cancels the +0.25 local threshold, but not the negative encoded offset or metabolic subtraction. Assuming its incoming positive branch is closed, the local equilibrium solves a = tanh(base - a²). This predicts:

- New head controls, toy: -0.0256524235; saved long detached probe: -0.0256524198.
- New head controls, peer: -0.0253716707; saved long detached probe: -0.0253716651.
- Unused original reader reference, toy: -0.430009127; saved long probe: -0.430008590.
- Unused original reader reference, peer: -0.429282844; saved long probe: -0.429282367.

The new-head comparisons differ by at most 5.59e-9. These are analytical consistency checks against saved detached probes, not reconstructed actual neural state. The negative reader closes max(source, 0), explaining the absent new memory-port contribution.

## Boundaries and correction

The prior V3 pressure probes changed nominal raw input contributions without applying native interoceptive/regional gains. They must be labeled nominal raw-input probes, not exact re-encoding. Here, pressure substitutions run the shared native encoder with the recorded before-chemistry effects held fixed. They are input-only counterfactuals, not new physiological trajectories.

The artifact lacks neural initial activations, metabolic loads, somatic offsets, enabled-route flags and structural edges. None were reconstructed or defaulted. No learned memory was manufactured. No need-gain change, V4 candidate or promotion was made.

Compact scalar results: `operating-point-summary.json`. Detailed local-only derived output and build logs remain outside the source commit. The original raw receipt remains local. The first test compile failed only on test-fixture borrow declarations; that log is retained, the declarations were corrected, and the two tests passed on rerun.
