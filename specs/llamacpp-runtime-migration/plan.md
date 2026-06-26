# llama.cpp Runtime Migration Plan

## Review Class Target

R2 separate reviewer pass. The reviewer will inspect the diff, active Ollama removal, localhost restrictions, tests, docs, and validation evidence.

## Architecture

- `crates/alife_semantic` owns the local llama.cpp clients and bounded output validation.
- `crates/alife_game_app` owns product smoke summaries and CLI commands.
- `docs/creatures_agi_roadmap_pack` records active model runtime status.
- `scripts/` owns local Windows launcher helpers for llama-server.

## Implementation Steps

1. Rename the active `local-ollama` feature/module surface to llama.cpp.
2. Update manifest structs from Ollama model names to llama.cpp aliases, ports, and endpoints.
3. Implement OpenAI-compatible `/v1/embeddings` and `/v1/chat/completions` parsing.
4. Add strict localhost-only endpoint validation and unavailable behavior.
5. Update app smoke summaries and formatters.
6. Add llama.cpp commands and startup scripts.
7. Update active docs and add a migration report.
8. Run focused smoke, full validation, R2 review, and merge only if clean.

## Risks

- llama-server may not be installed. Mitigation: install or record `USER_ACTION_REQUIRED`; do not fall back to Ollama.
- llama.cpp chat output may include fenced JSON or prose. Mitigation: request JSON, then parse only bounded JSON object content and reject malformed output.
- Long model load time may exceed short command timeouts. Mitigation: explicit high smoke timeout but bounded config limits.

## Rollback

Revert the migration commit. No model files or runtime binaries are committed.
