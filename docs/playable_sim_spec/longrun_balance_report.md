# G19 Long-run Balance Report

Schema: `alife.g19.long_run_balance.v1`

G19 adds a deterministic headless balance smoke that composes the existing G06, G07, G08, G09, and G18 product loops. It records first-pass balance metrics without changing `alife_core` contracts, GPU policy, save/load schemas, or action arbitration.

## Fast CI Smoke

Run:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
cargo test -p alife_game_app --test app_shell longrun_balance
```

The fast smoke tracks:

- survival score
- energy stability
- food success rate
- hazard avoidance score
- sleep cycles
- reproduction births and blocked reproduction
- social diversity
- sealed patch and packed log counts
- population and resource caps
- invalid-ID rejection
- finite value validation
- no unsealed learning

## Extended Manual Run

Extended balance is intentionally not part of normal CI:

```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

The manual run uses the same deterministic report path with a larger cycle count. It is useful before tuning releases, but it remains an ignored/manual command so normal validation does not gain an unbounded long-run test.

## Known Degenerate Behaviors

- The fast hazard path is still scripted. G19 keeps the pain/hazard metric visible instead of hiding it behind a pass/fail label.
- Upper population tiers remain expected-slow and manual unless later hardware evidence makes them portable CI gates.
- The current smoke proves bounded deterministic loops and plausible metrics. It does not prove that every ecology is fun across arbitrary player-created worlds.

## Constraints

- CPU/headless remains the correctness oracle.
- G18 LOD preserves sensory, motor, homeostasis, and action arbitration priority.
- CPU fallback reports are not GPU performance claims.
- Population/resource caps are finite and validated.
- Balance tuning must not overfit golden traces or mutate core invariants.
