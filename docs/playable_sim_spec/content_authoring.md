# G16 Content Authoring Pipeline

G16 adds a small, versioned content-pack contract for adding playable-sim worlds,
lesson packs, creature presets, generated-weight references, semantic fixture
references, and scenario packs without changing Rust code.

The pipeline is tooling-only. It validates content before runtime use and does
not make `alife_tools` a dependency of `alife_core` or default gameplay crates.

## Tiny Sample Pack

The committed sample pack is:

```text
content/fixtures/g16/content_pack_manifest.json
```

It references existing P34 config, save, and asset manifest fixtures, plus tiny
G16 world, lesson, creature, and optional fake semantic files. Bulk tensors,
large logs, GPU captures, and generated reports must stay out of committed
content packs.

Validate the sample pack with:

```powershell
cargo run -p alife_tools --bin g16_content_authoring -- validate-pack content/fixtures/g16/content_pack_manifest.json
```

## Authoring Rules

Content manifests must use current G16 schemas:

- `alife.g16.content_pack.v1`
- `alife.g16.world_preset.v1`
- `alife.g16.lesson_pack.v1`
- `alife.g16.creature_preset.v1`

All paths are workspace-relative and portable. Do not use absolute paths, parent
directory escapes, Bevy entities, Avian handles, wgpu handles, renderer handles,
or OS window handles in content files.

World presets use stable `WorldEntityId` values and finite positions. Lesson
packs are perception-only: they may describe hearing, vision, or touch context,
but they must not issue motor commands, bypass arbitration, or inject hidden
vectors. Creature presets may reference generated inherited weights as birth
assets only; they must not include lifetime state or learned consolidated
weights.

## Validation Commands

Validate individual content files with:

```powershell
cargo run -p alife_tools --bin g16_content_authoring -- validate-world content/fixtures/g16/worlds/tiny_meadow_world.json
cargo run -p alife_tools --bin g16_content_authoring -- validate-lesson content/fixtures/g16/lessons/grounded_food_lesson.json
cargo run -p alife_tools --bin g16_content_authoring -- validate-creature content/fixtures/g16/creatures/nano_forager.json crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json
```

Use P34 validation for referenced runtime configs, saves, and asset manifests:

```powershell
cargo run -p alife_tools --bin p34_persistence -- validate-fixtures crates/alife_world/tests/fixtures/p34
```

On Windows, repository shell validation must use the PowerShell Git Bash
wrappers, not plain `bash scripts/check.sh`.

## Extending Content

To add a small world or lesson pack:

1. Add a new versioned JSON file under a content fixture or asset directory.
2. Reference it from a G16 content pack manifest using a workspace-relative path.
3. Keep files under the committed fixture size cap unless the file is an
   external asset referenced by digest.
4. Run the G16 validator and the P34 fixture validator.
5. If a new gameplay action or sensory channel is needed, implement the relevant
   future plan first; do not smuggle behavior changes through content metadata.

World editor exports from G13 should remain stable-ID based and may be referenced
from a content pack after P34 save/load validation succeeds.
