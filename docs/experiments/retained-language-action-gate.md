# Native utterance timing and activity payload experiment

This continuation follows the preserved negative [V2 embodied-input experiment](retained-language-embodied-input.md).
It establishes native speech production under a new experimental objective, but
**does not pass the predefined grounded-language criterion or establish CFF
ChiChi competence**. No default brain, live training or accepted private life is
changed. All execution is native CPU, with scripted teacher speech.

## Causal gate diagnosis

Positive narration targets supervise raw presence output 16 at +0.8 and stop
output 17 at −0.8 for the first word, +0.8 for the final word. Quiet payload targets
supervise presence −0.8 and stop +0.8. The native runtime samples an action, calls
the speech decoder only for a selected Vocalize motor, and requires positive
presence plus nonzero token bits to emit a word. Presence is a hard threshold,
not a sampled category. Training differentiates the raw speech objective, not
that inference threshold.

The actual public founders have no enabled conditional vocal sampling mask.
Vocalize wins through representative action selection; other representatives
can abstain. Native gate weights are trainable, both Adam moments and integer
ages are retained, positive speech gradients reach decoder and recurrent weights,
and action loss has zero direct gradient on speech-decoder coordinates. Initial
full/one-step replay matches the saved actual positive presence values exactly.
The trained V2 actor is negative even on the original replayed positives, so the
failure is not solely a cold-evaluation context shift.

The prior 0.9/0.1 positive/quiet budget balances shared-output mass, but its
actual first-presence weight gradients conflict:

| Saved arm | Positive/quiet gate-gradient cosine | Positive gradient dot previous mixed gate gradient |
| --- | ---: | ---: |
| Grounded | −0.994582 | −0.016199 |
| Shuffled | −0.992861 | −0.016875 |

Negative dot means descent along the old combined direction locally increases
positive presence loss on those coordinates. A central finite difference under
real positive supervision matches the native gradient: 0.0594644 versus 0.0594650 in
grounded, 0.0610903 versus 0.0610895 in shuffled. Direct normalized 0.01-L2
perturbations of only the 32 presence weights confirm the conflict. Grounded
positive loss rises 0.106837→0.109062 along the old mixed direction and falls to
0.104661 along the positive direction; shuffled rises 0.118955→0.121097 versus
falling to 0.116850. These use disposable diagnostic copies without an optimizer
commit. They are local objective evidence, not behavior or learning scores.

[Native diagnostic evidence](retained-language-presence-gradient-diagnostic.json)
contains the exact source identity, native gradients and interventions.

## Experimental objective and controls

`--action-gated-payload SOURCE` restricts this experiment to its measured public
V2 source and representative-only vocal arbitration. It keeps native recurrent,
action and speech math, runtime thresholds and sampling intact. Payload content
is supervised only after real completed activity. On other quiet rows, an action
target permits every available physical representative except Vocalize; it does
not select a physical action. Audible requests retain their grounded native
representative/motor labels. Positive narration retains native Vocalize and
exact two-word payload targets. Both speech and physical action remain learned
choices in evaluation.

All three arms restore the **same actual step-224 grounded actor and Adam
history**, hash `5b2849c1713a6fa815492f33a335f7a9746ab86a9e90baf6bfbe557f4802b93c`.
The common untreated V2 baseline is measured first. Grounded, shuffled and
no-speech arms then collect two rounds of eight paired 24-decision real lives,
with 16 native updates per round, reaching step 256. Their positive narration-row
counts are 42, 45 and 24, from 14, 15 and eight actual completed activities.
No-speech has zero teacher-heard frames but retains truthful activity/payload
labels and the common source's prior grounded history. It is an incremental
request-speech control, not a language-naive or unsupervised brain.

The raw collector trace preserves its original labels for provenance. A separate
`effective-native-training-targets.jsonl.zst`, bound to each real experience-patch
hash, records the actual offline labels used. No target or completion label is
inserted into perception. Biology, outcomes, ownership changes and rewards are
sealed by the real world. Training uses fresh cohorts and retained native actor/
Adam state, not a same-individual optimizer rebase. Conditional recurrent replay
retains captured native fast/lifetime offsets and context.

## Completed physical and utterance results

Measured implementation: `9b0be1169629031e12fa0364d8b5bca4beb7fe41`.
Executable SHA-256:
`8bd6e78dae72b87b4c2ca5ac5b23a0f31277f559a52e0cbe745ab2e7f1a2ef5d`.
The run completed in 569.073 seconds externally, with concurrent diagnostics and
compilation. This is not a performance benchmark. It scores 2,016 decisions:
1,152 training, 648 final assessment and 216 common baseline, plus cold repeats.

| Outcome | Common baseline | Grounded | Shuffled | No request speech |
| --- | ---: | ---: | ---: | ---: |
| First physical completion matches actual request | 3/6 | 2/6 | 3/6 | 3/6 |
| Postsleep request matches | 1/2 | 1/2 | 1/2 | 1/2 |
| Nourishing meals | 8 | 0 | 6 | 5 |
| Genuine toy acquisitions | 5 | 6 | 6 | 5 |
| Eligible narration rows | 33 | 18 | 33 | 24 |
| Actual emitted world utterances | 0 | 9 | 7 | 4 |
| Exact expected narration within eligible rows | 0 | 0 | 0 | 0 |
| Utterances outside primary window | 0 | 9 | 4 | 4 |
| Exploratory exact description of latest preceding activity | 0 | 6 | 0 | 0 |

Utterances are genuine decoded native Vocal payloads, verified against actual
world speech receipts. All measured emitted speech acts are Declare. Grounded
emits eight `get toy` sequences and one `eat toy`; shuffled emits five single
`get` words, one `get food`, and one single `eat`; no-speech emits four `eat food`
sequences. Grounded's six late `get toy` descriptions occur six or nine ticks
after genuine acquisitions, including postsleep cases. Three other grounded
utterances have no preceding completion. Grounded's constant toy noun and
word-independent toy choices prevent interpreting these late matches as a
learned object/action distinction. The late count is exploratory; it does not
replace the original three eligible narration-row criterion. The field named
`unsolicited_utterances` counts all speech outside that window, including late
activity-consistent descriptions.

All requested and silent evaluation forks have teacher absent, semantic prior
off, relocated contact/near positions and visible food, toy and distractor.
No evaluation action is forced. All arms pass exact fresh-process full native
trace/private continuation/next-Adam checks and native RecoveryProtocol sleep
assessment. Each postsleep cue trio shares one consolidation ancestor per pass;
this is induced native sleep, not spontaneous sleep. The historical V2 complete
assessment/private/next-Adam recovery also reproduces exactly under this code.

The objective change overcomes silence, while the predefined language and request
outcomes remain negative. No equal-dose legacy-objective arm was run; objective,
quiet action labels and recollection changes cannot each be assigned all the
production gain. This is one seed with shared prior history and unequal real
activity distributions, not independent population replication. Native language
comprehension, far navigation and a living LLM harness remain unestablished.

[Full source-safe results](retained-language-action-gate-results.json) include
exact actor/continuation hashes, training receipts, final outcomes and the
exploratory timing analysis. Raw traces, measured executable and exact recoveries
remain local. Ten normal pilot tests, the explicitly invoked real-experience
native gate diagnostic, strict Clippy including tests, formatting, headless
compilation and the final build pass.

## Reproduction and provider limit

```sh
target/debug/experiment_retained_language --disposable-public-founder \
  --experimental-word-bridge --action-gated-payload /absolute/measured-v7 \
  --output /absolute/new-v8 --seed 847501 --rounds 2 --epochs 16 --decisions 24
python scripts/summarize_retained_language_speech.py /absolute/new-v8
```

The ignored `diagnose_actual_v2_gate_targets_and_native_gradients` test uses an
explicit fresh `ALIFE_GATE_DIAGNOSTIC_DIR`, saved V7 source and its V6 predecessor;
it makes no model request. Its default source locations match the disposable
workspace, and `ALIFE_GATE_SOURCE` can select the measured V7 directory.

The previous Luna request spawned the official CLI and exited 1 after about
0.125 seconds: zero validated replies, deliveries or encoded model outputs. It
did not time out. The adapter discards stdout/stderr and deletes its temporary
run directory, so permitted saved evidence cannot distinguish startup/config,
authentication, model-access, usage or network causes. No further inference,
credential change, route change or guard bypass was attempted. A working
authorized bounded Luna execution, or an authorized compatible local model/server,
is still required for genuine SLM delivery. All measured teaching here is scripted.

The [three-command timing follow-up](retained-language-factorial-grounding.md)
separates noun/action contrasts, fixes native-action teaching and preserves all
three measured variants, including their remaining negative language results.
