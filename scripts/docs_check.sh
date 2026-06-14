#!/usr/bin/env bash
set -euo pipefail

test -f docs/master_spec.md
test -f docs/architecture_decisions.md
test -f AGENTS.md
test -f docs/AGENTS.md
test -f docs/release_checklist.md
test -f docs/final_status_report.md
test -f docs/gpu_soak_performance_plan.md

for crate_agents in crates/*/AGENTS.md; do
  test -f "${crate_agents}"
done
