# Private topology preparation CPU slice — 2026-10-01

Implementation: `bf8f6b48e92cc18a9c86163ebea5e0616a89de38`, based on the attention candidate `471c55dfc2a29e760669e8bb6252865464e064fa`. This is source and bounded CPU evidence. The graphics/memory/attention cleanup successor is `851c3a6ffcc498d6b2b3e9a43969cc57aa1308ec`; this measurement branch was kept on its fixed baseline. Its CI could not be checked here because the GitHub API returned Forbidden.

The goal is to remove measured redundant work while preserving every observation, concept/edge/simplex/gap update, degradation, receipt, ownership/replay guard and saved-state validator. Architecture scope is individual causal identity (`AOA-INV-011`, `AOA-AUTH-004`), concept/contradiction learning and grounded influence (`AOA-CON-001`, `AOA-CON-003`–`AOA-CON-006`), and exact saved cognitive state (`AOA-PERSIST-001`, `AOA-PERSIST-002`, `AOA-PERSIST-004`). These tests support this change; they do not certify all architecture capabilities.

Before choosing the change, the dated probe attributed observation cost to the two validations of an already generated private mutation plan. Each reconstructed the entire map from its diff, validated it, and checked its digests. In the final baseline mature-map/26-candidate/50-call process-one profile, these two stages total 53.95 ms wall time; official observation costs 84.94 ms thread CPU. Two app context-summary calls for this empty-memory fixture cost 0.58 ms thread CPU. These separate clocks support choosing the reconstruction seam, without treating the stage sum as an exact CPU share.

Release builds now omit these two private reconstruction checks. The complete planned map is still validated and digested before its private diff is created. Planning and commit are synchronous, with no intervening callback, mutable exposure or serialized plan. Checked replacement application remains unconditional. Full public patch/key/profile/owner/replay checks, expected/final receipt digests, capacity/degradation rules, and portable save validation remain. Debug builds keep both reconstruction diagnostics; the diagnostic method is also tested directly in release unit tests.

For an accepted release observation this reduces full map clones from three to one, full map validations from three to one, and canonical digests from six to two. It leaves the causal topology work and all public trust boundaries in place. A fabricated private plan with a late invalid replacement target could partially mutate before an internal panic in release; such plans are neither constructible nor submitted through the public path. The change does not promise transactionality for arbitrary internal corruption. No context cache, cognition reduction, population/cadence change, neural execution change, GPU launch operation, or save migration was introduced.

## Measured CPU work

[Raw samples, exact build identities and result hashes](evidence/20261001-topology-preparation.json), [the one-off probe](evidence/20261001-topology-profile-probe.rs), and [its dated builder](evidence/20261001-topology-profile-build.py) are the supporting evidence. There are 24 cases: 8/64/256 seeded observations, 1/26 candidates and 8/16/32/50 independently cloned sidecars. Two process pairs run before→after, then after→before, with five samples per process/case: ten per variant/case. The pooled median uses the mean of the two middle samples.

For 26 candidates and the mature map (256 concepts, 255 edges, 256 simplexes, 64 unresolved gaps):

| Independent calls | Observation thread CPU, before → after ms | Reduction |
| ---: | ---: | ---: |
| 8 | 12.107 → 4.730 | 60.9% |
| 16 | 23.890 → 9.846 | 58.8% |
| 32 | 49.221 → 19.879 | 59.6% |
| 50 | 78.188 → 31.218 | 60.1% |

Across all 24 cases the pooled CPU reductions range from 15.1% to 64.6%. The mature 50-call process medians were 84.939→36.728 ms in pair one and 76.521→29.891 ms in pair two. These are synthetic process-sample medians, not confidence intervals. Shared-host variation is visible. The corresponding two identical context-summary calls use empty episodic memory; their mature 50-call CPU medians are 0.576/0.573 ms in pair one and 0.536/0.546 ms in pair two. They do not represent an actual baseline→attention→routed context chain.

The cloud host is Linux x86_64/KVM with five exposed AMD EPYC 9V74 CPUs. Runs were pinned to CPU 0 with no other build or measurement in progress. Rust was `1.98.1`; named rlibs were selected from Cargo compiler-artifact JSON with opt level 3 and debug assertions off. Exact library filenames/profiles/features/package IDs and SHA256s, baseline/instrumented/helper/probe/builder/executable hashes are recorded. Isolation from other host tenants is not guaranteed.

Seed fixtures learn unique tracked objects from valid CPU-sealed outcome patches. They retain 9/8/8/8, 65/64/64/64 and 256/255/256/64 concept/edge/simplex/gap counts, respectively; default capacities are 256/512/1024/64. The measured next patch is new, with candidate count 1 or 26, selected candidate zero, negative valence and pain. Each cohort has separate identical maps owned by organism 811, rather than heterogeneous live residents. Clone/reset, fixture construction and proof serialization are outside timers. Official observation clocks include receipt-vector collection; context clocks include fresh summary allocation.

The source-derived baseline copy adds six wall-clock timing/counter points. It is always the old algorithm, including when linked beside the optimized production library, and its timings must not be read as optimized-stage timings. The only unrelated adaptation changes the return type of an unexecuted context-contribution function across the copied module boundary. The app context helper is extracted unchanged. Official observation also records Linux thread CPU time independently of wall time.

Every one of the 480 official timed batches compares complete serialized receipt and sidecar vectors with the baseline copy outside clocks, covering 12,720 instance comparisons. All initial-map, patch and final full-result BLAKE3 hashes match across both variants and both process pairs. The receipt retains times and digests; full serialized state is regenerated by the probe instead of duplicated in checked-in logs.

Topology observation was excluded from the earlier **64.216 ms** 50-call attention-preparation chain. This change therefore does not lower that previously measured chain or justify adding independent fixture timings into a gameplay estimate. World/perception, actual resident state, neural/GPU inference/readback/learning/synchronization, save I/O, lifecycle and rendering are outside this probe. **20 FPS with 50 creatures, sustainable 20 TPS and bounded debt/drops remain Unknown.** No device, desktop test or GPU campaign was run.

## Verification and reproduction

Core debug and release topology suites passed: 12 topological-map tests, eight sidecar-degradation tests, and the new private-plan diagnostic unit test in each profile. They cover grounded concept creation, strengthening/decay, contradictions/gaps, deterministic IDs and bounded degradation, 10,240 observations, invalid IDs/scalars, organism isolation, duplicate/out-of-order replay, profile/missing-key rejection, portable roundtrip/tampering and lack of action/score authority. The new unit test calls the retained diagnostic directly and rejects six corrupted/stale plan forms without mutating the input map; valid generated replacement commit remains valid.

Strict `alife_core --all-targets` Clippy passed in debug and release. Formatting, static core boundaries and the 77 documentation assertions passed. The separate read-only Sol6.1 worker performed R2 source/test/instrumentation/replay/measurement review. This slice touches only `topology.rs`, its degradation test, and these four dated evidence files. Operational launch source and the other worker's owned simulation/fixture/bundle areas were not edited.

To reproduce, use separate worktrees at the baseline and implementation above. In each, use the lean environment and build only CPU tests. Do not run other compilation or measurements during the timed probes. The builder always obtains the frozen reference topology through `git show 471c55df`; the official library comes from that worktree's emitted artifacts.

```bash
source /workspace/cloud-cpu-validation/env.sh
git worktree add --detach /tmp/topology-baseline-worktree \
  471c55dfc2a29e760669e8bb6252865464e064fa
(cd /tmp/topology-baseline-worktree && \
  cargo test -p alife_core --release --test topological_map \
    --test topology_sidecar_degradation --message-format=json) > /tmp/topology-before-artifacts.jsonl
cargo test -p alife_core --release --test topological_map \
  --test topology_sidecar_degradation --message-format=json > /tmp/topology-after-artifacts.jsonl
python3 docs/performance/evidence/20261001-topology-profile-build.py \
  /tmp/topology-before-artifacts.jsonl /tmp/topology-before
python3 docs/performance/evidence/20261001-topology-profile-build.py \
  /tmp/topology-after-artifacts.jsonl /tmp/topology-after
taskset -c 0 /tmp/topology-before > /tmp/topology-before-run1.jsonl
taskset -c 0 /tmp/topology-after > /tmp/topology-after-run1.jsonl
taskset -c 0 /tmp/topology-after > /tmp/topology-after-run2.jsonl
taskset -c 0 /tmp/topology-before > /tmp/topology-before-run2.jsonl
```

Mode 1 Micro-Spec, R2 review. Rollback is reverting `bf8f6b48`; no saved-state or GPU ABI migration is needed. Remaining risk is broader live-gameplay behavior and host-specific performance outside these CPU fixtures.
