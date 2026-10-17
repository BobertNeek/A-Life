# Continued critic residual objective

The critic-only correction used a full sampled one-step Bellman residual. The
continued training cycle subsequently restored GAE-return regression. On saved
production GPU values, the two objectives had opposing derivatives along a
head-scaling direction in361of378cohorts. Final head residual MSE rose to71.49/71.62
on identical saved128-transition feature contexts; initial16MSE reached566/570.
These fixed-feature measurements isolate head drift. Actor weights also changed,
but their independent representation effect was not measured.

## Implementation

Continued foundation cycles retain their existing GAE actor credit. The recurrent
PPO pass runs with value coefficient zero; it cannot reinstate the old critic
return-regression loss. The critic then takes the corresponding number of actual
AdamW steps on the captured production GPU current/actual-next activations,
frozen before the actor update. These are the same feature authority used by the
repair. Host staging copies GPU values; it does not evaluate a CPU neural model.

The critic loss is `c/2 * mean((V(s)-r-gamma(dt)*V(actual_next))^2)`. Both predictions
participate in its gradient, including the bias factor`1-gamma`. The existing
coefficient0.5, AdamW configuration, weights, and both moments continue. Terminal
discount is zero only for a real terminal. Truncation retains its real next state;
sleep gaps use the captured next decision and actual elapsed-time discount.
The bootstrap state remains outside actor and critic loss rows.

Critic minibatches contain at most2048loss rows. Their schedule covers the fixed
cohort in order, repeating for the number of accepted actor updates. This is an
explicit frozen-feature critic phase, not differentiation through a changing
actor representation during a PPO epoch. Qualification must therefore include
coupled actor/critic drift and fresh current-actor trajectories.

Every residual update validates finite predictions, gradients, gradient norm and
all prospective weights/moments before changing any coordinate. A rejected update
preserves weights, moments and optimizer age. The native analytic regression
checks elapsed-time/terminal gradients, bias, coefficient before clipping,
AdamW weights/m/v, and early and late nonfinite rejection.

`cycle.json` records critic objective version1and binds
`critic-residual-update.json` by SHA256. Historical receipts deserialize as
version0. The new sidecar records actual ages, real feature hash, loss rows,
optimizer configuration, and initial16/whole/last16residual slices per update.
It does not add offline repair steps to policy or episode counters.

## Qualification and current limits

Run `qualify_critic_residual --gradient-regression` on an available native training
GPU. `scripts/qualify_continued_critic.py` then copies a preserved source into a new
disposable output, pins binaries and protected checkpoint hashes, checks resource
reserves, and caps every native call before an explicit UTC deadline. It never
starts or adopts a production owner.

The bounded experiment checkpoints1/4/16/64/128cycles, with matched Eating/Grab,
full Adam and row accounting, fixed-feature validation contexts and untouched
acceptance seeds. Each initial16, whole128and last16MSE must separately remain
at or below`max(4*baseline,1.0)`, with finite in-support predictions. A whole-window
average cannot hide an initial transient. An optional old-binary control verifies
that one matched cycle produces the same actor checkpoint and full Adam state.
Final acceptance also captures fresh current-actor trajectories; those are
generalization evidence, not a causal partition of representation changes.

Native qualification is **pending** in this source revision. A short run is
explicitly incomplete extended qualification; no checkpoint is automatically
promoted. Residual minimization does not establish absolute continuing-return
calibration or guarantee convergence in stochastic worlds. There is no CPU neural
fallback, fabricated terminal, brain reset, reward change, or runtime hidden
trainer. The sealed policy5502checkpoint and old rollback copies remain separate.

## Architecture scope

This training-only change preserves ordinary causal organism interfaces and
teacher separation (AOA-INV-009, AOA-FOUND-002, AOA-FOUND-006). Whole-seed validation
and fresh acceptance address AOA-FOUND-004. No founder capability or promotion is
claimed; trainer-removed competence evidence remains required by AOA-FOUND-007.
