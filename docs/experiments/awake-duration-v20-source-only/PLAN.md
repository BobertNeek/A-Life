# V20: explicit translation duration provenance

Separate revision after frozen V19. Do not modify V19 evidence or relax its exact no-effect predicate.

Reproduced defect: a full accepted move at X=3 yields rounded displacement0.099999905 and reconstructed elapsed time0.99999905, creating phantom remaining time and effort. An origin-centered fixture passes but does not repair the defect.

Proposed internal-only change:
1. Carry the intended actuator duration from move planning: Forward/Flee own a full interval at positive speed. A goal-directed move owns a full interval when the goal exceeds the planned step, otherwise the explicit goal-arrival fraction computed from the unrounded relative vector. Zero-speed and already-at-goal remain distinct.
2. Preserve this duration through the existing local steering and terrain acceptance. These may rotate/project the accepted path but must not reconstruct elapsed time from final absolute-coordinate subtraction. Inspect every route before assigning duration; blocked attempts retain their existing full-attempt time semantics.
3. For a swept painful contact, allocate planned duration multiplied by the admitted contact fraction. The remainder alone is available to the reflex. Actual receipt displacement/path/energy still reflect actual physics and remain validated against the shared budget.
4. Carry the accepted duration in ephemeral transaction state, with no persistent neural/save schema changes or ordinary default execution changes. No epsilon-based no-effect exception.

Fixed regressions, not autonomous lives: paired translated coordinates X=0/X=3 (plus a larger-coordinate control); real CPU Move+Consumed toxic meal with exact ordinary credit/state; early goal arrival with genuinely positive remaining time; full and half-speed movement; failed/blocked attempts; initial and swept contact; carrying; flat/curved terrain acceptance; diagonal and vertical free movement; no-contact parity; no phantom reflex effort. Keep V19's current conservative curved-prefix rejection as a separately documented issue, not silently redesign it here.

Freeze source before native tests. Copy each executable privately before build target reuse. Publish source and finite scalar evidence only.
