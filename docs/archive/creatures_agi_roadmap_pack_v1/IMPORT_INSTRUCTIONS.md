# Import Instructions

Use this prompt to import the pack into the repository.

```text
Import the Creatures-to-AGI roadmap pack.

Do not implement any plan yet.
Do not create S12, G25, P37, or a release tag.
Do not change runtime code.
Do not change alife_core.
Do not modify historical P/G/R plan files except adding a pointer if necessary.

Copy the uploaded pack to:
docs/creatures_agi_roadmap_pack/

Then run:
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core

Commit:
Import Creatures-to-AGI roadmap pack

Receipt:
Roadmap pack import receipt
Files changed:
Runtime code changed:
Validation results:
Next executable plan: CA00
Stopped before CA00: yes
```
