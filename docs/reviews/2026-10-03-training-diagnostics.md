# Measurement-only training diagnostics and dose comparison

This candidate adds offline measurements to the existing N2048 warm-up and pilot
receipts. It does not change the objective, episode weights, batch divisor,
learning rate, masks, optimizer, policy graph, prior defaults or trained assets.
The legacy imitation entry point performs no additional GPU readback. Warm-up
uses the diagnostics entry point to read speech outputs from its existing forward
pass; the cost of that readback on the target adapter is **Unknown**.

## Receipt meanings

- `warmup.losses` retains its action imitation meaning. The aligned
  `speech_payload_losses` contains separate positive and silence contributions
  to the existing class-normalized payload MSE. Both use the unchanged episode
  weights and actual-weight batch mean; neither is added to the action loss.
  Label-row and token-step counts in these entries are counts in that update's
  windows, excluding burn-in. They are repeated across epochs.
- `corpus_lesson_budgets` counts each demonstration once, with its unique loss
  rows, positive/silence/unlabeled payload rows, positive token steps, and captured
  teacher Vocalize selections. It also reports window count, burn-in replay work
  and the sum of existing window weights. Record share and effective episode
  loss-budget share are different quantities. Burn-in does not receive labels.
- `pilot.speech_behavior` counts Vocalize selections once per decision row, even
  if both representative and speech-motor selections name it. Legal support is
  the captured representative/speech-motor support. A `vocalize_opportunity`
  requires a positive payload label and a legally available Vocalize action.
  Cooldown label rows remain in the legacy `speech_opportunities` field but do
  not enlarge this new denominator. Rates are absent when it is zero.
- Valid utterances must be validated creature output from the learner at that
  decision tick. Correctness means exact expected token sequence, as in the
  existing counter; it does not establish comprehension or correct speech act.
  New valid/correct rates use legal positive opportunities. Teacher-mode receipt
  counters are demonstration evidence; prior-off evaluation measures the actor.

New fields default to absent when reading old receipts. Old evidence must not be
retrospectively described as containing these diagnostics.

## Later 2-versus-8 epoch experiment

Use [training_dose_diagnostic.py](../../scripts/training_dose_diagnostic.py) on the
authorized GPU development machine after a reviewed `foundation-training`
binary is built. Python 3.8 or newer is required. Supply a build receipt recording
the exact reviewed source SHA/tree, clean/dirty status, build command/profile,
binary SHA256, adapter/driver and relevant prior configuration. The runner hashes
this receipt but cannot independently prove that a binary came from that source.

First create a plan, with the actual frozen manifest and its founder seed, an
explicit evaluation world seed/tick budget/request token and a new output path:

```powershell
python scripts/training_dose_diagnostic.py --binary $TrainerBinary --source-receipt $BuildReceipt --manifest $FrozenManifest --output $NewPlanDirectory --world-seed $EvaluationSeed --founder-seed $CorpusFounderSeed --eval-ticks $EvaluationTicks --request-token $RequestToken
```

Review `dose-plan.json`. Execute later with the same arguments, a different new
output directory and `--run`. The script does not build, start a model server,
resume the protected checkpoint, or promote a candidate. Preserve the frozen
warm-up prior configuration for both arms. Evaluation overrides `ALIFE_SLM_PRIOR`
to `off` in each child environment without changing the calling shell.

Both arms start fresh from the same manifest source asset/founder and use the same
binary, corpus, compiled default rate/masks/objective and ordering. Only epoch
count differs. All eight lessons use matching world/founder seeds and tick budgets
(1–2048). Only vocabulary reception uses the controlled request token, which must
be 1, 2, 9, 13, 15 or 16 (food/toy, get/play, root/fruit pairs). Vocabulary
production uses its ordinary scenario utterance; its evaluator does not accept
`--request-token`. Inputs (binary, manifest, build receipt, source asset
and every pilot file) are SHA256 checked before and after each command. A shared
40-minute monotonic cap includes input verification, both warm-ups and all
evaluations; timeout kills and waits for the runner's own child. Partial outputs
remain available with `comparison_complete: false` and must not be called a
finished dose comparison. There is no automatic retry or checkpoint selection.

Compare action and payload loss separately, completed optimizer updates, lesson
budgets, legal opportunity counts, selections and valid/correct utterance rates,
alongside existing grounded physical outcomes. Report raw numerators and
denominators. A single matched seed is a bounded diagnostic, not population or
autonomy evidence. GPU execution and behavioral improvement are **Unrun** in the
CPU cloud validation.
