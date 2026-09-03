# alife_bevy_adapter

Bevy runtime adapter for A-Life.

`AlifeBevyAdapterPlugin` installs only stable Bevy-to-world identity mapping. It is safe to use in the production game because it adds no simulation systems and does not scan render entities.

`AlifeReferenceAdapterPlugin` is an explicit example/test pipeline. It gathers tick-bound derived sensory snapshots and translates queued actions into plans. The authoritative host supplies the tick, executes each plan, and measures physical and biological outcomes. The adapter never advances a simulation clock, runs a neural brain, or fabricates reward and homeostatic changes.

Cognitive contracts belong in `alife_core`. World legality and outcomes belong in `alife_world`, where they remain testable without Bevy.
