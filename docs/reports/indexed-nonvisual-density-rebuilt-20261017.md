# Rebuilt nonvisual density fallback, 2026-10-17

## Draft status and exact provenance

**Unqualified draft candidate. Production performance is Unknown. Keep this PR
in draft even if hosted CPU CI becomes green.** Fresh paired dense/sparse CPU
timings and live RTX production qualification remain separate adoption gates.

This is newly rebuilt source based on exact main
`73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`, freshly confirmed before publication.
It is not recovered historical densityfix or combined-density-forward-ray bytes.
It inherits none of those missing candidates' measurements. The independently
published forward-ray/sphere fix is not included in this change.

Source inputs verified before composition:

- Quarantined original indexed patch SHA-256:
  `25e554263e578535afa4ff2e7de73c659eb35e065f68b9444688ee6bf4cb6fba`
- Rebuilt indexed/density patch SHA-256:
  `70d35fe7629516bd78bd227ca2acb4d751dc0e3c94a3b6cf47737d87fe1e0aa1`
- Append-only nine-test live-read delta SHA-256:
  `a4c7e8915bdeac68a5b10a05f1d2a397b804608a7802633211fbbc2b19b905e7`

The composed source retains the exact rebuilt production implementation and all
26 prepared nonvisual tests. One test seed literal is regrouped from
`0xE10_71017` to `0xE107_1017`, preserving its numeric value for strict linting.
This report supersedes the source-only rebuild report. The adjacent original
indexed-perception report is explicitly historical; its 52 focused tests,
formatting and Clippy successes are not results for this candidate.

The original indexed patch independently regressed dense CPU totals by
4.34x-8.05x. Those measurements quarantine the original; they do not measure
this different rebuilt candidate. No CPU speedup or FPS improvement is claimed.

## Implementation and preserved semantics

The existing batch index additionally records maximum unconsumed contact radius,
ascending owner/carried IDs, and a nonfinite-input full-scan flag. Query support
includes contact and both actual f32-rounded lateral odor samples. Outward f64
bounds conservatively include support; finite cell-volume guards and canonical
fallback prevent pathological cell traversal. Far carried sources are included
independently of position. IDs are sorted/deduplicated into canonical entity
order before the unchanged sensory arithmetic runs.

The rebuilt fallback adds two deliberately unmeasured policies:

- At at most 16 objects, use canonical nonvisual enumeration before computing
  query geometry and skip nonvisual-only builder metadata. Existing optical
  cells, organism mapping, position and duplicate-ID validation remain intact
- Above 16 objects, set a raw-ID occupancy ceiling of ceil(object_count / 16).
  Check cell and carried-list sizes before copying IDs. Reaching the ceiling
  selects canonical enumeration before sorting or per-ID resolution. Self,
  consumed and duplicate membership count conservatively. Retained sparse
  sequences are the original indexed sequences

Fallback always runs the complete original kernel, with no partial sensor result.
Canonical float accumulation, four odor bands, left/right samples, touch,
temperature, possession, ground, care, taste, injury/pain, current gains and
clamping remain unchanged. Optical sensing, ray/sphere code, candidate ranking,
neural/GPU execution, training, lifecycle and persistence are unchanged.

## Live reads and the actual index boundary

This index caches source-query metadata, not completed perception or observer
biology. Each capture continues reading current observer pose/yaw, authoritative
registry/embodiment gains, injury-derived pain, pending player care and current
ingestion/lifecycle eligibility. Selected IDs resolve current source objects.

The current production preparation batch builds the index before per-owner
capture and finishes captures before selected actions and world advance. Registry,
sleep/ATP bookkeeping and ingestion pruning can occur between captures without
changing source-query metadata. A registry replacement alone does not require
rebuilding this index. No observer-state freezing or reordering is introduced.

The required invariant is stable source-query metadata sufficient to select a
complete candidate set. The existing guard checks only seed, world tick and
object count; it does not detect arbitrary same-tick movement, radius increases,
new ownership or remote finite-to-nonfinite chemistry transitions. Rebuild after
relevant source mutation. A moved observer's own query over fixed other sources
does not establish globally movement-safe reuse, because that body may itself be
a source in another observer's cached cells. No stronger invalidation contract
is added or claimed, and no staged-tick production bug is established here.

## Checks actually performed before publication

- Exact original/rebuilt package checksums and both input patch hashes passed
- The nine-test delta replayed onto the rebuilt module byte-for-byte; all 26 test
  functions remain after the seed-literal-only lint normalization
- Fresh source checks passed for unchanged canonical arithmetic, four optical
  helpers, visual nearby enumeration, snapshot validation and nonvisual bounds
- The rebuilt production delta from the quarantined original remains limited to
  its two intended functions and two density constants
- All 60,504 deterministic Python selection-model cases passed again, including
  20,000 randomized cases; these model selection/control flow only
- Direct source review verified composition, live reads and narrow production
  scope; exact-main affected-file Git blob identities were checked
- Combined forward/reverse patch applicability and whitespace checks passed

At initial publication no rustc, cargo or rustfmt was available in this native
environment. No local Rust compilation, Rust test, Clippy or rustfmt ran. The
26 Rust tests are prepared assertions, not passed tests at publication. Hosted
CPU CI is being used for fresh compiler/test/lint qualification; its results
must be tied to the exact remote head and must not be inferred from this report.

The original 17 tests cover spatial/float boundaries, four-band ordered sums,
far and duplicate possession, nonfinite fallbacks, <=16/17 metadata boundary,
dense packet parity and ceil(N/16) occupancy thresholds. Nine added tests cover
reused-index live gains, injury, care, ingestion/removal/pruning, selected payload,
consumption and observer pose/yaw. Two of those nine deliberately characterize
unsupported stale-source reuse and recovery after rebuilding. Invalid nonfinite
fixtures bypass the public validator and do not assert that it accepts NaN/Inf.

## Open adoption gates

1. Compile and execute the final 26-test module, perception_candidates,
   carried_food_terrain_vision, terrain-vision and carry-pose groups. Run normal
   formatting, Clippy and relevant repository aggregate checks on the exact head.
   Distinguish current executed results from source models and historical tests
2. Measure actual canonical, original-indexed and rebuilt production paths with
   alternating paired CPU timings, including index construction/destruction and
   all observers. Cover sparse and dense occupancy at 1%, 5%, 6.25%, 10%, 12.5%,
   25%, 50%, 100%; 16/32/64/128-object worlds; 1/8/32 observers; local/far/duplicate
   ownership; oversized-radius and nonfinite fallbacks. Above 16 objects, dense
   fallback still pays added nonvisual builder metadata cost
3. Require dense totals near canonical and measured sparse gains before accepting
   the final threshold. Model operation counts and early exits are not timings
4. Separately qualify the exact production source in a fresh live RTX benchmark.
   No GPU, training or live runtime/FPS benchmark was run for this publication

No merge, mark-ready, external reviewer request, PC task, saved environment,
toolchain installation, dependency/cache copy or cache deletion was performed.

## Narrow architecture trace

- AOA-WORLD-001/002 and AOA-SENSE-001/002: grounded physical observables and
  unchanged unscored world sensing; no semantic recognition or policy added
- AOA-TIME-001: canonical entity and floating-point accumulation order retained
- AOA-AUTH-002: batch-owned read-only tick-bound metadata; live authoritative
  observer inputs preserved, existing source-lifetime limitation disclosed
- AOA-INV-005: finite query-cell and conservative raw-ID budgets select complete
  canonical fallback; threshold efficiency remains empirically unqualified

This narrow source review does not certify full architectural compliance or
causal gameplay acceptance. Missing runtime/performance evidence remains Unknown.
