# R41 saved-life evaluator

This branch preserves the exact source of the active saved-life evaluator and the disclosed portion of the original R41 scorecard. It does not contain a trained checkpoint, withheld case data, private runtime paths, or private artifact links. The source overlay is anchored to this branch's public parent commit. `SOURCE.json` pins all 827 files byte-for-byte, including the active self-grip extension. The historical full-card 92.5 is not a fresh measurement on this branch.

## Reconstruct and build

Use the branch checkout as the Git source and restore into a new sibling directory, not inside the checkout. The script checks the anchored commit, patches and full source closure.

```sh
python3 docs/r41-saved-life/restore_source.py "$PWD" ../r41-exact-source
cd ../r41-exact-source
export CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_INCREMENTAL=false
cargo build --frozen --offline --release --target x86_64-unknown-linux-gnu -j 1 -p alife_game_app --features foundation-training,gpu-tests --bin evaluate_saved_life_retention
```

Use the matching pinned Rust/Cargo registry and Linux Mesa lavapipe runtime from the project's private restore kit. The proven active ELF SHA-256 is `588b60942ded9959eef5860e96bf546b2acd05a71fe6a6d77d9296304ff22bf6`; verify it before using a prebuilt binary. Rebuilding elsewhere need not produce that identical ELF, so qualify a rebuilt binary with the known control, not merely a successful compile. The exact source entry point is `crates/alife_game_app/src/bin/evaluate_saved_life_retention.rs`; manifest/case plumbing is `crates/alife_game_app/src/foundation_training_cycle/saved_life_retention/{experiments,card_cases,case_room}.rs`.

## Reopen and compare (native game, software Vulkan)

The project restore bundle supplies the authentic mature R41 Current at tick 3074. Verify its SHA-256 against `control-example.json`. Set absolute paths to the runner, input and manifest under an admitted scope, the vetted Mesa library directory and genuine lavapipe ICD JSON. Give each arm a separate private working home and output directory that does not already exist. `control-example.json` is the known control configuration; verify/renew the run identity, nonce, and host execution deadline according to the experiment admission process without changing the matched scenario settings.

```sh
mkdir -p "$RUN_HOME/tmp" "$RUN_HOME/cache" "$RUN_HOME/runtime"
chmod 700 "$RUN_HOME/runtime"
env HOME="$RUN_HOME" TMPDIR="$RUN_HOME/tmp" XDG_CACHE_HOME="$RUN_HOME/cache" XDG_RUNTIME_DIR="$RUN_HOME/runtime" \
  ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS=0 ALIFE_EXPLICIT_SOFTWARE_RETENTION_TEST=1 \
  ALIFE_SLM_PRIOR=off ALIFE_SLM_PRIOR_BACKEND=off LD_LIBRARY_PATH="$VETTED_MESA_LIB" \
  VK_DRIVER_FILES="$VETTED_LAVAPIPE_ICD" "$RUNNER" --experiment-manifest \
  "$SCOPE_ROOT" "$AUTHENTIC_CURRENT" "$NEW_OUTPUT_DIR" "$MANIFEST"
```

Run control and challenger with the same executable, input Current, scenario setup, ticks/decisions, and prior state. The only experimental difference should be the declared intervention. Validate the produced retention receipt and rows, not merely a launch or a newly spawned creature.

## What the 92.5 actually depends on

The recorded score used common live-game report-card cases on the authentic saved-life evaluator, not newborn pilots. The account restored world/entity and body state plus the resident learned brain, including active actor/value, dynamic neural and lifetime/fast plastic state, memory and structure including V11 branch data; the semantic prior was forced off for the pinned shared-evaluator protocol. Foundation weights plus V11 branches do **not** identify that complete input. No component-removal ablation establishes how many points any particular lifetime delta, memory bank, or structural feature contributed. `scorecard-r41-92.5-disclosed.json` records the score denominator, exact domain weights and evidence boundary; the 29 case definitions and complete published navigation setups are alongside it. The withheld case remains withheld and gets no imputed outcome.
