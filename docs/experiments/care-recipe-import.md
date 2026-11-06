# Import a model-authored care recipe

The offline recipe importer lets a user or an existing Codex agent author JSON and
compile it into the same bounded care-sampler bank used by the provider client.
It does not start Codex, call a model, claim authorship, or run training. An
orchestrator must separately obtain the recipe and pass its exact JSON file.

```sh
python3 scripts/import_care_scenario_recipe.py --recipe recipe.json --host host.json --output new-bank.json
```

The recipe has the schema, wall-map legend, goal and fixed seed-roll contract
described in `care-frontier-sampler.md`. The host owns the budgets, difficulty,
family IDs, pool seeds and holdouts. The same host/compiler checks apply: malformed,
unsafe, unreachable or unsupported layouts are rejected, not repaired or
silently rerolled. Native movement and assessment remain authoritative.

The importer accepts bounded regular files, does not overwrite an output, and
prints SHA-256 hashes of the exact recipe, host and generated bank bytes. It does
not certify the producer or establish solver competence. Use stable directories
under your control; parent-directory replacement races by another process are not
protected. Do not place credentials or private checkpoints in recipe or host JSON.
