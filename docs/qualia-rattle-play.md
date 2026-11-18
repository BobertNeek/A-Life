# Rattle garden

Branch: `qualia/resonance-play-20261118`. This continues the orchard, field signs and nursery understory on main, not any training experiment.

Cassidy's direction remains a cozy soft-sci-fi **Black & White × Creatures 3 / Docking Station**, with the original mammalian Hearthling intact. Teach through visual language alone: self-explanatory one-word main-menu buttons and the creatures' own speech bubbles are the only visible words. Keep every file cleaner than it was. This slice changes a world object and its Hand controls, not brains, weights, cognition, training or biochemistry.

## Play

Offer the existing orbital rattle to a selected resident using its toy glyph. Hover the loose ball: a string of warm pearls points toward its fixed chime table. Tap the secondary button for that nudge, or hold over the ball, sweep the pearls toward a different piece of ground, and release. The primary button still selects and carries. Selecting a ball and activating the toy glyph (or J) is a timing-independent roll; selecting a creature keeps the original offer. A held ball never turns a refused roll into a new creature offer.

The rattle travels up to six metres on simulation ticks. Terrain height, water, unsupported slopes and solid bodies constrain travel; nearby animals are not knocked aside. The instrument sends out three mint rings only if the real rolling ball touches it. These optical rings are not audible notes or evidence of resident play. The resident remains free to ignore the game entirely.

## Ownership and verification

An active shove is the remaining X/Z displacement for its actual loose world object. It survives an exact save/load, cancels on acquisition or explicit reposition, and leaves no new fields in old, unplayed saves. Restoring a save does not replay a past strike. The Guide pearls are not a promise that an obstacle-free path exists.

On Windows the QA example runs the production GPU frontend with one actual resident, uses the real pointer and accessible toy button, and records before/after shots and observations in its ignored artifact directory. Build with sufficient stack for the debug renderer:

```powershell
cargo rustc --locked -p alife_game_app --features production-voxel-frontend --example rattle_walk -- -C link-arg=/STACK:16777216
.\target\debug\examples\rattle_walk.exe
```

Earlier orchard runs produced harvests, but did not establish that a resident ate a harvested pear. The subsequent fifteen-minute fieldglass run found the identical directly placed food still loose. Neither result promises a biological reaction to this rattle.

On real RTX 3050 hardware the pointer tap reached the instrument at tick 89. A separate, aimed return travelled about three metres across the ground; the accessibility button then sent that identical ball back into the table at tick 165. Both strikes produced their own optical feedback. That verifies Hand play, not a resident's decision to join it.
