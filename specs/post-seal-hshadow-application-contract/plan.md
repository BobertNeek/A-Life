# Post-Seal H_shadow Application Contract Plan

## Architecture Touch Points

- `alife_core`: new versioned delta types and `CreatureMind` application method.
- `alife_gpu_backend`: convert P26 diagnostic output into core delta batches.
- `alife_game_app`: apply deltas after sealed ticks in static-plastic mode.
- `docs/productization`: update runtime/gap reports.

## Implementation Plan

1. Add core post-seal lifetime delta module and exports.
2. Add initialization-only projection schema loading to `CreatureMind` so the
   product GPU smoke can use the same validated sparse fixture as the backend.
3. Add H_shadow target traversal and application helpers over existing
   `NeuralProjectionSchema` tiles.
4. Add core tests for acceptance and rejection paths.
5. Extend GPU full runtime plasticity report with core delta records and batch
   conversion.
6. Update app smoke to install the validated fixture schema, disable CPU Oja for
   the GPU-plastic tick, apply the post-seal batch, and report the receipt.
7. Update reports and validation evidence.

## Review Target

R2 separate review pass required before merge.

## Verification

Focused tests first, then full validation with Windows PowerShell wrappers. No
plain `bash scripts/check.sh`.

