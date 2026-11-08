# Grounded hazard contact

Ground-anchored movement follows the terrain's canonical piecewise-linear triangle route for hazard contact and physical damage. A straight endpoint chord can pass below a convex hill and falsely injure a creature, or intersect a source at a point that is not on the surface. Ordinary movement and the opt-in shared-time withdrawal planner now share the actual terrain segments.

The segment partition is factored from terrain sweep validation: the same canonical f32 grid coordinates, X/Z and diagonal boundaries, finite 64,000-cut limit, triangle ownership, slope and water rules remain in use. No uniform path sampling is substituted. Actual non-grounded starts retain the existing chord transition rather than receiving fabricated zero-time vertical movement. Flat/no-terrain routes keep their prior arithmetic.

The withdrawal planner intersects each real segment with actual damaging sources. At translated coordinates, an exact f64 intersection can round to an f32 point outside the source. It tries the original progress and a bounded stencil of the next eight representable coordinate centers in each moving axis, mapped back to that same plan. Candidates must satisfy terrain, source and unchanged speed/time bounds. Time is taken from accepted progress; a represented full endpoint consumes the full planned interval. This is the first admitted stencil point, not a universal earliest-representable-contact proof. Failure to find a valid point remains fail-closed.

Source reporting requires real segment-entry evidence. A nearby source that never intersects the route cannot be added merely because it is within reporting tolerance. Self-carried hazards remain at constant body-relative separation and cannot supply a body-withdrawal escape direction. Existing blockers and carry legality remain enforced; this is not a redesign of every collision shape.

## Scope and evidence

This changes ordinary grounded hazard contact/dose physics. It does not enable the opt-in awake reflex, change saved schemas, neural genes, optimizer state or policy credit. Receipt path fields remain per-move displacement norms, not curved arclength or a new energy proof.

The preserved prerepair regression measured false chord injury of 0.2 despite no real grounded-path intersection. A separate translated diagonal regression measured an f32 source gap of 0.000015377998, beyond the unchanged reporting tolerance. The corrected finite matrix passed all twenty origin 0/±128/±1000, convex/concave, straight/diagonal rows, with actual grounded point and time/path checks. The main integration additionally asserts actual pain dose and physical source identity for each row.

An earlier near-miss test was invalid: the editor clamped requested radius 0.01 to 0.1 while its assertions used 0.01. The corrected fixture verifies the stored radius. Therefore true-entry filtering is source-evidenced hardening, not a measured pre-filter defect established by that invalid test. Those historical failures are preserved separately.

Frozen research source `67f642a6cf7755beeeb4bbb9ae56055135d10417` passed the full world suite: **645 passed, 0 failed, 3 existing ignored**, plus strict world lib/tests Clippy and workspace formatting. Focused results overlap that full suite and are not additive. The ignored cases are the manual contact-endpoint cost comparison, manual mating-query cost comparison and manual extended soak.

Main integration `08cd4976c4a59bee30e78ef2e1009e200f7d4545`, based on accepted `dd5f669705f41efc983c44249b9d797bae37ffb9`, passed **300 world-library tests, 0 failed, 2 existing ignored**, including the strengthened twenty-row actual-dose/source matrix. Its five world files match frozen research source except for those twelve added assertion lines. Strict integration world lib/tests Clippy, workspace formatting and diff checks also passed.

The source/finite-log archive SHA-256 is `68b476bcc27e6b8812468b26cc09ad6fc87dfd0790f7d2665335d1d8e1ec6bcc`. It preserves the historical failures and fixture correction, source identities, finite results and executable hashes; it contains no executable or private learned-state payload. No GPU execution, autonomous behavioral study, cognitive wake or general learning competence is claimed.
