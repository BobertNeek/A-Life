# Selected focal novelty reuse — 2026-10-19

This is a fresh current-source optimization against main
`ea15a82e0a0ff2f4b92b8dc78c75c7e55bda0cff`, verified by a fresh remote fetch.
It is not an exact reconciliation of the unavailable preservation bundle.

## Candidate disposition

| Candidate | Current-source finding | Disposition |
| --- | --- | --- |
| Focal novelty reuse | `crates/alife_game_app/src/gpu_live_runtime/cpu_preparation.rs` computes tracked novelty before selection and recomputes the first tracked focal object's familiarity after routing. | A genuinely remaining waste pattern; addressed by this fresh focal-only change. Exact historical patch equivalence is unknown. |
| Food-transform bit guards | `crates/alife_game_app/src/production_voxel_renderer/live_food_projection.rs` still assigns retained-object translation in `sync_care_objects` and X/Z in `align_carried_care_objects` without bit guards. | Remaining separate pattern; not implemented here. Signed zero, NaN and carrying/change-tick parity need separate tests. |
| PR73 carrier-offset reuse | `align_carried_care_objects` lazily builds one local carrier-offset map for carried objects. | Existing earlier reuse, distinct from the absent transform guards. |
| PR59 trainer pipelines / verified replay bytes | `crates/alife_training/src/trainer.rs` retains device-local pipelines/layout across replay shapes; `crates/alife_game_app/src/foundation_training.rs` authenticates one byte buffer before decoding it. | Existing independently landed work; no changes or GPU execution here. |
| PR78 nonvisual index / density fallback | `crates/alife_world/src/headless.rs` retains the raw occupancy fallback, including the early observer-cell shortcut. | Existing independently landed work; dense-regressing candidates remain quarantined. |
| JSON-read reuse | Not investigated or changed. | Deprioritized. |

GitHub reports PR59 and PR78 merged; their integrated heads are ancestors of the
verified baseline. That proves the identified independent work is present, not
that either unavailable candidate patch landed.

## Preservation limits

The supplied food guard identity is tree
`119b7cac916017b9ec19a6cca09a8bf7da627dab`, source blob
`3c7ed1e26a8dc70eb09d7734505eb09b7d0cbd16`, patch SHA-256
`97aacfbd57e8034ae5e0869cade703b9f956f9c9ee015a0bd4b4ba185c1eac94`.
The supplied focal identity is tree
`0d4fc5f5ab851a1b3231c60af34e0ea2cf68fb89`, patch SHA-256
`c6ee75a9b1925213ee3773455948de82ddc70e2e2b0458301c4dae4a665bad3a`,
based on main tree `d5507cba2f7222217560d7250c8b6a7b5a9d3eda` and reportedly
combined with PR70 tree `ed2b98d60bb236d20e5bc07cbb517db761a604aa`.

The fresh checkout contains neither candidate tree, the supplied food blob,
nor the combined PR70 tree. It does contain the stated main base tree and
PR73 commit `bd88de192df5e52fdb43fa26ea44f430791b9fe4`, whose actual tree is
`e6b9b9038019bc4ea8692c5676319f5c2d5a1762` and food projection blob is
`f2795d01fef850e77c1ff35e95b6f90c18ae39cc`. Those base objects cannot
reconstruct the missing patch or verify either patch SHA-256. Repository and
app instructions were read; no checkout `.agents/skills` directory is present.

## Safety and bounded benefit

In `gpu_live_runtime/cpu_preparation.rs`, the ordinary path writes
`NormalizedScalar::new(1.0 - object_familiarity(...))` to each tracked summary.
`NormalizedScalar::new` validates and stores the original float without changing
its bits (`alife_core/src/units.rs`). `select_focal_targets` copies identities
and salience from the same final selected indices (`alife_core/src/attention.rs`),
including incumbent retention, ranking, ties and truncation. Routing borrows the
attention frame and reconstructs only the draft. The memory bank remains an
immutable shared borrow throughout these operations.

`object_familiarity` reads the target namespace index and records, sums bounded
observation counts and returns `1.0 - 1.0 / (1.0 + observations as f32)`
(`alife_core/src/memory.rs`). It does not mutate the bank. Reusing the paired
selected novelty therefore preserves the exact result of the removed call.
Empty or nontracked first focus still yields positive zero; the implementation
does not look ahead to a later tracked target. No persistent cache is added.

For P tracked peripheral summaries and a tracked first focus, source-derived
familiarity calls fall from P+1 to P. One removed call means at most
`MEMORY_TARGET_SEARCH_CAP = 64` target record IDs need no second visit. Empty
and nontracked focus already made P calls. This count is not a measured
speedup, and no elapsed-time, FPS or GPU benefit is established. The separate
`approximate_cognition.rs` path remains unchanged.

This preserves the existing mechanisms relevant to AOA-ATT-002/005,
AOA-MEM-002 and AOA-CTX-001; it makes no new architecture compliance claim.
No PC, GPU dispatch, training, learning evaluation, production checkpoint,
biology, reward or optimizer state is accessed or changed.

## Verification

Two new CPU regressions compare 72 selected-attention cases bit for bit against
the former familiarity expression and 12 full preparation cases against the
independently spelled frozen legacy implementation. Cases cover ranking,
reversed peripheral order, retained and absent incumbents, zero requested
focus, empty summaries, all three nontracked identity kinds ahead of tracked
targets, and multiple focal capacities. Full preparation comparison includes
frame keys, memory receipts, candidate routing, cognitive projection,
hysteresis and literal upload words. Fixture memory/state snapshots must
remain equal. The existing zero/64/256-record full-trace and worker tests remain
independent and unchanged.

The CPU preparation suite passed with `--features gpu-runtime`: 21 tests passed,
3 existing timing probes were intentionally ignored, and none failed. Formatting,
static core boundaries and all 77 documentation assertions passed. The existing
full-workspace CI command enables `alife_game_app/gpu-runtime` through
`alife_tools/Cargo.toml` and runs these tests without a filter. This feature
connection is confirmed by `cargo tree --locked --workspace -e features -i
alife_game_app`; these preparation tests construct no device or dispatch.

All 7 `alife_core --test attention_hysteresis` integration tests passed. Strict
`alife_game_app --features gpu-runtime --all-targets` Clippy passed with warnings
denied. These checks used Rust/Cargo 1.98.1 in the fresh cloud environment.
Draft PR100 initially added a redundant targeted CI step. The hosted art audit
correctly rejected that workflow edit because its exact bytes are source-pinned.
The redundant step was removed, restoring `.github/workflows/ci.yml` byte for
byte to the verified base. No pin, art audit, historical profile or existing
workflow behavior was changed. The resulting three-file change retains
aggregate test coverage without expanding into art provenance updates.
After restoration, both art audits passed locally, along with the 74 composition,
4 preview-rotation and 3 main-integration CPU tests and all 77 docs assertions.

An independent internal R2 source review passed on the initial source head.
The final corrected head requires its own R2 confirmation and hosted aggregate
checks before the parent-owned ready/merge gate. No runtime performance
measurement was made.
