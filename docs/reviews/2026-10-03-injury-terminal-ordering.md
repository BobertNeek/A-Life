# Biology follow-up: injury and terminal ordering — 2026-10-03

Source baseline: remote `main` at
`2580eec837931624726685f153e053b99fb7a561`, independently checked with fetch
and `ls-remote` before work. Candidate branch:
`phase3-biology-continuity-followup`. The integration owner retains merge
authority. This follow-up covers the shared CPU body/world transition; it does
not claim GPU learning or gameplay evidence.

Root/core/world/docs AGENTS, the relevant v2.0 biology requirements, and the
repository Spec Loop Ponytail review guidance were consulted. The original
checkout and other worktrees were preserved. Relevant requirements are
AOA-BIO-001, AOA-BIO-011, AOA-BIO-015 and AOA-BIO-025/026: authoritative body
state, reserve accounting, derived drives, and measured biological consequences.
The controlling architecture does not specify a new essential-organ death rule.

## P1 — Repair masks globally fatal injury before the death check

**Verified through a canonical founder and registered motor action on the
baseline.** Put the existing `hazard-01` at the first founder's position and
issue Idle through the production motor bundle boundary on each interval. The
hazard causes damage on every interval. At tick 300 the founder remains alive
with health `0.000058100006` and energy `0.7840379`.

The baseline applies damage and then repairs each organ within the same loop:
[damage and repair](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_core/src/biochemistry.rs#L174-L248).
The canonical founder has an inherited OrganRepair nominal of `0.02`:
[founder calibration](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_world/src/new_game.rs#L230-L245).
Circulatory and NeuralSupport repair on every tick:
[organ cadences](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_core/src/biochemistry.rs#L124-L127).
The world checks terminal health only after that biology transition:
[terminal check](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_world/src/headless.rs#L1223-L1261).

The actual live runtime calls this same registered motor boundary:
[production caller](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_game_app/src/gpu_live_runtime.rs#L4816-L4836).
It merges real hazard contact into the motor body event before advancing the
organism once:
[physical consequence and biology](https://github.com/BobertNeek/A-Life/blob/2580eec837931624726685f153e053b99fb7a561/crates/alife_world/src/headless.rs#L3659-L3694).
This is reachable production physiology, rather than a permissive standalone
core call or stale assertion.

**Creature-visible consequence:** sustained damage can exhaust every organ's
integrity while ordinary repair restores a sliver of projected health before
the world can mark the creature dead. This delays injury death while repair
signals and usable local reserves persist. Energy and age remain terminal
causes; the reproduction does not establish unconditional immortality.

**Smallest repair:** apply the interval's damage to all six organs first. Allow
ordinary repair only when some post-damage organ integrity remains. Keep the
existing inherited repair amounts, local reserve limits and costs. An individual
organ at zero integrity can still recover while another organ survives. Gate
both chemical and legacy sleep-repair paths. A pre-existing globally terminal
body likewise cannot revive through basal repair before the next world death
check. No essential-organ rules or resuscitation mechanism are introduced.

The implementation changes only
[BodyState::apply_event](../../crates/alife_core/src/biochemistry.rs#L172-L235).
It adds one bounded six-organ damage pass and one viability scan. Canonical
founders, genomes, drive/chemical ABIs, GPU cognition, lifetime learning and
acquired-state persistence are unchanged. The final post-state remains the
ordinary measured biology used by experience sealing. No persisted field or
schema changes.

## Reproduction and verification

The desired-behavior world regression failed on the exact baseline with the
health/energy/lifecycle values above. A separate core regression also failed:
otherwise globally fatal damage recovered to health `0.727` with an explicit
repair signal, demonstrating the ordering in isolation. The latter uses a
strong test signal, not canonical founder calibration.

With the repair, the same founder dies of zero health at tick **96**, with energy
still **`0.7902301`**. The terminal interval retains its actual biological
consequence rather than buying a positive health floor through basal repair.

Focused tests cover both repair paths, already terminal bodies, and recovery of
a locally zero-integrity organ in a surviving body with the exact reserve cost.
The world regression uses the ordinary canonical constructor and registered
motor biology rather than editing body projections or reproducing a separate
damage implementation:
[regression](../../crates/alife_world/tests/biology_injury_terminal.rs).
An additional JSON portable-save regression checkpoints the injured founder at
tick 80, compares every subsequent world signature with uninterrupted execution,
then restores the dead state and verifies that later world advancement cannot
revive it. Save summaries follow the existing canonical-save pattern; the
fixture supplies the constructor's creature records and current biology.

All Cargo work is CPU-only, serial, with `CARGO_BUILD_JOBS=1` and one test thread.
Final outcomes and the publication commit are provided in the handoff receipt.

```sh
cargo test -p alife_core --lib biochemistry::tests::
cargo test -p alife_world --test biology_injury_terminal -- --nocapture
cargo test -p alife_core --lib --test biochemical_graph_v2 --test biochemical_material_baselines --test biochemical_reaction_conservation --test biochemical_valuation --test biochemistry_development --test newborn_nociception
cargo test -p alife_world --lib --test save_load_roundtrip --test canonical_new_game --test task_3_2a_atomic_action --test world_organism_registry_persistence --test v11_cognitive_biology --test biology_audit_taste --test headless_world_signature
cargo clippy -p alife_core -p alife_world --all-targets -- -D warnings
cargo fmt --all -- --check
bash scripts/check.sh --quick
```

A separate read-only reviewer checked the two-pass gate and regressions and
found no blocker, including both repair branches and paid local-organ recovery.

All **224 selected CPU tests pass**: 71 core, 151 existing world tests and two
new injury/restore regressions. The existing manual CPU topology timing probe
remains ignored. This includes world late-failure rollback, fresh readiness and
terminal-mating cancellation, current-age death, exact registry restoration,
canonical construction, taste continuity and measured biology consequences.
No test replaces production cognition with a live CPU neural policy.
Core/world all-target Clippy with warnings denied, workspace formatting, staged
diff whitespace checks, core boundaries and all 77 documentation assertions
also pass. R2 review is closed with no blockers.

## Audited boundaries and remaining coverage

The follow-up traced ingestion into nutrition/energy/damage, local organ
accounting, repair receptors, derived drives, terminal checks, biological age,
mating readiness revalidation, and save/restore ownership. Existing taste,
current-age development, prediction-residual and social-recognition repairs are
present on this baseline. They are not reported again as open bugs. Existing
reaction conservation and material-baseline fixes are likewise preserved.

The registered motor path checks effector capability before ingestion. A direct
Eat helper lacking that check is not a production bypass. The world advances
biology once per interval; permissive direct core equal-tick or long catch-up
calls do not establish a live duplicate-event or retroactive-repair bug.
Sleep is not nutrition, repair still spends reserve, and cognitive work has its
separate measured debit. Reproduction checks fresh post-transition readiness;
offspring do not inherit acquired weights or memories.

Combining locomotion and Rest is reachable: their events are merged into one
body interval, positive sleep recovery sets sleeping and applies the inherited
sleep upkeep rate. Whether a move-then-rest interval should be permitted is
**Unknown**. Source does not settle the gameplay policy, so this patch does not
add a cross-channel restriction. A future rule should account for actual
successful displacement and preserve rest after failed movement.

GPU causal injury avoidance/learning, lifetime survival rates, death archival
through a running GPU resident, and acquired neural checkpoint restoration were
not exercised. No training, GPU, desktop, merge, or review-bot request occurred.
