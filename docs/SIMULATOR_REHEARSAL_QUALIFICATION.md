# Short simulator rehearsal qualification handoff

Owner: the parent coordinates the existing PC owner after the integrated source
pin is published. The 491defda candidate failed initial runtime construction in
the continuity fixture before any neural decision. Its build/CI passes do not
qualify a repaired pin; use the replacement head recorded in PR109. Do not retry
the unchanged failed fixture or reuse its source-bound evidence for the repair.
The subsequent 910078f8 fixture completed its preceding continuity assertions but
failed Windows output cleanup with live archive owners. Its overall result was
still zero passed and one failed. The fixture now drains persistence and releases
both runtimes before strict deletion. Exact e21749e8 continuity subsequently
passed one test in 28.653 seconds including cleanup. Accepted-source A on that
pin failed collection admission after 28.706 seconds with no complete attempt,
no update and no continuation. B never ran. Preserve both failed-run evidence
sets and the consumed A seed; do not retry that source pin unchanged.
This handoff does not start a workload. The parent coordinates the same PC owner
on a reviewed replacement pin, with fresh resource checks and new diagnostic
seeds. Do not run default mode, comparison, extra organisms or a wider campaign.

The private policy7400 Library identity exists; the PC verified its upload and
manifest. CPU/group cloud consumers could not read bytes because proxy CONNECT
returned 403. Consumer byte validation is blocked. Do not retry that transfer for
this qualification, and do not put the private identity or signed URL in reports.
Use the PC's already validated local source package.

## Bounds

`--qualify` is mutually exclusive with comparison/control modes. Each invocation
admits at most four attempts of eight decisions and two holdouts of eight
decisions: at most 48 neural decisions, 24 actor/critic loss rows (20 for four
ordinary truncated attempts), one actor update and one residual-critic update. Burn-in and real bootstrap rows remain masked.
Collection admission stops at **240 seconds from runner entry**, including source
inspection, cold device setup, every initial durable snapshot, pre-decision
exact capture, real decisions and replay/value preparation. The qualification
profile has a separate **360-second whole-run allowance** including actor/critic
updates, export/rebind, both holdouts, persistence drain, GPU/archive owner release
and completion publication/verification. Ordinary modes retain their existing
600-second collection profile; qualification cannot invoke comparison/control.
The decision/window masks, exact captures, one-step updates and full acquired
source/Adam identity remain unchanged.

These replace agent-selected 15/60-second smoke bounds, not a user cutoff. The
failed A's capture ledger measured 9.6569172 seconds for one mandatory exact
capture. Filesystem timestamps suggest ~8.7 seconds before output and ~18.2
seconds for the first fresh runtime including that initial capture. These offsets
are **not instrumented phase durations** and prove no neural decision. Using
~18.2 seconds of fresh setup plus ~9.7 seconds of pre-decision capture per attempt
and ~8.7 seconds of cold setup gives a planning estimate of roughly 120 seconds for
four attempts before decisions/replay preparation. Collection's 240 seconds
allows roughly twice that estimate; the remaining 120 seconds reserves update,
holdout and publication work. Decision/PPO/holdout completion times remain
unmeasured. Neither allowance guarantees completion or establishes benefit.

`stage-timing.json` is mandatory after this invocation creates its new output
folder. It retains entry-clock start/end times and completed/failed/running stages
for source, cold device, scene/resident/archive setup, initial durable captures,
pre-decision captures, each real decision, replay/value preparation, actor and
critic updates, export/rebind, holdouts, persistence/resource release and final
publication. Actual training/holdout decision counters are persisted even if an
atomic GPU call crosses the allowance. A running stage has a null duration;
missing stages were not observed. Nested durations are inclusive, not additive.
Each timing update first appends and syncs a JSON record to
`stage-timing.jsonl`, then writes the convenient latest JSON summary. If an
operator stop interrupts a write, recover the last complete journal line; counts
then describe only that last observed point, not work that may have followed.
Journal recovery is diagnostic; do not reconstruct completion to admit a stopped
run.
Journal sync and logging overhead remain charged to the same clock. Zero-row attempt receipts retain null reward, including on budget failure. The
successful behavioral receipt's elapsed time ends after resource release; the
timing file also measures finalization. Failed runs retain partial artifacts and
stage errors and never retain an admissible continuation marker. Package/Adam
validation precedes marker publication. If an abrupt stop leaves marker bytes
before finalization, subsequent qualification-source admission fails closed unless
the timing summary records completed, in-budget finalization with matching actual
decision counts. A marker file alone does not establish completion.

Whole-run checks admit work before/after each stage and bound polling deadlines.
Atomic GPU calls, filesystem work and driver teardown cannot be preempted by an
in-process clock. Use an operator-enforced **360-second safety stop** per update
invocation, or a smaller actual resource-window limit coordinated by the owner;
record the external measured process time and any stop. The independent five-
decision continuity fixture retains its existing 60-second bound. A timeout,
zero trainable rows, zero selected continuity tests or partial output is **not a
pass**. Stop and return the measured failure; never retry automatically or reuse
a consumed seed. Compile separately, with no concurrent training job or paid
provisioning. No checkpoint is promoted by qualification.

## Execution order

1. Pin the PR's exact qualification head and record `git rev-parse HEAD`. Build
   `rehearse_n2048_care` and `inspect_archived_success` with `foundation-training`.
   Precompile the single continuity test with `foundation-training,gpu-tests` and
   `--no-run`. Obtain its exact test-executable path from Cargo's compiler-artifact
   JSON; do not guess a binary from a wildcard. Set `RUST_MIN_STACK=33554432`.
   The explicit prebuild commands are:

   ```text
   cargo build --locked -p alife_game_app --features foundation-training --bin rehearse_n2048_care --bin inspect_archived_success
   cargo test --locked -p alife_game_app --features foundation-training,gpu-tests --lib simulator_rehearsal::gpu_tests::exact_restore_preserves_private_context_and_next_real_consequence --no-run --message-format=json
   ```

   Retain the compiler-artifact JSON from the second command and select the
   executable whose target is `alife_game_app` and whose profile has `test: true`.
   Record the source tree (`git rev-parse HEAD^{tree}`) as well as the commit.
2. Read-only source admission, with no GPU session:

   ```text
   inspect_archived_success --validate-checkpoint SOURCE --expected-policy 7400
   ```

   Record the native file hashes privately. Require the expected source identity,
   objective2 and actor/value ages 16684/19800. A mismatch stops qualification;
   never repair it by resetting a head or editing the source.
3. In the first approved execution window, run only this already compiled test:

   ```text
   TEST_EXECUTABLE simulator_rehearsal::gpu_tests::exact_restore_preserves_private_context_and_next_real_consequence --ignored --exact --test-threads=1 --nocapture
   ```

   Require exactly one test passed, not zero selected. This executes five neural
   decisions using a seeded candidate and compares full snapshot restoration,
   next sensory frame/logits/probability, canonical physiology and replay context
   at the same sampling counter. It is a mechanical continuity check, not an
   accepted-policy7400 learning or promotion result.
   The seed asset must come from the shared canonical expressed-founder factory;
   raw scaffold metadata cannot substitute for its strict decoder contract.
4. Only after continuity passes and the PC owner confirms the next window, use a
   new output directory and an explicitly recorded diagnostic seed:

   ```text
   rehearse_n2048_care SOURCE QUALIFY_A 7400 SEED_A --qualify
   ```

   Require successful schema 2 own-package continuation publication, the actual nonzero trainable
   row count, all-trainable-attempt coverage, current collection policy7401, and
   both optimizer ages advancing once without reset. Expected final ages are
   16685/19801. Report actual attempt/holdout counts, waits, budget exits and times.
   The complete marker must name `rehearsal.json`, bind its hash and all four
   native training/evidence files, retain the inspected source hashes and state
   `execution_domain: gpu-wgsl`, `historical_freshness: unknown`, and no reset.
5. Only after that passes and the next short window is confirmed:

   ```text
   rehearse_n2048_care QUALIFY_A QUALIFY_B 7401 SEED_B --qualify
   ```

   This must read A's exact manifest, admit policy7401 and preserve its file hashes
   and optimizer state. Require collection policy7402 and final ages 16686/19802.
   Verify SOURCE and A still match their pre-run hashes. B starts new pools;
   archived states from A cannot silently become current-policy branches.

The two update/continuation executions together admit at most 96 decisions; the
separate continuity check adds five. Every execution needs its own confirmed
resource window. Diagnostic seeds have historical freshness **unknown**. A small
qualification can discover no useful successes and exercise no success retries;
record that absence rather than manufacturing a success or claiming benefit.

## Return to the parent

Return the exact source/code pins, native admission result, one-test continuity
result, source/A/B hashes, policies, optimizer ages before/after, trainable row
counts, source mix, successful discoveries/retries, all stage times and any hard
stop. Keep raw snapshots and private source details on the PC. Identify a timeout
or failing assertion by exact phase and outcome; do not mark it complete.

This qualifies mechanical continuity and state preservation only. Discovery,
retention benefit, throughput benefit and historical unseen-scene generalization
remain unmeasured until a separately resource-authorized comparison is completed.
That comparison gates benefit claims, rather than safe delivery of the opt-in
standalone experiment. Ordinary care/CPU/group adapters are separate work.
