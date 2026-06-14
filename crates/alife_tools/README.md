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

## Optional ETF/Neural Collapse tooling (`p31_offline`)

P31 tooling is offline and optional. It generates optional versioned representation
prototype tables and computes neural-collapse style summary metrics from trace data.
It is intentionally not required for normal gameplay or runtime builds.

### 1) Generate ETF simplex prototypes for fixed affordance classes

```powershell
cargo run -p alife_tools --bin p31_offline -- generate `
  --classes 10 --dimension 64 --out target/artifacts/p31_etf_prototypes_v1.json
```

```bash
cargo run -p alife_tools --bin p31_offline -- generate \
  --classes 10 --dimension 64 --out target/artifacts/p31_etf_prototypes_v1.json
```

### 2) Analyze trace-style exported logs

```powershell
cargo run -p alife_tools --bin p31_offline -- analyze-trace \
  --trace crates/alife_world/tests/fixtures/golden_traces/food-seeking.json \
  --prototypes target/artifacts/p31_etf_prototypes_v1.json \
  --out target/artifacts/p31_nc_summary.json
```

```bash
cargo run -p alife_tools --bin p31_offline -- analyze-trace \
  --trace crates/alife_world/tests/fixtures/golden_traces/food-seeking.json \
  --prototypes target/artifacts/p31_etf_prototypes_v1.json \
  --out target/artifacts/p31_nc_summary.json
```

### 3) Emit a versioned sensory lobe asset for P08/P14 consumers

```bash
cargo run -p alife_tools --bin p31_offline -- write-lobe-asset \
  --prototypes target/artifacts/p31_etf_prototypes_v1.json \
  --out target/artifacts/p31_sensory_lobe_etf_v1.json
```

## Evolution genome lab (`p33_genome_lab`)

P33 ships deterministic offline evolution helpers in `alife_tools::p33_evolution`
and a tiny smoke CLI. The lab mutates and crosses valid `BrainGenome` records,
summarizes fitness from packed logs, and keeps optional generated weight assets
as birth-only initializer references.

```powershell
cargo run -p alife_tools --bin p33_genome_lab -- smoke `
  --seed 43981 `
  --generations 1 `
  --out target/artifacts/p33_generation_smoke.json
```

```bash
cargo run -p alife_tools --bin p33_genome_lab -- smoke \
  --seed 43981 \
  --generations 1 \
  --out target/artifacts/p33_generation_smoke.json
```

P33 does not import P32/D2NWG types. If generated weight assets are available,
refer to them through `BirthWeightInitializerRef` with `birth_only: true`.
