# Forward sphere-ray correctness candidate, 2026-10-16

Base: `73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`. Isolated CPU candidate; no publication or graphical qualification.

## Observed defect

`HeadlessWorld::first_sight_hit` visits all unconsumed opaque objects other than the observer and optional target. It does not reject objects behind the ray. The original `sight_sphere_hit` clamped its near root to zero without first checking the far root. When both roots were negative, an intersection on the infinite supporting line became a zero-distance ray hit.

Compiled regressions against the unchanged production helper confirmed all three consequences:

- Origin `(0,0,0)`, direction `(1,0,0)`, center `(-100,0,0)`, radius `1`, max range `10` incorrectly returned `Some(0)`
- A rear opaque obstacle caused a front-food `APPROACH` candidate to disappear from real `GroundedTerrainVisionV1` perception
- A sphere behind fan ray 8 changed that lane's ordinary ground range to zero

The corresponding inside/contact and forward/range controls passed before the fix. This is a visibility correctness defect, separate from any spatial-query optimization.

## Bounded repair

Compute the square root once and reject only when the far root is strictly less than zero. Keep the existing near-root clamp and inclusive maximum-range comparison. An eye inside or on the sphere still reports zero, including an origin-surface intersection while pointing away. Forward tangency and exact max-range hits remain accepted.

No changes to optical opacity, candidate scoring, object enumeration, terrain ordering, neural execution, sensory ABI or spatial indexing.

## Focused CPU evidence

Five new tests cover fully rear entries and tangency; inside and surface contact; forward entry, tangency, misses and a one-ULP range boundary; real front-food perception through both indexed and unindexed entry points; and all 16 actual fan lanes against a transparent control.

Before the production edit: 3 regressions failed, 2 controls passed.

After the production edit and formatting: all 5 new tests and 41 existing tests passed:

- `headless::sight_ray_direction_tests`: 5
- `headless::terrain_vision_tests`: 9
- `headless::carry_pose_tests`: 7
- `perception_candidates`: 20
- `carried_food_terrain_vision`: 5

The actual `alife_world` unit-test crate, production rlib and integration tests were compiled using cached CPU dependencies. Source comparison verified the cached `alife_core` dependency source against all 146 Rust source files in the base checkout. Builds ran one at a time, without incremental output, optimization or debug symbols, with debug assertions enabled. Output stayed in the isolated temporary candidate. Formatting, focused Clippy and patch application were checked separately.

## Architecture and qualification limits

This repair supports grounded spatial evidence under AOA-SENSE-001. It does not add recognition, planning or value judgement, preserving AOA-SENSE-002. The controlling architecture and requirement registry remain unchanged.

Unknown: full workspace gates, Windows/release execution, GPU/graphical integration, end-to-end gameplay or trained behavior. No performance or FPS claim. The focused CPU evidence does not establish those broader qualifications. No PC access, GPU, training, package installation, publication or full workspace build was performed.
