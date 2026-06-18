# alife_game_app

Playable-sim app shell crate.

The default path is headless and validates P34 config/assets without requiring
graphics, GPU, semantic providers, school UI, or Bevy runtime support. The
optional `bevy-app` feature constructs a minimal Bevy app shell with the
existing adapter plugin. G02 adds feature-gated visible placeholder entities
from the P34 portable save, but it still does not run live creature cognition.

CI-safe smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
```

Feature-gated Bevy construction smoke:

```powershell
cargo test -p alife_game_app --features bevy-app
```

G02 visible-world signature smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- visible-signature crates/alife_world/tests/fixtures/p34
```

G02 feature-gated visible Bevy scene construction:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/p34
```

The visible-world smoke constructs deterministic placeholder entities from the
P34 portable save and binds Bevy entities to stable IDs through the adapter-local
map.

G03 live brain tick bridge, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-tick-smoke crates/alife_world/tests/fixtures/p34
```

G03 pause and fixed-step scheduler smokes:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-paused-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- live-brain-fixed-smoke crates/alife_world/tests/fixtures/p34 2
```

The G03 bridge runs the existing P15/P17 CPU reference path from gathered
sensory through action arbitration, action execution, outcome measurement,
sealed `ExperiencePatch`, and packed-log telemetry. It does not add G04
rendering polish or G06 gameplay tuning.

G04 creature visual state smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-visual-smoke crates/alife_world/tests/fixtures/p34
```

The G04 visual state is a one-way presentation snapshot derived from P34
visible objects, the G03 live tick summary, bounded drive/hormone values, and
sleep phase. It maps the creature into placeholder animation, expression,
intent color, and bounded cue bars without changing cognition or gameplay.

G05 creature selection and inspector smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-inspector-smoke crates/alife_world/tests/fixtures/p34
```

The G05 inspector uses stable `WorldEntityId`/`OrganismId` values for model data
and remains read-only. Feature-gated Bevy helpers may keep a local Bevy entity
mapping for picking, but that local engine ID is not written into saves or core
contracts. The inspector reports camera focus/follow state, bounded drives and
hormones, current action, last sealed patch summary, memory/topology update
counts, and optional backend/provider troubleshooting messages. It does not
implement any cognition editing.

G06 playable survival-loop smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke
```

The G06 smoke runs a deterministic one-creature loop with visible food, a
hazard, an obstacle, and a rest/sleep cue. Scripted fixture proposals still pass
through structured action arbitration and the P15/P17 CPU reference path, then
produce sealed patches, packed logs, memory/topology updates, bounded
drive/hormone changes, and an app-level event feed. It is a first playable
survival loop, not G07 ecology or balance tuning.

G10 playable school/teacher smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- school-mode-smoke
```

The G10 smoke builds a teacher avatar, ordinary world cue token, highlighted
object, lesson panel, verifier panel, and P34-compatible school save summary.
Teacher cues enter through sensory perception only; low-scoring
teacher-tagged action metadata cannot win arbitration by metadata alone.

G11 semantic provider boundary smoke, no graphics or model required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
```

The G11 smoke keeps the default semantic provider disabled and nonfatal, then
uses a deterministic fake/local table provider to display bounded semantic and
Gaussian context lines. The provider manifest is private-prior metadata only:
it cannot issue actions, mutate weights, or become a required runtime model.

G12 GPU product telemetry smoke, no GPU hardware required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
```

The default command reports CPU fallback, no-readback guardrails, and the manual
hardware report command. With the optional `gpu-runtime` feature, it bridges to
the P29 GPU runtime contracts without making GPU hardware required:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-product-smoke
```

G13 world editor/sandbox smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- world-editor-smoke
```

The G13 smoke pauses simulation, applies bounded stable-ID world edits, saves
and reloads through the P34 portable save contract, then resumes the normal
CPU reference brain path and verifies that a sealed patch is produced. It is a
small sandbox/editing proof path, not the G16 content-authoring pipeline.
