#!/usr/bin/env bash
set -euo pipefail

test -f docs/master_spec.md
test -f docs/architecture_decisions.md
test -f AGENTS.md
test -f docs/AGENTS.md

for crate_agents in crates/*/AGENTS.md; do
  test -f "${crate_agents}"
done
