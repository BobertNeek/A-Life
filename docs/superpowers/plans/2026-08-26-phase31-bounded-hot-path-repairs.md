# Phase 3.1 Bounded Hot-Path Repairs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Batch same-tick sleep promotions into one atomic publication and replace ordinary-seal full neural snapshots with a validated compact GPU authority receipt.

**Architecture:** The durable promotion helper will build and validate one candidate from one immutable pre-state, then invoke publication once. The GPU backend will expose a versioned compact post-commit authority receipt derived from already validated transaction metadata. The game runtime will bind that receipt into the cognitive state graph without reading full mutable GPU state. Exact snapshot paths remain unchanged for manual and sleep checkpoints.

**Tech Stack:** Rust, Cargo, wgpu, serde, existing canonical digest and portable-save infrastructure.

**Spec:** `docs/superpowers/specs/2026-08-26-phase31-runtime-performance-repair-design.md`

## Global constraints

- Do not change sleep checkpoint cadence or add asynchronous checkpoint staging in this tranche.
- Do not weaken sleep, save, GPU, state-graph, or compatibility validation.
- Do not add CPU neural mirrors, full-state GPU hashes, training, or new runtime paths.
- Keep exact `snapshot_brain` use at manual-save and sleep-checkpoint boundaries.
- Leave implementation changes unstaged for supervisor diff review.

---

### Task 1: Batch durable Completed-to-Committed promotions

**Files:**

- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Test: the closest existing focused game-app persistence test target, or a new single-purpose integration target if no existing target can exercise the production helper without widening APIs.

**Step 1: Write the causal RED**

Add a test that supplies two valid same-pre-state promotions and observes one publication call. Add a second case with one invalid member and assert zero publication calls plus byte/state equality of the prior durable save.

**Step 2: Run the focused test and record the behavioral failure**

Run only the selected test target and confirm the existing per-organism implementation publishes twice or lacks atomic batch rejection.

**Step 3: Implement one candidate and one publication**

Replace `promote_durable_completed_sleep` with `promote_durable_completed_sleep_batch`. The implementation must:

1. canonicalize and reject duplicate organism IDs;
2. validate every requested promotion against the same `durability.published.save` pre-state;
3. apply all changes to one cloned candidate;
4. validate the completed candidate;
5. call `durability.publish` once;
6. restore the previous in-memory durability object on every failure.

Replace the tick-loop promotion call with one batch call. Keep per-organism memory compaction and replay cleanup ordered after successful durable publication unless existing validation requires a stricter pre-publication check.

**Step 4: Run the focused GREEN**

Confirm success produces one publish and invalid input produces none with unchanged durable authority.

---

### Task 2: Add the compact versioned GPU authority receipt

**Files:**

- Modify: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs` only if the new receipt needs an explicit export.
- Test: existing focused GPU learning receipt tests.

**Step 1: Write the receipt REDs**

Add pure validation tests for a valid V1 receipt and tampering of schema, handle generation, phenotype, tick, sealed sequence, and transaction generation. Confirm each mismatch fails closed.

**Step 2: Preserve the compact GPU transaction generation**

Add `transaction_generation` to `GpuLearningReceipt` and populate it from the existing 16-word `GpuFastPlasticityCommitRecord`. Do not add a readback.

**Step 3: Implement `GpuAuthorityReceiptV1`**

Construct it only from a validated handle, pending eligibility/sealed selection metadata, sealed patch identity, and the matching `GpuLearningReceipt`. Include an explicit schema version and deterministic canonical digest. Reject zero, stale, cross-handle, cross-phenotype, cross-tick, or cross-sequence inputs.

**Step 4: Run focused backend GREEN tests**

Run only the receipt and learning-commit test filters. Do not run the long physical baseline.

---

### Task 3: Use compact authority on ordinary seals

**Files:**

- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Test: focused production GPU runtime integration target.

**Step 1: Write the causal boundary RED**

Exercise one ordinary production GPU seal and assert it emits a validated compact authority receipt while ordinary snapshot calls and bytes remain zero. In the same focused boundary test, invoke explicit manual checkpoint capture and the existing sleep checkpoint boundary and assert exact snapshot activity remains nonzero there.

**Step 2: Remove only ordinary-seal full-state work**

Replace `commit_sealed_batch` calls to `snapshot_brain` and full `ResidentCognition`/`TopologySidecar` JSON serialization with the validated V1 receipt digest. Retain the existing memory and topology receipt bindings, tick checks, state-graph validation, and error behavior.

**Step 3: Run focused production GREEN tests**

Run the smallest compiling app/GPU target that proves the ordinary/manual/sleep boundary distinction. Keep tests serialized and require the physical GPU when the selected target already does so.

---

### Task 4: Review checkpoint

**Step 1: Run bounded verification**

Run the focused promotion, backend receipt, and production boundary gates. Run `git diff --check`.

**Step 2: Inspect exact scope**

Report `git status --short`, `git diff --stat`, and the exact changed paths. Confirm no sleep cadence, asynchronous staging, world replacement, training, or unrelated formatting change entered the diff.

**Step 3: Stop unstaged for supervisor review**

Do not commit the implementation. Report RED/GREEN receipts and the exact diff boundary before beginning asynchronous checkpoint staging.
