# The garden survey

Branch: `qualia/garden-survey-20261118`, based on the [grounded landmarks](qualia-grounded-landmarks.md) merge `e740cf25`.

A nursery can hold twelve lantern-pear beds. The Hand’s existing return sign
recalls just the latest cycle. The survey lets the player travel between all
the beds they actually planted, including an older ripe plant after starting a
second garden. It does not replace the Hand or decide what the player harvests.

**Garden** opens the survey from the main menu. F6 opens the same instrument
from play. The needle points toward decreasing world Z. The survey samples
the current terrain’s height and slope to draw its relief and contours. Bed
marks use real positions with equal world scale on both axes, not invented
landmarks or travel routes. A seed, pair of leaves, flower or fruit mark records
the current growth stage. The side lens shows the corresponding release mesh
at a fixed scale. A pair of brackets marks the selected bed.

Touch a mark to turn the lens to that plant. The two arrows reach every bed, even if their positions crowd together at island scale. The return-to-garden sign closes the survey
and frames the marked bed; the seed sign frames a legal nearby patch for a new
planting. Neither button sows or picks. Use the pictured Hand tool on the actual
soil. A trip resumes time only if opening the menu paused it. A previously
paused world stays paused. An empty map contains no pretend crop. A world
without residents can still be tended from an existing bed. The map reads the
existing save, stores no duplicate world records and sends no action to a creature.

The reference remains a cozy, soft-science-fiction *Black & White* × *Creatures
3 / Docking Station* with the original mammalian Hearthling. Use only visual
language in play, apart from true creature speech and self-explanatory single-word
main-menu buttons. Keep screen-reader labels, keep touched code clean, and leave
brains, training, learning, weights and biochemistry alone. A ripe bed is not
evidence that its fruit was eaten.

## Reproduction

- `blender --background --python scripts/render_garden_specimens.py` makes the four original specimen plates directly from the game's garden GLBs.
- `cargo test --locked -p alife_game_app --features production-voxel-frontend --lib` covers zero-resident save/restore, stage truth, coordinate projection, input gating and native UI hit bounds.
- `cargo run --locked -p alife_game_app --features production-voxel-frontend --example garden_survey_walk` drives the native game, plants two beds, switches between them and harvests the older one. A numeric run argument reopens only its own disposable save under `target/artifacts/garden-survey`. On Windows debug builds, use `cargo rustc --locked -p alife_game_app --features production-voxel-frontend --example garden_survey_walk -- -C link-arg=/STACK:16777216` and run the linked executable from the checkout root.
