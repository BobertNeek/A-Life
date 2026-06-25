# Automated Development Loop

This pack allows bounded automation, not unattended product claims.

## Allowed automation

Codex may proceed sequentially between review gates if:
- the plan passes,
- validation passes,
- no blocked decisions occur,
- no forbidden scope appears,
- no human evidence is required.

## Hard stops

Codex must stop at:
- every `CARxx` review gate,
- any failed validation it cannot fix locally,
- any plan requiring new external human evidence,
- any release/tag decision,
- any change to alife_core public contracts unless the plan explicitly owns it,
- any request to remove CPU shadow gating,
- any attempt to create S12/G25/P37.

## Maximum run size

In Goal Mode, run at most three implementation plans before producing a progress receipt, even if no review gate appears.

## Receipts

Every plan receipt must include:
- Plan ID
- Branch
- Files changed
- Runtime code changed
- Core API changed
- Public APIs changed
- Tests
- Commands
- Results
- Invariants
- Known limitations
- Next plan
- Stopped/continued
