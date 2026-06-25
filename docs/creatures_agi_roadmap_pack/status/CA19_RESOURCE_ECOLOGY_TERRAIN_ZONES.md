# CA19 - Resource Ecology and Terrain Zones

Status: complete on `codex/CA19-resource-ecology-terrain-zones`

CA19 makes the GPU alpha graphical path show persisted ecology instead of only
object markers. The GPU alpha fixture now includes stable-ID terrain zones and
one tracked food resource lifecycle; the CA19 smoke adds a bounded deterministic
sprout policy to its temporary clone to prove spawn indicators without changing
the default live fixture behavior.

Evidence added:

- `graphical-ecology-smoke` restores the portable save, validates terrain-zone
  and resource-cycle data, exercises a bounded consume/regrow/spawn cycle, and
  verifies ecology survives save/load roundtrip.
- The Bevy shell renders display-only terrain-zone markers behind the world:
  green resource-biased zones and red hazard-pressure zones.
- The graphical overlay reports zone/resource counts, regrowth/spawn indicators,
  hazard-pressure zones, stable-ID tracking, and the display-only boundary.

Boundaries:

- `alife_core` was not changed.
- Terrain and resource visuals are presentation only and cannot emit actions.
- The product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU fallback and CPU shadow parity remain preserved.
- No release tag, S12, G25, or P37 was created.
