# S06 Balance And Ecosystem Report

Status: implemented on `codex/S06-nonscripted-ecology-balance`.

S06 strengthens the existing G19 long-run balance smoke by making the non-scripted behavior criteria explicit in code, tests, and this report. The change does not tune core cognition contracts or claim that the current smoke loop has full emergent foraging. It records the current evidence honestly: deterministic resource cycling is the clearest autonomous ecology signal, while creature-level free foraging and hazard avoidance remain bounded smoke or not-yet-emergent.

## Scope

- Owned path: `alife_game_app` balance metrics/reporting and productization docs.
- No changes to `alife_core`.
- No GPU, graphics, save/load, or release-claim changes.
- Fast CI remains bounded and deterministic.
- Extended balance evidence remains manual/ignored.

## Current Fast Smoke Evidence

Command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
```

Expected fast smoke signature shape:

```text
G19 long-run balance schema=alife.g19.long_run_balance.v1 version=1 cycles=3 survival=0.750 energy=0.953 food=1.000 hazard_avoidance=0.200 sleep=1 births=1 social=1.000 sealed=19 population_bound=true resource_bound=true
```

The smoke stays intentionally small. It checks survival, energy stability, food success, hazard penalty visibility, sleep/rest, reproduction bounds, social diversity, sealed-patch learning boundaries, population caps, and resource caps.

## S06 Non-Scripted Criteria

| Criterion | Status | Evidence | Limitation |
|---|---|---|---|
| Food seeking | bounded-smoke | Food action succeeds and lowers hunger through the validated visible-food path. | Not yet proof of open-ended free foraging. |
| Hazard avoidance | not-yet-emergent | Hazard pain remains visible in the smoke metrics instead of being hidden. | Hazard contact is still scripted in the fast loop. |
| Sleep and rest | bounded-smoke | Sleep/rest path is exercised through sealed patches. | Rest is still a bounded smoke-path event. |
| Reproduction and death bounds | bounded-lifecycle | Births and blocked reproduction are reported against a population cap. | Population pressure is deterministic and not yet tuned for broad player fun. |
| Resource stability | autonomous-ecology-signal | Ecology reports deterministic resource regrowth/spawn and active resource bounds. | This is fixture ecology, not a full open ecosystem. |
| Social diversity | bounded-smoke | Population/social loop reports social samples, collisions, and vocal-token diversity. | Social variety remains bounded and perception/modulatory only. |

## Degenerate Behaviors

- Hazard contact is still scripted in the smoke loop; S06 keeps the pain metric visible instead of hiding it.
- Creature-level free foraging is not yet emergent; food success currently proves the validated visible-food path.
- Manual upper population tiers remain expected-slow and are not normal CI gates.
- The fast balance smoke proves bounded deterministic loops, not full player fun across every ecology.

## Extended Manual Evidence

Manual command:

```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

The extended run is intentionally ignored for normal CI because it is a larger balance exercise. It should be used when evaluating future tuning changes, not as a hidden release claim.

## Invariant Status

- `alife_core` remains engine-independent and untouched by S06.
- Headless CPU remains the correctness oracle.
- Learning-relevant values remain validated through existing finite scalar checks.
- Population and resource bounds are explicit in fast smoke metrics.
- No unsealed learning path is accepted.
- GPU and graphics evidence remain unchanged by this plan.

## Recommendation

Proceed to S07 only after S06 is reviewed and merged. Future tuning should improve creature-level free foraging and hazard avoidance with new evidence, not by weakening metrics or editing core contracts to force desired behavior.
