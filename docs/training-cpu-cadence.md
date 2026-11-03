# Opt-in CPU cadence replay

Reference remains the default. `cadence-csr` selects phase-specific incoming-edge
indices for offline CPU forward replay and recurrent BPTT. Canonical edge indices,
relative accumulation order, weights and Adam state are unchanged. Collection
remains Reference. The mode is process-local and is absent from checkpoints.
This does not select a CPU backend for GPU care or accelerate GPU PPO.

Build the headless CPU care executable:

```sh
cargo build --release --locked -p alife_game_app --no-default-features --features cpu-training --bin train_cpu_care
```

A disposable public fixture (choose an unused output directory):

```sh
ALIFE_SLM_PRIOR=on ALIFE_SLM_PRIOR_BACKEND=deterministic cargo run --release --locked -p alife_game_app --no-default-features --features cpu-training --bin train_cpu_care -- --fixture --output /tmp/alife-cadence-care-70703 --seed 70703 --lesson eat_held_food --decisions 16 --replay-execution cadence-csr
```

For an admitted CPU continuation, `train_cpu_care` supports the same flag with its
normal `--previous`, `--seed-history`, explicit seed and lesson arguments. Existing
source admission, full actor/value Adam checks, private collection archives and
seed reservations still apply. This is fresh prepared-cohort training; it does not
restore a previous terminal GPU individual.

`scripts/run_cpu_care_serial.py` accepts `--replay-execution cadence-csr` when
creating a new plan. The selected mode is pinned in that plan and checked against
native output receipts, including on resume. Old plans imply Reference; their
source, helper and executable pins must not be edited to switch them. A retained
lineage needs its owner's normal source/history reconciliation and a new admitted
plan before selecting a newly built binary. This change does not attest any
particular retained lineage's compatibility.

The simpler CPU foundation fixture is also runnable:

```sh
cargo run --release --locked -p alife_training --features cpu-backend --bin train_cpu_foundation -- --fixture --output /tmp/alife-cadence-foundation-70702 --seed 70702 --ticks 16 --replay-execution cadence-csr
```

Total elapsed time includes index construction and post-update rebuilding;
`update_seconds` does not include those two costs. Compare full matched runs before
claiming a production gain. Earlier replay-component measurements are pinned in
`docs/performance/20261103-cpu-cadence-opt-in.json`; they do not establish current
care episodes/hour, GPU throughput or learning per hour.
