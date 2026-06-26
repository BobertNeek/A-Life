# llama.cpp Runtime Migration Tasks

1. Audit current active Ollama references.
   - Done when code, manifests, docs, and tests with active Ollama references are listed.
   - Verify with `rg -n "Ollama|ollama|ollama-localhost" crates docs examples scripts`.

2. Replace semantic crate runtime client.
   - Target: `crates/alife_semantic`.
   - Done when llama.cpp config, localhost validation, embeddings, chat parsing, and tests compile.
   - Verify with `cargo test -p alife_semantic --features local-llamacpp`.

3. Replace app smoke wrappers.
   - Target: `crates/alife_game_app`.
   - Done when compatibility and new llama.cpp smoke commands route to llama.cpp and summaries report llama.cpp fields.
   - Verify with focused app tests and smoke commands.

4. Update manifests, scripts, and docs.
   - Target: `examples/model_manifests`, `docs/creatures_agi_roadmap_pack`, `scripts`.
   - Done when active docs mark Ollama superseded and no model/binary files are tracked.
   - Verify with scans and docs check.

5. Validate and review.
   - Done when focused smoke, full validation, R2 review, post-merge validation, and push complete.
