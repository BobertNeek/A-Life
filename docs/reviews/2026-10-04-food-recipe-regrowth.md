# Tracked food recipes survive regrowth — 2026-10-04

Cassidy authorized fixing discovered issues while keeping efficient WWCD
behavior. This follow-up is independent of the contact-chemistry branch and
starts from main `482948ec116bad505cee0225cad74c1ab110acb7`.

The extended existing canonical food-variety test reproduced the defect before
production changes. New Game seed 240824 constructs tracked Seed food with
nutrition 0.85. The scenario's `seed / 12` recipe selects Fruit (0.45). The first
registered meal received 0.45, but after the actual 48-tick regrowth interval the
same Fruit physical cues accompanied nutrition 0.85. The assertion failed with
`left: 0.85; right: 0.45`. No trainer or neural execution was needed to reproduce
the shared world recipe path.

The private intentional-recipe setter now synchronizes an existing tracked
`ResourceLifecycle.base_nutrition` with the object's new recipe nutrition.
It is named `set_food_recipe_nutrition` to keep this boundary explicit. Its only
caller is `set_food_variety`; consumption, decay and regrowth do not call it.
Untracked food still receives the same immediate nutrition. The existing meal
energy calculation, physical cues, inherited chemistry and genetics are
unchanged. No new persistent field, migration or compatibility branch is added.

The existing food/toy regression now checks Fruit, Root and Seed through first
registered ingestion, current portable save/load while consumed, natural
regrowth and second registered ingestion. Physical cues remain paired with
recipe nutrition, loaded and uninterrupted world signatures and second-meal
biology agree exactly, and the second meal has the same body-event energy as
the first. Existing toy and genetic consequence assertions remain in place.

This adds one allocation-free bounded resource lookup only when a recipe is
intentionally changed. It adds no per-tick scan, contact work or new index.
That cost statement follows the source; no timing benchmark or population
performance claim is made for this small construction fix.

Sources: [recipe setter](../../crates/alife_world/src/care_objects.rs),
[nutrition and lifecycle](../../crates/alife_world/src/headless.rs),
[current canonical regression](../../crates/alife_world/tests/canonical_new_game.rs).
Requirements traced: AOA-WORLD-001, AOA-BODY-005, AOA-BIO-006, AOA-PERSIST-004.

The shared `headless.rs` change is confined to the private recipe setter, separate
from the contact branch's chemistry/perception sections. No GeneForge PR3,
island, art, renderer, training or PC files are edited. Both drafts require
coordinated merge review; neither is merged by this task.

Validation passes: all 353 world tests (one existing optional benchmark ignored),
strict workspace all-target Clippy with warnings denied, formatting,
`git diff --check`, dependency/source core boundaries and all 77 documentation
assertions. The before-fix reproduction failed at actual regrowth with the
expected stale 0.85/0.45 mismatch. No training, GPU or graphical workload was run.

```sh
cargo test -p alife_world --test canonical_new_game food_and_toy_variants_have_physical_gene_controlled_and_persistent_effects -- --nocapture
cargo test -p alife_world --all-targets
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check_core_boundaries.sh
bash scripts/check.sh --quick
```
