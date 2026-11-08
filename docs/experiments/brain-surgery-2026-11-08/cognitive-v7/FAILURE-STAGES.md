# Preserved failure stages, sanitized

1. Initial literal legacy-wire fixture assumed schema 1. Native output showed existing schema 2. Corrected the old literal and chose distinct local schema 3 for interruption; malformed new states cannot use legacy migration.
2. First CPU run passed the new real interrupt/resume fixture but failed unchanged quiet exact-checkpoint comparison because the new end-of-interval loop rewrote the sleep-seal timestamp without a transition. Fixed production to reseal only actual transitions. The guard was not weakened. Raw failure printed whole synthetic checkpoint arrays, so it remains local and is excluded here.
3. Independent source review found that restoring an interrupted checkpoint and calling ordinary step could latch arousal because release was inside the opt-in branch. Fixed state-authoritative release for already-interrupted residents and added actual restored ordinary-mode exact-continuation coverage.
4. Strict Clippy found a redundant borrow in the existing duplicate-credit fixture. Removed it; final full CPU 32 run and strict Clippy passed.

No failed attempt is counted as a passing result. These tests used fresh fixed fixtures, not private retained organisms.
