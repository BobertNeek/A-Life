# A-Life Alpha Playtest Runbook

This runbook prepares an external alpha playtest for the current A-Life
playable-sim candidate. It is not a release process, does not create a release
tag, and does not authorize a public release claim.

## Scope And Boundaries

Current classification: alpha / external playtest candidate.

Supported evidence path:

- headless CPU playground
- deterministic product smoke suite
- P34 tiny world fixture/config/assets
- S10/S11 alpha evidence and decision docs

Manual evidence paths:

- persistent graphical playground on local graphics hardware
- GPU runtime/performance with explicit hardware validation flags
- screenshots, videos, and tester observations

Not allowed during alpha evidence collection:

- no release tag
- no S12, G25, P37, or hidden implementation chain
- no GPU performance claim from CPU fallback
- no broad player-ready claim from smoke tests alone
- no committed screenshots, videos, logs, benchmark artifacts, captures, or
  hardware dumps

Store local evidence under `target/playtest_evidence/alpha/` and keep it
untracked.

## Clean Clone / Build / Run

From a clean Windows machine with Rust installed:

```powershell
git clone https://github.com/BobertNeek/A-Life.git
cd A-Life
git rev-parse --short HEAD
cargo check --workspace --all-targets
```

If testing a specific commit, check it out explicitly before running evidence:

```powershell
git fetch origin
git checkout VALIDATED_MAIN_SHA
```

Replace `VALIDATED_MAIN_SHA` with the exact commit SHA approved for the
playtest.

## Windows-Safe Validation Commands

Run from the repository root. Do not run plain `bash scripts/check.sh` on
Windows; it may invoke WSL instead of Git Bash.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
```

Optional broader validation before any tag decision:

```powershell
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

## Headless And Playground Commands

These commands should pass without graphics or GPU hardware:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
```

Expected result:

- commands exit successfully
- output reports the headless CPU/playground path
- `release-candidate-smoke` reports no release tag created
- generated outputs remain under `target/` and untracked

## Graphical Command

Dry-run command wiring:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

Real manual graphical playtest:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

The real graphical command is manual playtest evidence only if a tester actually
observes the A-Life window. Record:

- whether the window opened and stayed interactive
- the window title
- visible creature marker or placeholder
- visible food, hazard, or resource markers
- stable ID/debug text or overlay
- CPU fallback/backend status
- pause, step, run/resume behavior
- camera, selection, inspector, and save/load UI behavior
- shutdown/exit behavior

If the graphical command cannot run, record the exact error and classify it as
`MANUAL_EVIDENCE_MISSING` or a finding severity if it blocks the intended test
machine.

## GPU Runtime Command

Use this only when the tester intentionally wants GPU runtime evidence and has
the validation flags set:

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Record:

- hardware identifier if available
- backend selected
- fallback status
- timing or frame-rate report if measured
- bottlenecks or unknowns

If the flags or hardware are not available, the command may honestly record CPU
fallback. CPU fallback is not GPU performance evidence.

## Manual Screenshots And Video

Recommended captures:

- terminal before launch with repo SHA visible
- graphical window immediately after launch
- visible world scene
- creature and food/hazard/resource markers
- inspector/debug overlay
- pause or step state
- save/load UX surface if reached
- GPU/backend status if visible
- clean exit or crash/error state

Store captures outside git, for example:

```text
target/playtest_evidence/alpha/TESTER_OR_RUN_ID/screenshots/
target/playtest_evidence/alpha/TESTER_OR_RUN_ID/videos/
target/playtest_evidence/alpha/TESTER_OR_RUN_ID/logs/
```

## Tester Observations To Record

- what the tester understood first
- what confused the tester
- whether controls were discoverable
- whether game state was understandable without reading logs
- perceived smoothness or frame rate if measured
- save/load success or failure
- crashes, hangs, missing windows, or errors
- whether the tester would call it playable, alpha-playable, or still tooling

Use `docs/productization/ALPHA_PLAYTEST_EVIDENCE_TEMPLATE.md` for the structured
record.

## Finding Severity

- `BLOCKER`: prevents launch or makes the supported product path unusable.
- `HIGH`: prevents a normal external tester from treating it as a game.
- `MEDIUM`: usable but confusing, brittle, incomplete, or poorly explained.
- `LOW`: polish, wording, docs, or minor UX issue.
- `MANUAL_EVIDENCE_MISSING`: cannot judge without hardware, screenshots, or
  playtest evidence.

Do not create GitHub issues unless explicitly instructed later. Record findings
in the evidence template or a local untracked evidence file.

## Artifact Discipline

Before ending a playtest pass:

```powershell
git status --short
git ls-files target dist target/artifacts graphify-out target/playtest_evidence
```

Expected result: generated evidence, benchmark reports, graphics captures, and
package artifacts are untracked and not committed.

## Release Boundary

This runbook does not approve release. It supports the S11 recommendation:
external playtests first. Any release tag requires a fresh explicit user
approval after reviewing the evidence and rerunning validation on the exact SHA.
