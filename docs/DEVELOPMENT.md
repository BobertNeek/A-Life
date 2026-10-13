# Development

## Environment

A-Life is a Rust workspace targeting Windows, Bevy 0.18, wgpu 29, Vulkan, and WGSL. Install a current Rust toolchain and Git for Windows. Repository PowerShell wrappers call Git Bash for shell gates and avoid accidental WSL use.

Run `bash scripts/setup.sh` to check the Rust, Git, Python, production-asset, Vulkan-tool, Blender, and optional Graphify environment. It reports optional tool gaps without pretending to install them.

Read root `AGENTS.md` and the nearest subtree `AGENTS.md` before changing code or documentation.

## Launch and package

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -NewGame -Population 1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -PreviewCommand
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

The first command starts a fresh Nano512 founder. Resume and `-DryRun` need a valid current save; the bundled legacy save is not a supported starting point. `-DryRun` executes application preflight without opening a window. `-PreviewCommand` only prints the Cargo command. The default profile is `MinSpecComfort1080p`. `MinimumSettings30x30` is a graphics floor, not permission for CPU neural fallback.

Use this release-default launcher for graphical playtests. The unoptimized
`target/debug` game can stall its window during live simulation and is not
performance evidence. For the unpromoted terrain N2048 candidate, add
`-Founder 'n2048:assets/founders/terrain-care-n2048-v1/trained.alife-foundation'`
and `-Seed 539363617` to `-NewGame`. `-UiSettings` accepts an isolated settings
path whose parent also receives the new save. `-RecordPerformance` requires
`-BuildProfile release`; the existing recorder also requires
`ALIFE_PHASE31_SOURCE_HEAD` to identify the measured source.

The player menu opens at graphical launch, except during performance recording.
Play resumes; Escape or the three-bar button opens a pause menu. Escape closes
it while restoring the prior pause state; Speed and Settings use Back/Escape to
return to the main menu. The menu captures world, camera, and teaching input,
including the dismissal frame and held keys/buttons. Typed and creature speech
remain unchanged. Save and Load use the existing authoritative checkpoint
paths; Load waits for pending publication and keeps an open menu paused.
Speed offers the existing 1x and Max graphical modes. Settings' Autopause toggle
controls the existing pause-on-focus-loss setting. Quit saves at the current
paused tick and exits only after that exact checkpoint and journal publication
finish; failures keep the window open. The debug rails remain separate.

The selected-creature HUD uses six distinct icon meters, read directly from the
last complete authoritative presentation frame matched to the selected root's
stable and organism IDs. Left to right: hunger drive (fruit), body-energy reserve
(lightning bolt, not brain ATP), fatigue drive (half-lidded eye), inverse-fear
calm (neutral face), sleep-pressure hormone (crescent), and praise hormone
extension[0] (thumb). These are mixed physiology readouts, not six interchangeable
needs, health scores, or proof of successful care. Calm is the display transform
1 minus validated fear, not a claim about environmental safety. Sleep pressure
is distinct from fatigue; praise hormone is neither happiness nor proof that an
action received learning credit. Meter fill shows the actual normalized amount.
High hunger/fatigue/sleep pressure and low energy/calm add an amber warning
triangle; thresholds 0.65/0.35 are presentation cues only. Praise absence is not
warned as poor health. No HUD logic changes chemistry or selects actions.

Missing identity/frame/row or invalid values show a question mark; a known zero
has a real empty track. Launch/save expression defaults are not evidence. The
HUD does not extrapolate between ticks, during pause, or after a simulation stop.
The reticle and full numeric stable ID identify selection without an invented
portrait. The count chip shows rendered creature count and running/paused/fast
playback icons. F3 hides these player readouts while preserving the existing
textual inspector, animation/response details, and debug rails. The HUD wraps
at narrow widths, and uses shorter cards below 300px viewport height. All HUD
nodes ignore picking and pass UI focus. The existing menu's modal/entry source
layers (100/90) remain above the HUD (30); CPU tests guard those layers and
non-interactive components, without proving combined live input dispatch.
CPU standalone-HUD tests cover 1280x720, 640x360, 320x480 and 320x240.
The ordinary player hint strip is replaced by a five-control wordless care dock:
food placement (apple over ground), gentle touch (open palm), physical toy offer
(striped ball), praise offer (thumb), and speech entry (bubble). These real 48px
buttons and E/C/J/K use one frontend dispatcher; they do not synthesize keys or
select creature actions. Food preserves the existing eligibility of any valid
selected reference with a tile. Care resolves stable selection against the
current living canonical organism and position. Toy placement rejects a carried
ball through the existing world authority. Repeated care doses remain bounded
by that same authority. No new hand mode, biological mutation, learning-credit
claim, or neural CPU fallback is introduced.

Cream silhouettes mean available, a muted silhouette/question means unavailable,
a cross means a rejected command, and a ground-reticle marks a returned physical
placement identity only. The broken ring means accepted queued care whose effect
is unconfirmed. There is no per-care-command completion receipt, so this marker
never becomes successful from a later tick or hormone rise and remains while
paused. Selection changes or a runtime load clear old care feedback. Player speech
translation is a separate global operation; stale runtime results show unavailable. Errors remain
visible as a wordless alert; the existing exact error and action stay in F3.
F1 explicitly opens keyboard help, while the ordinary strip stays hidden.

The dock captures its pointer area and held-button dismissal frames before world
readers. Mouse speech opens the same existing composer path as Enter, establishes
conversation capture before hand/world readers, and keeps all typed teaching text,
creature speech and named-addressee precedence. The composer chrome itself remains
for a separately bounded restyle. A clipped scroll parent follows the existing
end caret so up to 512 typed characters stay intact inside short viewports. The dock has priority in short windows: selected
physiology temporarily hides if it would overlap the actual dock budget or the
expanded text composer/help budget, and restores when space permits. Full live
GPU compositing, engine glyph layout and production neural integration remain
separate qualification boundaries.

The keyboard opening frame consumes its raw input in order: text before the
first non-repeated Enter is discarded, and text after it is retained immediately.
The opening frame does not submit speech, even with a second Enter. Submission in
an already-open composer requires a fresh, non-repeated raw Enter as well as the
existing aggregate input edge, so held repeats cannot retry after an input reset. Closed,
lineage, menu, pointer-open and cancellation paths advance the speech reader so
old shortcuts cannot become later text. Pointer-open still discards the entire
unread keyboard batch; mouse and keyboard messages do not establish a shared
ordering boundary. Six keyboard opening-boundary CPU regression tests were
freshly compiled and passed in the combined care/speech code source tree
`9e6054b6db37673eb299a347c5bf092a8c4744f6`. They exercise the production
boundary helpers; GPU-only input handlers and live input remain unqualified.

Accepted asynchronous player translation clears and closes its submitted draft
immediately. A later successful worker completion preserves the current composer
text and open/closed state, clearing the completed Pending cue while preserving
a newer draft's rejection cue, so reopening to type another draft is safe. Inline
unaided completion, including a worker-start fallback, still clears and closes
the submitted draft after successful emission. Focused CPU state-helper tests
exercise this ownership boundary; they do not instantiate the GPU-only input or
completion handlers or establish live graphical speech acceptance.

Camera controls: hold the arrow keys or move the pointer within 16 pixels of a
game-window edge to pan. Panning releases creature follow. Home snaps to the
selected creature, or finds the first creature when none is selected.
Page Up and Page Down select and follow the previous or next creature. F toggles
follow, and R returns to the starting view. New Game uses the authored island.
Camera movement changes the view without moving creatures or advancing simulation
time. Unbound CPU/ECS scenes remain static diagnostics for meshing, vertex colors,
identity and persistence; they do not install the retired procedural camera stream
or establish playable island acceptance. World shortcuts are suspended while typing
in the conversation panel.

Creature pose transitions blend over time. Ground contact uses the posed mesh
bounds and rendered terrain heights. The approved Hearthling art and its
animations are described under Repository hygiene below.
The shipped assembly metadata covers inherited parts on all supported torso
types. Creature display capacity follows the graphics profile budget, so births
can appear beyond the initial population.
Runtime checkpoints are separated by world identity and graphics profile.
Matching older profile-wide checkpoints are copied to the world's checkpoint
path, retaining the original. Resume accepts population changes within the
selected profile's capacity.

## Standard checks

Run the smallest check that can falsify the changed behavior.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1 --quick
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
```

`check.ps1 --quick` runs only Git whitespace, static dependency/source boundaries, and documentation assertions. The default full check adds formatting, workspace check, workspace tests, and Clippy.

Use a focused `cargo test -p <crate> <filter>` when a Rust behavior changes. Do not launch a second Cargo build while another shared-target build is active.

Source-bound historical evidence tests require complete Git history. CI uses
`actions/checkout` with `fetch-depth: 0`; for a shallow local checkout, run
`git fetch --unshallow` before those tests. The validator still checks the exact
producer revision's source blobs and tree, and rejects altered reports.

Default workspace tests are CPU-safe: they retain shader validation, portable
state, admission/receipt contracts, and typed failure checks without requiring
a real neural GPU. Optional adapter diagnostics may report honest unavailability
in preflight/file-boundary tests. Tests that require neural hardware use the explicit
`gpu-tests` feature. On a suitable Vulkan machine, use
`cargo test -p alife_game_app --features "gpu-runtime gpu-tests"` or
`cargo test -p alife_gpu_backend --features gpu-tests`; the existing
`scripts/run_gpu_closed_loop_gates.ps1` commands opt in as well.
`--all-features` also opts into hardware tests. The backend's `test-support`
feature exposes synthetic admission-budget fixtures for CPU contract tests;
those fixtures do not establish real adapter admission or GPU performance.

The Pass 2 fast-gates workflow also runs a focused CPU speech lane when the
composer, care-dock integration, speech dependencies, or its build setup change.
Unrelated updates skip its Rust/dependency setup; new branches conservatively run
it because the push has no before-SHA. Manual dispatch opts in with `speech_cpu`.
Reproduce it on Linux after installing the distribution's `libudev-dev` and
`pkg-config` packages:

```sh
bash scripts/check_speech_cpu.sh
```

This checks at least seven composer-state, six worker, and six opening-input
tests, including the reopened-draft regression. It refuses empty or ignored
coverage. Two build jobs and test-only optimization/debug overrides bound compile
cost; debug assertions remain enabled. These CPU fixtures do not exercise the
GPU-only Bevy completion handlers, render the live composer, or prove gameplay
acceptance. The existing default-workspace and Tier 0-1 gates remain unchanged.

Release-oriented checks may also require:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

These commands are examples, not evidence by themselves. Record outputs under
ignored `target/artifacts/` paths when a gate requires a durable receipt.

## Focused content and persistence checks

Use the narrow validators for content and portable persistence:

```powershell
cargo run -p alife_tools --bin p34_persistence -- validate-save crates/alife_world/tests/fixtures/p34/tiny_save.json crates/alife_world/tests/fixtures/p34
cargo run -p alife_tools --bin g16_content_authoring -- validate-pack content/fixtures/g16/content_pack_manifest.json
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
```

Validate the committed content pack with `validate-pack` before using it in a
tutorial or package. Optional GPU demonstrations remain manual.
Production creatures use the approved Hearthling GLB. The generated GeneForge
OBJ, socket and mask pack is retired and is not required for launch or packaging.
The compiled `geneforge_recipes.json` supplies current appearance family IDs
and birth/mutation metadata; its output paths and source hashes are historical
authoring metadata, not shipped asset requirements.

For optional offline GeneForge authoring, `scripts/build_geneforge_creature_parts.py`
launches `scripts/geneforge_blender_worker.py` inside Blender. Supply the original
external donor sources and write regenerated output to a separate staging tree.
Do not invoke the worker directly or add that retired pack to the production manifest.

Optional systems must remain optional. A typed GPU unavailability result is a
failure state, not permission to substitute a reference brain.

The readiness resume test checks acquired-state restoration and later gameplay.
Set `ALIFE_PERSISTENCE_EXACT_DIAGNOSTICS=1` to additionally require identical
post-wake receipts, memory metadata, and complete neural snapshots. Differences
limited to inactive external hints are reported separately from gameplay failure.

## GPU gates

GPU claims require a physical adapter/backend receipt. Run the existing serialized hardware gate only when the source identity, worktree, process state, target path, and output path are explicit:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_gpu_closed_loop_gates.ps1
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Rules:

- require source-bound physical-adapter evidence for every hardware claim;
- do not count CPU, noop, fallback, preflight, or schema-only paths as GPU passes;
- bind commit, tree, adapter, backend, arguments, monotonic timing, exit code, and output digests;
- treat unavailable hardware or incomplete evidence as `Unknown` or `Blocked`;
- preserve source-bound caches and reports until their durable result is committed or explicitly discarded;
- never start a competing GPU corpus while one healthy run owns the target.

The formula-derived performance ledger used by its focused test lives at
`crates/alife_tools/tests/fixtures/P04_5_performance_contract.md`.

Remaining retired source snapshots and example assets live under
`archive/legacy_true25d` and `archive/legacy_app_milestones`. They are historical
references, not Cargo targets, supported commands, package inputs, or active
documentation authorities. Removed Pass 2 runners and milestone release-report
wrappers remain available in Git history.

## Architecture boundaries

- World code enumerates unscored candidates and owns legality and outcomes.
- Production neural results come from WGSL state.
- Presentation consumes read-only projections and cannot mutate world truth.
- Teacher cues enter through ordinary perception.
- The local SLM cannot act, target, reward, or write neural state.
- Archive ordering is part of the lifecycle transaction, not a best-effort side effect.

## Documentation changes

The v2.0 controlling architecture is the single normative authority. The root README links the current implementation guides, status, and plans. Update the relevant guide; retain a separate plan only while it coordinates active work. Keep useful outcomes and unresolved decisions when removing superseded plans. Git history retains the originals.

Before committing documentation:

```powershell
git diff --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

Check relative links and scan for stale claims such as production CPU fallback, EI1 promotion, N4096 production support, or unsupported end-to-end gameplay claims.

## Optional code navigation

Graphify is optional and must not be a prerequisite for Cargo commands.
On Windows, use `scripts/graphify.ps1` to find the installed executable even
when it is absent from PATH.

- Run `scripts/graphify.ps1 update --no-cluster` to build or refresh the code
  graph. This performs structural extraction without an LLM.
- Once `graphify-out/graph.json` exists, use
  `scripts/graphify.ps1 query "<question>"`, `path "<A>" "<B>"`, or
  `explain "<concept>"` for focused questions.
- Use `graphify-out/wiki/index.md` for broad navigation when available. Read
  `graphify-out/GRAPH_REPORT.md` only when focused queries are insufficient.
- Generated files under `graphify-out/` are ignored. Check source files
  directly when graph results are stale or incomplete.
- Refresh an existing graph when this task relies on graph results that the changes made stale.
- When the user requests `/graphify`, follow the available Graphify skill.

## Repository hygiene

Production graphics load the approved Hearthling GLB and landscape props from
`crates/alife_game_app/assets/creatures/hearthling` and `assets/landscape`.
The character includes idle, walk, and seated sleep clips. Existing organism
state selects animation, pause freezes it, and the appearance genome supplies
coat, mass, ear/head, and tail variation. The source Blender files and export
commands are documented in the Hearthling asset README. After any art export,
run `python scripts/register_approved_art.py` to refresh the manifest digests.
Terrain remains a projection of the existing streamed world, with shared hill
corners and blended ground colors. The full Blender sample landscape is a
reference scene and is not substituted for the simulated world.
Mouse wheel zooms the creature view. F12 saves the current game view under
`target/artifacts/player-captures`. For bounded visual verification, setting
`ALIFE_GRAPHICS_CAPTURE_DIR` captures at most 96 frames and read-only animation
and grounding receipts while the ordinary game continues.

- Keep one worktree per active isolated task; remove it after its commits are merged and any uncommitted instruction edits are backed up.
- Keep durable reports in tracked report paths. Keep generated Cargo, graph, screenshot, log, and raw corpus output under ignored targets.
- Do not use `git clean`, force-push, destructive reset, branch deletion, reflog expiration, or Git garbage collection as routine cleanup.
- Stage intended files only. Preserve unrelated user work.
