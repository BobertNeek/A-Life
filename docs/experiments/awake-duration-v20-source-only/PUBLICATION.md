# V20 source proposal: preserve actuator time explicitly

This is an **uncompiled, unqualified source backup**, not a result, default activation or merge candidate. It follows the measured [V19 component and failures](../awake-shared-cpu-v19/PUBLICATION.md). The original source-only package and SHA256SUMS are preserved byte-for-byte.

V19 demonstrated a false residual interval when elapsed time was reconstructed from rounded absolute world coordinates. V20 instead carries planned/accepted duration and its provenance through movement. A full step owns the interval; early goal arrival retains genuinely remaining time; a swept contact consumes the planned duration multiplied by the contact fraction. Actual displacement, effort, lawful terrain/carry checks and strict path limits remain separately validated. The exact no-effect learning predicate is unchanged.

Frozen source: f085d026b7a8eec21866cb44cc21754708a6b105, tree503a51eb. The three source files here match that proposal. Fixed fixtures cover translated full/half-speed movement, early goal arrival, zero speed, ordinary blockage and a translated actual CPU meal. They have not run at this backup point. See PLAN.md for design and intended controls.

Known separate limitations remain: a curved-ground candidate prefix can still be rejected, and strict f32 path caps can reject some rounded endpoints (X128 arithmetic is a prospective counterexample, not yet a native outcome). There is no arbitrary-translation-invariance, GPU/live-game, acquired learning or biological improvement claim.

Source and finite test design only are uploaded. No executable, live journal, checkpoint, bank or optimizer state is included. Original patch whitespace is retained for hash identity. Historical V19 results do not qualify this revision.

The design practice follows [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), especially targeted repair and iterative feedback; these component proposals do not reproduce its neural-network results.
