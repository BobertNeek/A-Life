# Model and Reasoning Allocation

Use expensive reasoning where architectural damage is possible. Use cheaper capacity for mechanical docs/fixtures after APIs stabilize.

- GPT-5.5 High or Pro High: G00-G16 except docs-only helpers, G18-G19, G21-G24, any save/load/GPU/Bevy/runtime/security/release gate.
- GPT-5.5 Medium: bounded implementation after schema is fixed, simple UI/docs, small fixtures.
- Codex Spark: read-only repo mapping, docs cleanup, mechanical fixture generation, command/path checks.
- Never use Spark/low reasoning for core runtime, action arbitration, persistence schemas, GPU correctness, teacher/semantic safety boundaries, merge conflict resolution, or final release gates.
