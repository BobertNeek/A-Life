# Cost and Mode Policy

Use cheaper/lower reasoning only for bounded tasks:
- docs-only formatting,
- manifest updates,
- search/inventory,
- no runtime code,
- no public API changes.

Use High/Extra High for:
- alife_core changes,
- GPU runtime changes,
- school/semantic safety boundaries,
- save/load schemas,
- release gates,
- review gates,
- architecture decisions.

Use subagents/worktrees only when the plan is explicitly parallel-safe. Most CA plans are sequential because gameplay, UI, and evidence interact.
