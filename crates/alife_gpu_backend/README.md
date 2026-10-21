# alife_gpu_backend

This crate owns wgpu resources and sparse WGSL neural execution. The shared
`alife_runtime` session connects consumers to it; the world owns perception,
legality and measured outcomes. Device loss stops gameplay neural actions.
Opt-in offline CPU training remains a separate execution path.

## Neural execution

[`closed_loop_buffers`](src/closed_loop_buffers.rs) packs compiled phenotypes,
perception, memory and receptor effects into class-bucketed heaps.
[`closed_loop_pipeline`](src/closed_loop_pipeline.rs) runs encoding, recurrent
projection and decoding. Generation-checked handles bind each resident and its
uploads to the correct slot.

[`closed_loop_activity`](src/closed_loop_activity.rs) owns activity masks and
work accounting for the actual sparse graph. The pipeline uses phenotype-owned
counts, offsets and layouts rather than a fixed global brain size.

## Learning and sleep

A waking dispatch stages candidate-specific recurrent and decoder eligibility.
The matching sealed world outcome supplies three-factor credit through compiled
receptors. Oja normalization is gated by credited eligibility, so ordinary
zero-credit activity preserves genetic and lifetime weights. The inactive fast
weight and eligibility banks become active together after the learning commit.

[`closed_loop_learning`](src/closed_loop_learning.rs) owns these transactions.
[`closed_loop_sleep`](src/closed_loop_sleep.rs) and
[`closed_loop_v11`](src/closed_loop_v11.rs) own sleep consolidation and mutable
causal state. A pending learning transaction must be committed or explicitly
discarded before another tick or retirement.

## Capture and diagnostics

Selection and learning return bounded results. Explicit checkpoint and exact
population capture paths preserve full acquired and private state at sealed
boundaries. See [`closed_loop_checkpoint`](src/closed_loop_checkpoint.rs) and
[`closed_loop_exact_capture`](src/closed_loop_exact_capture.rs).

Real GPU measurements use selected classes, profiles, populations and explicit
performance targets. See [Development](../../docs/DEVELOPMENT.md#gpu-measurements)
for commands. Host fixtures and estimates do not establish device timing or
learned behavior; record missing hardware and measurements directly.
