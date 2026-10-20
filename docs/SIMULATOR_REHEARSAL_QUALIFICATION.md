# Short simulator rehearsal qualification handoff

Owner: the parent coordinates the existing PC owner after group50's bounded test.
This handoff does not start a workload. Use only the resource window confirmed by
that owner. Do not run the default 600-second mode or `--compare` while Minecraft
is active. If the agreed short window is unavailable, leave qualification pending.

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
Collection admission stops at 15 seconds, including source/device setup. Existing
atomic GPU work, snapshots, optimizer work and holdouts are outside that soft
collection deadline, so it is not a whole-process wall limit.

Run processes serially with an operator-enforced hard stop of at most 60 seconds
per execution, or the PC owner's smaller agreed allowance. A slow cold start,
timeout, zero trainable rows or partial output is **not a pass**. Stop and return
the exact failure; do not increase the budget or retry automatically. Compilation
must be completed separately when CPU resources permit; do not hide a long build
inside the execution allowance. No paid provisioning or extra concurrent job is
part of this handoff.

## Execution order

1. Pin the PR's exact qualification head and record `git rev-parse HEAD`. Build
   `rehearse_n2048_care` and `inspect_archived_success` with `foundation-training`.
   Precompile the single continuity test with `foundation-training,gpu-tests` and
   `--no-run`. Obtain its exact test-executable path from Cargo's compiler-artifact
   JSON; do not guess a binary from a wildcard. Set `RUST_MIN_STACK=33554432`.
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
4. Only after continuity passes and the PC owner confirms the next window, use a
   new output directory and an explicitly recorded diagnostic seed:

   ```text
   rehearse_n2048_care SOURCE QUALIFY_A 7400 SEED_A --qualify
   ```

   Require successful native continuation publication, the actual nonzero trainable
   row count, all-trainable-attempt coverage, current collection policy7401, and
   both optimizer ages advancing once without reset. Expected final ages are
   16685/19801. Report actual attempt/holdout counts, waits, budget exits and times.
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
