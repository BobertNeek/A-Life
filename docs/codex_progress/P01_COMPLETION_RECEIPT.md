# Completion receipt

Plan: P01 - Scaffold cleanup, workspace hygiene, portable hooks, CI baseline

Branch: codex/P01-scaffold-cleanup

Files changed:

- Removed tracked duplicate scaffold mirror under `a_life_revised_spec_pack/`.
- Updated `.codex/hooks.json`.
- Updated `scripts/build.sh`.
- Updated `scripts/test.sh`.
- Added `scripts/check.sh`.
- Added `scripts/check_core_boundaries.sh`.
- Added `.github/workflows/ci.yml`.
- Added crate README files under all seven workspace crates.
- Updated `docs/codex_progress/PLAN_PROGRESS.md`.
- Updated `docs/codex_progress/DECISION_LOG.md`.
- Updated `docs/codex_progress/SPEC_TRACEABILITY.md`.
- Added `docs/codex_progress/P01_COMPLETION_RECEIPT.md`.

Public APIs changed:

- None.

Tests added/changed:

- Added `scripts/check_core_boundaries.sh` as a dependency/source boundary validation gate for `alife_core`.
- Added GitHub Actions CI to run format, check, tests, clippy, core boundary checks, and docs checks.

Commands run:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `bash scripts/check_core_boundaries.sh`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check_core_boundaries.sh`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check.sh`
- `cargo clippy --workspace --all-targets -- -D warnings`

Results:

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo test --workspace --all-targets`: passed, including 7 scaffold invariant tests.
- `bash scripts/check_core_boundaries.sh`: failed because Windows `bash` resolves to WSL and WSL cannot start without virtualization in this environment.
- Git Bash substitute `C:\Program Files\Git\bin\bash.exe scripts/check_core_boundaries.sh`: passed.
- Git Bash aggregate `scripts/check.sh`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

Invariant checks:

- `alife_core` remains free of Bevy, Avian, wgpu, renderer, Python, and OS windowing dependencies.
- No Unity, C#, `.csproj`, `.sln`, `.unity`, or HLSL files were added.
- No neural runtime kernels, Bevy runtime work, GPU shader work, SLM work, D2NWG work, or visual playground work was added.
- Graphify remains optional for build/check/test.

Deviations:

- The exact `bash scripts/check_core_boundaries.sh` command cannot run through the default Windows `bash` executable because it invokes unavailable WSL. The closest local substitute, Git Bash, ran the same script successfully.

Known limitations:

- CI has been added but has not run on GitHub yet.
- Future local Windows sessions should call Git Bash explicitly or put Git Bash ahead of WSL `bash` in `PATH` when running shell-script validation.

Next plan(s): P02
