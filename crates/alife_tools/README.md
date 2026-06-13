# alife_tools

Developer tooling contracts.

This crate is for repository tooling, Graphify/DOX helpers, validation manifests, and future offline utilities. Tooling must remain optional for normal Cargo build/check/test unless a specific validation script invokes it.

## Offline log tools (`p30_offline`)

P30 ships a Rust CLI for offline analysis of packed logs, scenario markers, and benchmark markers.

### 1) Import scenario/benchmark markers and packed records into a stable bundle

```powershell
cargo run -p alife_tools --bin p30_offline -- bundle import `
  --record path\to\packed_records.json `
  --scenario-fixture crates\alife_world\tests\fixtures\golden_traces\food-seeking.json `
  --benchmark-markdown crates\alife_tools\docs\benchmark_markers.md `
  --source "p30 smoke bundle"
```

```bash
cargo run -p alife_tools --bin p30_offline -- bundle import \
  --record path/to/packed_records.json \
  --scenario-fixture crates/alife_world/tests/fixtures/golden_traces/food-seeking.json \
  --benchmark-markdown path/to/benchmark_report.md \
  --source "p30 smoke bundle"
```

`packed_records.json` is a JSON array of `PackedExperienceRecord` (frame + side buffers).

The bundle defaults to:

```text
target/artifacts/p30_offline_bundle.json
```

### 2) Summarize a bundle or packed-record log

```powershell
cargo run -p alife_tools --bin p30_offline -- summary `
  --bundle target/artifacts/p30_offline_bundle.json `
  --cluster-k 4 `
  --cluster-iterations 12 `
  --markdown target/artifacts/p30_offline_summary.md `
  --trajectory-csv target/artifacts/p30_trajectory.csv `
  --action-csv target/artifacts/p30_actions.csv `
  --json target/artifacts/p30_summary.json
```

```bash
cargo run -p alife_tools --bin p30_offline -- summary \
  --bundle target/artifacts/p30_offline_bundle.json \
  --cluster-k 4 \
  --cluster-iterations 12 \
  --markdown target/artifacts/p30_offline_summary.md \
  --trajectory-csv target/artifacts/p30_trajectory.csv \
  --action-csv target/artifacts/p30_actions.csv \
  --json target/artifacts/p30_summary.json
```

You can also summarize direct packed-record JSON with:

```bash
cargo run -p alife_tools --bin p30_offline -- summary \
  --record path/to/packed_records.json
```
