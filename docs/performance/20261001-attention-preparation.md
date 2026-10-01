# Prepared attention evidence CPU slice — 2026-10-01

Implementation: `770ef12a06c333b4d18e73879d4799d3cd3f907d`, based on the coherent graphics/memory candidate `a11dae6a5708bddf7d4255990f3bd4abaf12759a`. This is dated source and CPU evidence, not a new performance contract or hardware receipt.

The live preparation path used a baseline finalized perception frame and candidate decision keys only to derive predecision attention evidence. It now derives that evidence directly from the exact validated prepared recall. The existing finalized API and new prepared API share the original salience calculation. Both baseline recall and cognitive context still run. Focus routing/reindexing, remembered novelty, fresh routed recall, projection, routed finalization, the full final guard, and GPU upload remain in place. Whole baseline recall reuse would be incorrect because routing and novelty change query inputs.

`PreparedMemoryRecall::attention_evidence_for_draft` retains the complete existing draft validator: owner/tick/profile/base/context identities, finite context, cognitive owner/tick, candidate count/order, exact query re-encoding, source-reuse provenance/features, and similarity-work receipt. The routed neural frame still receives its decision keys and strict dispatch validation. No policy, cognition, physiology, learning, persistence, population, world cadence, or neural/GPU behavior was reduced. No operational launch source was edited.

Acceptance and architecture scope: preserve organism/causal identity (`AOA-INV-011`), physiology/memory/concept/novelty attention and hysteresis (`AOA-ATT-002`, `AOA-ATT-004`, `AOA-ATT-005`), and evidence-only memory that does not select actions (`AOA-MEM-001`–`AOA-MEM-003`). These checks support this mechanism change; they do not certify all architecture capabilities.

## Measured CPU work

[Raw samples and source/payload digests](evidence/20261001-attention-preparation.json) retain three process runs, 36 cases per run, and 15 samples per variant/case. [The one-off probe](evidence/20261001-attention-preparation-probe.rs) calls the production API and uses an exact temporary extraction of the app's private CPU helpers. Cloud Linux/KVM exposes five AMD EPYC 9V74 CPUs; the probe was pinned to CPU 0. Rust was `1.98.1`; matching Cargo release artifacts were selected from compiler JSON, with opt level 3 and debug assertions off. Shared host isolation is not guaranteed.

For 26 candidates and 64 actually retained memory records:

| Synthetic calls | Attention phase, before → after ms | CPU helper chain, before → after ms | Chain reduction |
| ---: | ---: | ---: | ---: |
| 8 | 3.505 → 0.600 | 13.185 → 10.205 | 22.6% |
| 16 | 7.101 → 1.213 | 26.269 → 20.405 | 22.3% |
| 32 | 14.656 → 2.542 | 54.147 → 42.887 | 20.8% |
| 50 | 22.575 → 3.888 | 82.168 → 64.216 | 21.8% |

The attention phase improved 81.8–83.6% across all 36 cases. Broader chain results varied with candidate/memory sizes: one 2-candidate/256-record/16-call cohort was essentially flat (13.698 → 13.701 ms); the maximum observed chain reduction was 28.8%. At 50 calls/26 candidates, the chain measured 75.545 → 57.529 ms with no records and 108.010 → 89.198 ms with 256 retained records. These results are process-sample medians, not confidence intervals or a gameplay speedup claim.

The isolated phase starts with prepared recall and cognitive context already built. It compares legacy clone/finalization/full final guard/attention extraction against strict prepared attention extraction. The broader chain includes fresh baseline and routed recall, contexts, salience/familiarity/selection, candidate routing/reindexing and novelty, projection, routed finalization/guard, and CPU perception/memory encoding/receptor binding. Both variants use the same linked production library and identical fixtures. Paired sample order alternates; the second process starts with the new variant. Warmup and proof serialization/word extraction are outside clocks.

Cohorts are independent identical fixtures with separate banks/topologies, all using organism 811 and one shared immutable compiled slot for CPU encoding. They contain two observed objects, one learned concept/gap, fixed valid receptor inputs, a default predictor, and focal capacity one. They are not actual resident gameplay. World/perception and receptor derivation, phenotype policy lookup, resident history/hysteresis mutation, backend admission, sleep/lifecycle/sealing/persistence, GPU inference/readback/learning/synchronization, animation, and rendering are excluded.

Even the faster 50-call/26-candidate chains exceed 50 ms. The requested 20 FPS with 50 creatures, sustainable 20 TPS, and bounded debt/drops remain **Unknown**. No RTX3050/i7-3770K test, desktop task, device operation, or GPU campaign was run. Earlier Windows leaf timings motivated removing unused baseline finalization; they are user-supplied evidence from a different tree/host and are not mixed with these samples.

## Equivalence and checks

- Core release `candidate_memory_retrieval`: 35 passed, 1 intentionally ignored microbenchmark. The two new tests also passed with debug assertions enabled on the integrated candidate.
- The new core matrix checks 72 combinations: 2/8/26 candidates, empty/learned/conflicting memory, owner/foreign organism, two novelty values, and attached/absent cognitive context. It includes unknown-individual category fallback and target-free identity, preserves bank state, and rejects stale geometry, reordered candidates, novelty, cognitive owner/tick. Fresh recall of changed drafts remains valid. Existing individual-exception, persistence, and cross-organism tests pass.
- The existing CPU app concept/gap test retains its object selectivity assertions and compares both attention paths through salience, hysteresis, focus/order, novelty, projection, final frames/keys/receipts, and 54-lane perception/memory/cognitive payloads. It passed in debug and release; the release test also passed after rebasing onto `a11dae6a`. This app fixture has empty episodic memory; learned memory is exercised by the core matrix and probe.
- Each of the 2,862 synthetic instances across three runs compares the complete typed result and literal memory header/record/cognitive words outside timing. Perception equality includes exact integer word vectors. All 108 cases retain matching representative proof hashes; nine unique payload proofs are stored as word counts and SHA256 of little-endian bytes. Temporary JSONL retained the full words during verification; the committed probe regenerates them.
- Strict Clippy passed for `alife_core --all-targets` and `alife_game_app --all-targets --features gpu-runtime`. Formatting and static core boundary checks passed. These are CPU source checks; the full native renderer and hardware execution are outside this verification.

## Reproduce the bounded probe

Use the saved cloud lean build environment. This targeted test does not open a GPU device. Select exact library filenames from its compiler artifacts rather than guessing cached rlibs.

```bash
source /workspace/cloud-cpu-validation/env.sh
cargo test -p alife_game_app --release --lib --features gpu-runtime \
  object_bound_concepts_and_gaps_change_predecision_focus_selectively \
  --message-format=json > /tmp/attention-artifacts.jsonl
python3 - <<'PYCODE'
import json, os, pathlib, subprocess
root = pathlib.Path.cwd()
source = (root / 'crates/alife_game_app/src/gpu_live_runtime.rs').read_text()
a = source.index('fn cognitive_context_for_recall(')
b = source.index('fn grounded_successor_state(', a)
helpers = pathlib.Path('/tmp/attention-app-helpers.rs')
helpers.write_text(source[a:b])
artifacts = {}
for line in pathlib.Path('/tmp/attention-artifacts.jsonl').read_text().splitlines():
    if not line.startswith('{'): continue
    item = json.loads(line)
    if item.get('reason') != 'compiler-artifact' or 'lib' not in item['target']['kind']: continue
    for filename in item['filenames']:
        if filename.endswith('.rlib'):
            artifacts[item['target']['name']] = (filename, item['profile'])
command = ['rustc', '--edition=2021', '-O', '-C', 'debug-assertions=no',
           '-A', 'dead_code', 'docs/performance/evidence/20261001-attention-preparation-probe.rs']
for name in ['alife_core', 'alife_gpu_backend', 'alife_world', 'serde_json', 'blake3']:
    filename, profile = artifacts[name]
    assert profile['opt_level'] == '3' and not profile['debug_assertions']
    command += ['--extern', name + '=' + filename]
command += ['-L', 'dependency=' + str(pathlib.Path(artifacts['alife_core'][0]).parent),
            '-o', '/tmp/attention-probe']
subprocess.run(command, check=True,
               env=dict(os.environ, ALIFE_ATTENTION_APP_HELPERS=str(helpers)))
PYCODE
taskset -c 0 /tmp/attention-probe > /tmp/attention-run-1.jsonl
taskset -c 0 /tmp/attention-probe --after-first > /tmp/attention-run-2.jsonl
taskset -c 0 /tmp/attention-probe > /tmp/attention-run-3.jsonl
```

The extracted helper block SHA256 is `c7d7272b085ee6d15876455a71cef5f90811dff5e330eefcdd817eb63b122d08`. The report's JSON records the probe/executable SHA256 and exact linked artifacts/profiles/content hashes. Avoid other compilation or measurement commands during the timed runs.

Mode 1 Micro-Spec; R2 source/test/probe/evidence review by the separate read-only Sol6.1 worker found no blocker; it independently recomputed all summaries and payload byte digests. Residual risk is broader population/gameplay behavior and host-specific performance outside these CPU fixtures. Rollback is reverting implementation `770ef12a`; no saved-state or GPU ABI migration is needed. The three unique final evidence files are this report, its JSON, and its probe; preliminary feasibility runs are not part of the published receipt.
