# V6 GPU producer integration (frozen qualification)

Source: `/workspace/scratch/ee5e08a3ca9b/motor-reflex-gpu-v6`, based frozen body-only v5 `8d43b8d2758ba4d923fc38195221c6e48e3d59f2`. Frozen commit `68eaeaed2b9e4f5c07f3fb129f6ddd4afd417913`, tree `e11f9b012363f77f31d581364e7c2f3bb56cab74`. Not published or default-enabled.

## Changes
- GPU world producer chooses explicit contact candidate only when requested; ordinary producer remains unchanged. Real withdrawal becomes separate V13 evidence, preserving voluntary channel receipts and joint physical accounting.
- Test-only per-call runtime option restores previous mode after success/failure; no checkpoint configuration/schema changes. Body-only responses are transient diagnostics, included in staged rollback snapshots, not fabricated voluntary episodes.
- Existing post-seal GPU learning, memory, topology and recovery ordering is unchanged.
- CPU sleeping-body fixture now adds an ordinary sleeping interval and requires body.sleeping before candidate response. This strengthened fixture passed, followed by the full exact-v6 CPU collector:30 passed,0 failed,1 ignored (existing explicit replay benchmark).

## Measured
- Native host producer fixture passed default control, localized withdrawal, exact voluntary physical receipt, measured biology, valid V13 JSON roundtrip, packer and GPU credit row conversion; late sealing failure retries identically. `reflex-gpu-producer-retry-v6.log`.
- Full game_app gpu-runtime library suite: 200 passed, 0 failed, 4 ignored. `reflex-gpu-host-full-v6.log`.
- Exact producer artifact `/workspace/shared/reflex-v13-gpu-producer-v6.json` contains a fresh fixed-fixture sealed patch and receptors. No private life or learned checkpoint used.
- SwiftShader production eligibility/evaluator test consumed that valid V13 patch, built the actual 44-word credit record and uploaded it unchanged, preserving its real one-tick interval. Eleven zero, masked, signed, canceled and decoder eligibility cases passed. Adapter SwiftShader Device (Subzero), Vulkan CPU, vendor6880/device49374. `reflex-gpu-credit-kernel-retry-v6.log`.
- Stronger persisted-credit assertion rerun passed all11 cases (`reflex-gpu-credit-kernel-final-v6.log`). Strict game_app gpu-tests-feature library/tests Clippy passed (`reflex-gpu-host-clippy-v6.log`), including compilation of the opt-in runtime entrypoint. Workspace fmt and diff checks passed.

## Limits and next checks
This is actual software Vulkan arithmetic, not hardware performance, full GPU decision dispatch, transaction admission, commit/replay deduplication, or live-game clearance. The software training backend explicitly rejects normal brain admission; that protection is unchanged. A full production GPU transaction test must use an honest offline harness or real supported hardware, not relabel a software device.

Earlier unchanged baseline pinned-golden test failed under this SwiftShader; separate actual-original-versus-candidate diagnostic passed 7,947 cases. Neither result is silently reclassified by the V13 evaluator pass.

Cognitive sleep interruption is still a separate proposed state-machine change: physical arousal is implemented, but pending/prepared/submitted frozen work must never overwrite later neural writes. See `cognitive-pain-interruption-design-v1.md`. Default gameplay remains unchanged while this boundary is qualified.

## Existing ignored timing tests
All four host-suite ignores are existing performance-only tests, not hidden correctness failures:
- cpu_preparation::approximate_preparation_cpu_timing_evidence: explicit bounded audited/cold/cached comparison.
- cpu_preparation::copy_contract_tests::routing_attention_copy_cpu_timing_evidence: release-only isolated copy comparison.
- cpu_preparation::production_runner_cpu_timing_evidence: explicit release timing.
- neural_receptor_cache::receptor_expression_cpu_timing_evidence: isolated timing without concurrent builds.

## Concrete normal-path promotion checks
1. DONE: exact v6 source frozen after the strengthened CPU body-sleep fixture and full CPU suite; v4/v5/v6 evidence remains separate.
2. Implement and validate versioned cognitive interruption with pending/prepared/submitted/completed/committed drain and exact restore/continuation. Do not falsely finish a cycle or clear its replay.
3. Qualify the GPU transaction boundary (actual pending eligibility, one commit, duplicate/retry rejection, checkpoint continuation) using V13 sealed outcomes. The evaluator fixture does not cover that boundary. Software offline admission remains prohibited for the normal live backend; use an explicit isolated transaction harness or supported hardware without relabeling the adapter.
4. Reconcile the candidate onto main, including already-published ordinary dose, translation/rest and sealed-ABI repairs without duplicating or reverting them; rerun affected CPU/world/GPU-source gates on that exact source.
5. Only after those technical results, switch normal CPU/GPU world producer/body-only paths together and verify existing checkpoints, founder/genetic identities, neural optimizer state and no-contact normal behavior. No additional permission gate is proposed; evidence and preservation determine eligibility.

Strict backend evaluator Clippy also passed. Executables are hashed in bindings.json but excluded. No private checkpoints or learned weight arrays are included.
