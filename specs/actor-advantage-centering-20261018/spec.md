# Opt-in actor advantage centering

Status: implementation complete; CPU and behavioral qualification pending
Mode: 2, bounded experimental training feature
Review target: R2 independent read-only review

## Contract

Add an explicit `--center-actor-advantages` option for sealed GrabFood
continuations. Omission preserves the existing production training behavior.
Subtract the complete captured batch's raw GAE mean only from the PPO actor
credit uploaded to WGSL. Preserve raw GAE, rewards, critic targets and residual
training, old collection probabilities/temperature, command identities,
ordinary Eat semantics, priors, lifetime learning, masks, weights, and full
actor/value Adam continuation. Replay windows retain the full-batch mean.
Reject incompatible overrides before creating GPU work. Record the actual
centering mean in the cycle receipt. Qualification runs must enable
`ALIFE_PPO_DIAGNOSTICS=1` to record actor credits separately from raw GAE.
Existing diagnostic summary advantage totals still describe raw GAE.

This is an experiment, not a claim that centering repairs learning. In the
latest 64 production cycles, centering would make seven of sixteen genuine
acquisition rows negative. Algebra and CPU tests cannot qualify adoption.

## Verification and execution plan

1. Test default bit preservation, centered actor values, unchanged raw targets,
   window slicing, constant inputs, and finite-range rejection.
2. Test CLI admission and diagnostic/receipt provenance. Run the relevant
   repository quick checks and focused CPU suites.
3. Preserve policy7400 and its complete Adam state as immutable source. Reconcile
   all previous seed reservations before a fresh non-overlapping experimental
   allocation. The663 unused seeds remain reserved; eight older uncredited seeds
   remain consumed. Disposable runs receive zero production credit.
4. Compare raw and centered branches from the same source with equal requested
   native GPU cycle/epoch/row dose and training seeds. Keep collection temperature32
   and historical probabilities intact. Verify matched first-cycle captures,
   rewards, raw GAE, critic targets, and unchanged source bytes. KL stopping can
   change accepted actor steps and the coupled residual-critic step count;
   verify actual accepted dose and report/reject unequal-dose comparisons.
5. Gate any proposed adoption on genuine possession-verified Grab outcomes in
   four physically distinct held-out T1 poses, complete held-food Eat retention,
   and selective probability/margin improvement against control. No adoption
   from fitting loss or CPU tests. Record later trajectory divergence honestly.
6. Run heavy work outside the automatic01:00–09:00 America/Edmonton window only
   while actual input idle is at least300seconds and RAM/disk/VRAM/temperature
   reserves pass. Serialize native GPU work with the existing lane owner lock;
   stop only owned jobs on loss of admission. The completed overnight job stays
   stopped. Bound the disposable comparison to15minutes after launch.
7. Publish the disabled-by-default experiment with exact test/behavior evidence,
   resolve actionable reviews, and merge only when eligible. Keep production
   checkpoint selection unchanged unless measured adoption gates pass.

## Architecture and recovery

AOA-LEARN-002/003/004 preserve causal identity and joint command evidence;
AOA-SLEEP-007 preserves replay/epoch/state identity; AOA-FOUND-003/004 require
generic competence and varied evaluation. No runtime policy or CPU neural
fallback is introduced. Roll back by omitting the opt-in flag. Preserve any
failed disposable output as uncredited evidence and repair ordinary tooling
failures within scope.

## Review and verification receipt

Mode2 uses an independent R2 read-only source review. All three findings were
resolved: reject mixed adaptation/warmup packages, require opt-in diagnostic
provenance during qualification, and compare accepted rather than requested
optimizer dose. The reviewer found no remaining actionable source defects.
Repository quick checks passed, including core boundaries and77documentation
assertions. Hosted Linux and Windows builds and focused tests are pending.
The local heavy-work admission at15:12UTC rejected launch because input idle
was0seconds outside the automatic window. No GPU comparison has run and no
learning benefit or production adoption is claimed.
