# Execution order and concurrency map

This map is the intended way to use the plan pack with Codex. Do not ask Codex to start random plans. Start at P00. Then follow the dependency graph below.

## Group 0 - Baseline and governance, serial

Run these in order on `main` or a short-lived integration branch:

1. P00 - Operating model, repo audit, and plan wiring.
2. P01 - Scaffold cleanup, workspace hygiene, portable hooks, CI baseline.
3. P02 - Spec traceability matrix, decision/progress logs, invariant test harness.
4. P03 - Core IDs, math primitives, units, stable adapter boundary.
5. P04 - ABI versioning, validation framework, error model.

Do not branch widely before P04. The goal is to make future branches easy to merge.

## Group 1 - Core contracts, partially parallel

After P04, these can run concurrently in separate branches because they mostly own separate modules:

- P05 `codex/P05-brain-lobes-routing`
- P06 `codex/P06-genome-weight-split`
- P07 `codex/P07-drives-hormones`
- P08 `codex/P08-sensory-contexts`
- P09 `codex/P09-action-arbitration`

Merge P05-P09 into an integration branch, resolve conflicts, run all core tests, then run:

- P10 - Three-phase ExperiencePatch runtime contract.
- P11 - Packed logging and side buffers.

After P10, these can run concurrently:

- P12 `codex/P12-memory-expectancy`
- P13 `codex/P13-topological-map`

After P05, P06, P07, and P08 merge, P14 can start:

- P14 `codex/P14-neural-state-projection-schema`

Then run serially:

- P15 - CPU reference brain tick loop.
- P16 - Sleep consolidation and structural editing.

## Group 2 - Harness, adapters, and tools, parallel after core merge

After P15, run these in parallel branches:

- P17 `codex/P17-headless-world-harness`
- P21 `codex/P21-bevy-avian-adapter`
- P23 `codex/P23-school-teacher`

After P11, P30 can start independently:

- P30 `codex/P30-offline-log-tools`

After P17, run:

- P18 - Scenario suite.
- P19 - Golden traces, property/fuzz tests, determinism.
- P20 - Benchmark harness and performance tiers.

## Group 3 - GPU backend, serial with parity gates

After P14 and P15, run GPU work in this order. Do not parallelize these unless a human integrator is actively coordinating shader schema changes.

1. P24 - GPU buffer layout and shader contract translation.
2. P25 - Static GPU forward passes and CPU parity.
3. P26 - GPU plasticity pass, fixed-point/Oja, weight split.
4. P27 - Super-tile culling, routing, active masks.
5. P28 - Structural recompaction, autophagy, sleep GPU path.
6. P29 - No-readback runtime integration and GPU performance tiers.

## Group 4 - Research and evolution tools, parallel after logs/core

After P30 and the core contracts are stable:

- P31 `codex/P31-etf-neural-collapse-metrics`
- P32 `codex/P32-d2nwg-weight-generator`
- P33 `codex/P33-evolution-genome-lab`

P31 and P32 are optional research lanes. They must never be required for normal runtime startup.

## Group 5 - Product integration and release, mostly serial

After P17-P29 are stable enough to run the playground:

1. P34 - Save/load, schema migration, configs, assets.
2. P35 - Full playground, examples, UX docs.
3. P36 - Production hardening, soak tests, release gate.

## How to run subagents without handholding

Use one Codex session per branchable plan. Give each subagent the master prompt and the single plan file. Tell it to edit only the files listed in that plan. When a subagent finishes, merge into an integration branch in dependency order, run the validation commands, and then ask Codex to continue with the next unblocked plan shown in the plan file.

## Merge gate rule

A plan branch may be merged only if:

- Its completion receipt is present.
- It ran or explained all required validation commands.
- It did not violate global invariants.
- It updated traceability and progress files.
- It names the exact next plan(s).
