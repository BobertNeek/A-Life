# Full GPU Neural Runtime Loop State

## Current Cycle

Cycle 1: implement the smallest optional GPU static scoring path plus honest plasticity gap reporting.

## Stop Conditions

- Stop if active GPU scoring cannot use compact readback only.
- Stop if implementation requires changing `alife_core` public contracts.
- Stop if validation reveals a dependency leak or no-readback violation that cannot be repaired locally.
- Stop after one reviewed, validated implementation branch and receipt.

## Current Decision

Action-authoritative full GPU plastic runtime is not safe without a core hook to apply `H_shadow` updates. The branch will implement CPU-shadow-guarded GPU static action scoring and diagnostic/shadow GPU plasticity evidence.

