# Training build-profile audit and shared-host fixture

**The installed PC binary's profile and historical b817 profile are UNKNOWN.**
No matching executable/build receipt was available in this workspace, and no PC
access or launch was performed. No whole-training gain is established here.

Source defaults do not identify the installed executable:

| Path | Source-established build/selection |
| --- | --- |
| [Windows training artifact](../../.github/workflows/windows-training-artifact.yml) | `cargo build` without `--release`; packages `target/debug`; receipt says `build_profile=dev`. |
| [Resident experimental artifact](../../.github/workflows/windows-resident-experimental.yml) | Manual `workflow_dispatch` defaults to `release`; PR runs fall back to `dev`. It selects the matching directory and records the chosen profile. |
| [Legacy local launcher](../../scripts/run_n2048_care_training.ps1) | Explicit release build, release binary and source receipt. |
| [Continuous host](../../scripts/run_continuous_care.py) | Requires explicit native/inspector paths and hashes. It neither chooses a profile nor authenticates a build receipt. Runtime checkout/source revision is not compile identity. |

[Cargo.toml](../../Cargo.toml) optimizes hot local crates only under
`profile.test`. Its dev package overrides remove debug symbols, not optimization.
The fixture's Cargo JSON artifacts independently confirm these actual builds:

| Profile | core/world/example opt-level | Debug assertions | Overflow checks | Test build |
| --- | ---: | --- | --- | --- |
| dev | 0 | true | true | false |
| release | 3 | false | false | false |

This compares complete dev/release profiles, not optimization level alone. Cloud
dev symbols were disabled with `CARGO_PROFILE_DEV_DEBUG=0`; that does not change
the recorded opt-level or assertion/check flags.

The [small example](../../crates/alife_world/examples/profile_shared_host.rs)
constructs a disposable canonical N2048 world using the existing policy-signature
fixture's procedural genetic asset. Seed is 71001, founder seed 539363617, sensor
profile `GroundedTerrainVisionV1`, population one or six. For each of 512 passive
world ticks it builds the shared perception index, constructs/finalizes each
grounded draft using actual current biological homeostasis and empty context,
then calls the safe public world-tick transaction, including its rollback clone.
These APIs are shared with GPU collection in `gpu_live_runtime/staged_tick.rs`
and `gpu_live_runtime.rs`.

There are **no neural actions, cognition, episodic recall, PPO, device work or
persistence** in this assay. It is not a complete causal-care/training run and
does not qualify learned checkpoint or GPU continuity.

| Population, 512 world ticks | dev median | release median | Absolute saved | Component ratio |
| --- | ---: | ---: | ---: | ---: |
| 1 | 238.087 ms | 16.503 ms | 221.583 ms | 14.43x |
| 6 | 1179.842 ms | 92.720 ms | 1087.122 ms | 12.72x |

Dividing those whole-loop medians by 512 gives derived average per-world-tick
costs, not a distribution of individual tick timings: population one
0.465013 → 0.032233 ms, saving 0.432779 ms/tick; population six
2.304379 → 0.181094 ms, saving 2.123285 ms/tick.

There are seven alternating paired trials per population, including the first
pair, each in a fresh child process. All complete frame-signature tuples and
initial/final canonical world signatures match exactly across profiles and
trials: 512 frame tuples at population one and 3072 at population six. An
independent reviewer recomputed the statistics and reran both profiles at both
populations, verifying all tuples and canonical digests against the cohort.

The primary timer excludes world construction, initial/final canonical hashing
and JSON output. It includes frame cached-digest extraction/vector recording and
world rollback cloning. `setup_seconds` measures construction only; initial
hashing, roster/preallocation and final hashing/output are outside that stage.
Full child invocation wall time includes startup, setup, signatures, JSON output
and capture: 543.808 → 59.235 ms at population one, and 1820.262 → 185.308 ms at
population six. Those 9.18x/9.82x invocation ratios are **not simulation or
training speed**.

[Raw evidence](20261031-shared-host-build-profile-trials.json) records every
trial, ranges, per-stage costs, full Cargo artifact profile fields, source and
binary SHA-256 hashes, and signature hashes. Host: Linux x86_64, AMD EPYC 9V74,
five visible CPUs. Compiler: rustc 1.99.1
(`58cf8f9c35b355199727627d07ae34a4e2846d67`), LLVM 23.1.1.
Dev/release example Clippy, formatting and whitespace checks passed.

Cloud reproduction:

```bash
source /workspace/cloud-cpu-validation/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life/target/cloud-profile
cargo build --locked -p alife_world --example profile_shared_host --message-format=json > dev-build.jsonl
cargo build --locked --release -p alife_world --example profile_shared_host --message-format=json > release-build.jsonl
$CARGO_TARGET_DIR/debug/examples/profile_shared_host 512 1 > dev-1.json
$CARGO_TARGET_DIR/release/examples/profile_shared_host 512 1 > release-1.json
```

Use population six as well; run seven pairs, alternating dev/release order. The
exact equality/hash recipe applied to each captured JSON receipt is:

```python
keys = ("ticks", "population", "initial_signature", "final_signature", "frame_signatures")
semantic = {key: receipt[key] for key in keys}
assert semantic == expected_for_this_population
digest = hashlib.sha256(json.dumps(
    semantic, sort_keys=True, separators=(",", ":")
).encode()).hexdigest()
```

No default change is proposed while the installed profile is unknown. The
existing manual resident workflow already offers an explicit release artifact.
The sole PC owner must first match actual native/inspector SHA-256 pins to a
trusted build receipt containing source revision/tree, profile, features, target,
compiler and workflow provenance; any Cargo overrides also matter. If already
release, a profile switch supplies no expected gain. If verified dev, a separate
exact-source release artifact can be qualified without replacing bound recovery
executables in place. Finish/reconcile the existing owner before an explicit new
launch, preserving Current, actor/value weights, moments, integer ages, private
world and semantic-prior identity. This audit changes no launcher, workflow,
checkpoint or active training behavior.
