You are importing the A-Life S01-S11 productization plan pack.

Do not implement S01 yet.
Do not create P37, G25, or S12.
Do not change runtime code.
Do not tag a release.

Import path:
docs/productization_s_plans/

Copy the provided plan pack there.

Verify it contains:
- README.md
- plan_manifest.json
- GLOBAL_INVARIANTS.md
- VALIDATION_PROTOCOL.md
- EXECUTION_ORDER.md
- PLAYABILITY_ACCEPTANCE_TESTS.md
- plans/S01 through S11
- prompts/GOAL_MODE_S01_TO_S11.md
- prompts/NEXT_PROMPT_S01.md
- roadmaps/NEXT_STAGE_AFTER_S11.md

Optionally update AGENTS.md with one short pointer:
Future post-R24 productization work should use docs/productization_s_plans/ and must not create P37/G25/S12 automatically.

Validation:
Use Windows wrappers. Do not run plain bash scripts/check.sh.

Run:
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core

Commit:
Import S-phase productization plan pack

Receipt:
S-plan import receipt
Files changed:
Runtime code changed:
Plan chain changed:
Commands run:
Results:
Next executable plan: S01
Stopped before S01: yes
