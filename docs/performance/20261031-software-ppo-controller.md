# Actual PPO-controller equivalence on software Vulkan

The public PPO controller produced **exactly identical captures of 1,470,668
32-bit words across 18 sections** with the legacy allocator and replay scratch
capacity reuse. This is correctness evidence on Mesa software Vulkan. It does not
establish physical GPU qualification or a whole-training speedup.

The explicit `gpu-tests` constructors retain the actual `DeviceType::Cpu`, Vulkan
adapter identity and normal required features/limits. They share backend resource
initialization, accounting and device-loss handling. Ordinary factory/acquisition
CPU and Noop rejection remain unchanged. The fixture permits the Ready offline
training queue and rejects live brain admission, tick dispatch, restore staging
and live actions through a gameplay session wrapper. Backend/runtime software
smoke tests and four admission/selection CPU regressions passed. This feature adds no ordinary
software fallback.

Both controller runs used the existing `FoundationTrainer::from_session`,
`PpoTrainingState` and public cohort controller. No alternate PPO implementation
or allocator mode flag was introduced. The only production API added before the
legacy run was the checked logical output/state word-length getter; it reads the
existing layout and changes no allocation or neural behavior.

| Run | Source commit | Allocator | Result |
| --- | --- | --- | --- |
| Legacy | `f1a3cad491bb2dfb80c7d68f6b6ffe77396e9b6c` | Original exact-shape allocation | Passed |
| Reuse | `3d5e0a3ba47aa9f92d527e7b81a45f5e90f345f0` | Core files byte-identical to `f0606b2d` | Passed, exact capture |

The test file was unchanged: SHA256
`df2f758a287df4b6ae900862152c407767697090e1a888345d4f7caf94644acd`.
Both complete semantic JSON files have SHA256
`918995da9e64af81b6bd4805de968c70d1e5b0551c7d4d7d3d1961882ee40ed1`.
`cmp` and a separate parsed comparison checked every section, category and word.
The [receipt](20261031-software-ppo-controller.json) retains source/binary hashes,
environment, scope and per-category word counts with little-endian u32 hashes.
The 9,446,712-byte raw capture and copied test binaries remain in the recorded
workspace directories; the large arrays are excluded from Git.

The procedural N2048 fixture has 32,768 synapses and one trainable decoder weight.
Its nine windows alternate three ticks/no burn-in and five ticks/one burn-in,
giving three/four loss rows. Effective batch size remains eight: the first update
uses 28 rows with original per-window scales 3/28 or 4/28, followed by a three-row
tail. Both zero and positive value coefficients run. Two accepted actor updates
advance the selected coordinate's integer age and nonzero moments; all 32,767
frozen coordinates remain exact. A second cohort reaches actor step four. Its
uninterrupted owned-window path and restored checkpoint/shared-window path match
exactly, including learned value state and its optimizer history.

A bad old probability in window eight is rejected by the global frozen pass
before any Adam update. A separate old-probability offset of -0.0005 remains
inside the existing 0.001 frozen tolerance while yielding KL 2^-23 above the
1e-10 test cutoff. All nine frozen validations and the first eight gradient
windows run in order before the gate cancels the batch. Actor/value checkpoints
stay unchanged, and checkpoint export confirms no active actor accumulation.

Captured data includes actor weights/moments/integer ages, logical trainer
state/output prefixes, all five value-head planes, active objective
adjoints/metrics/values/preflight, receipts and selected immutable collection
targets. The fixture hash identifies generated context; those selected targets
are not a full replay/provenance serialization. Raw trainer gradients are covered
by the separate direct-device scratch oracle. This test uses terminal transitions
and does not exercise continuing/bootstrap GAE, forced actor-apply failure/value
rollback, post-KL training restart, live world behavior or a real checkpoint.

For each recorded source, the cloud command was:

```bash
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
ALIFE_SOFTWARE_PPO_CAPTURE_DIR=/workspace/software-ppo-legacy \
  cargo test -p alife_training --features gpu-tests \
  --test software_ppo_controller -- --ignored --nocapture
```

The second run used `/workspace/software-ppo-capacity` as the capture directory.
The environment selects only the local Mesa Vulkan ICD with two software worker
threads. The test profile retains workspace package overrides. Test elapsed times
are execution logs, not a training performance benchmark.
