# Three-command native grounding and timing comparison

This follow-up to [the action-gated payload study](retained-language-action-gate.md)
finds and fixes concrete teaching defects. **The final comparison still does not
demonstrate reliable noun/action comprehension or timely grounded narration.**
All subjects are disposable public founders; execution is native CPU. No PC,
live training, default brain topology or accepted private checkpoint is changed.

## Findings and targeted changes

The old `eat food` / `get toy` curriculum couples each verb to one noun. Add
`get food`: food ingestion and food acquisition separate the verb, while food
and toy acquisition separate the noun. Acquisition requires a real ownership
change under a selected native Grab; ingestion requires a real consumed outcome.
No transition or reward is manufactured.

The V8 training cue continued after a nonrequested activity until the requested
activity or cue timeout. Assessment stopped after its first activity. Actual
nonrequested narration labels arrived 8–15 ticks after the event, versus 1–4 for
requested activities. All factorial variants stop training and assessment cues
after any real completed activity and retain the original three eligible quiet
narration-row criterion. Their observed positive label lags are 1–4 ticks.

The first factorial run also exposed a teacher-affordance problem. At the visible
0.36-unit food placement, the shared vocabulary helper chooses Approach because
its visual-contact condition is stricter than the grasp executor's reach checks.
Grounded and shuffled therefore collected no GetFood activities in that run.
A closer 0.24-unit fixture did not yield a reception choice; its detailed
visibility/approach-enumeration cause was not isolated and it is not used.

The correction keeps the original visible 0.36/0.44 training distances. A small
read-only `HeadlessWorld::can_grab_target` query delegates to the **existing native
grasp preconditions**. The experimental teacher can label the available native
Grab, or Eat for its food task, before geometric visual contact. Availability is
queried before execution and remains only an offline teaching input. Sensing,
physical execution, sampling and evaluation action choice are unchanged. The
shared legacy vocabulary helper remains unchanged.

Native Grab also releases an already-held target. The intermediate legal-action
variant mistakenly taught that repeat action in three grounded and three
shuffled rows while request speech lingered. The final variant preserves Get's
completed-task semantics: a held requested target receives Idle, rather than a
second Grab. Its complete trace audit finds zero such erroneous release labels.
The physical unit control checks actual food acquisition without ingestion, a
full grip's refusal of another target, native same-target reversibility, and the
teacher's held-target Idle choice.

All variants use the native recurrent/action/payload objectives. Payload budgets
are balanced over **observed actual activity classes**, without inventing missing
examples. Quiet physical choices remain unconstrained apart from excluding
Vocalize. Targets never enter neural perception. Every real world outcome and
biological reward remains authoritative.

## Matched protocol and results

All arms and variants restore the exact same actual V8 step-256 actor/Adam
history, hash `0fab85cb1e324bf579571ceca6defa2afdcdb8548cf6f3159909fbf555ee6c13`.
They retain both moments, per-coordinate integer ages, mask and frozen state.
Each variant adds 32 native updates in two 16-update rounds, reaching step 288.
Each collection has two shared native world/private parents, each forked into
three commands. Assessment has contact, near and postsleep parents, each forked
into three player commands plus silence. All initial targets and a distractor
are visible. Positions change from training; teacher and semantic prior are off.
No evaluation action is forced.

Each variant scores 2,016 actual decisions: 864 training, 864 final assessment
and 288 common baseline, plus cold-repeat verification. Source/binary identities
and complete per-trial results are in the [source-safe comparison](retained-language-factorial-comparison.json).
Concurrent compilation/diagnostics occurred; durations are not benchmarks.

| Variant | Measured source | Seconds | First outcome correct G / S / N | Timely exact narration G / S / N |
| --- | --- | ---: | --- | --- |
| V9 timing + three commands | `d8ea6ba9` | 501.508 | 1/9 · 4/9 · 3/9 | 0 · 0 · 0 |
| V10 legal-action teaching | `3dcdcaca` | 489.707 | 3/9 · 2/9 · 3/9 | 6 · 6 · 0 |
| V11 completed-Get semantics | `b9cbd7f6` | 484.743 | 3/9 · 3/9 · 3/9 | 0 · 0 · 0 |

G is grounded request alignment; S shuffles request phrases through a three-cycle;
N receives no teacher request speech. **All three retain truthful offline activity
and payload labels**, plus their common prior grounded history. These controls
test incremental request alignment, not an untreated narration-label ablation.
Their activity distributions can differ; all actual counts are reported.

The common baseline has 3/9 correct first outcomes, 4/9 eventual requested
completions, 13 utterances and zero timely exact narration. The final result is:

| Final V11 outcome | Grounded | Shuffled | No request speech |
| --- | ---: | ---: | ---: |
| First outcome matches request | 3/9 | 3/9 | 3/9 |
| Requested activity eventually completes | 3/9 | 5/9 | 3/9 |
| Correct paired verb choices | 0/3 | 0/3 | 1/3 |
| Correct paired noun choices | 0/3 | 0/3 | 0/3 |
| Actual world utterances | 12 | 12 | 12 |
| Timely exact activity declarations | 0 | 0 | 0 |

Each paired contrast uses the same initial private/world state: EatFood versus
GetFood for verbs, GetFood versus GetToy for nouns. First-outcome discrimination
is separate from eventual completion because acquiring food can legitimately
precede eating it. Shared parents and a single founder seed are not independent
population replications.

The intermediate six timely matches are real native declarations following real
food acquisition, and survive exact cold/sleep checks. However, both actors say
`get food` in **all twelve trials**, so those matches do not establish noun
binding. In final V11, grounded emits six `get food` phrases and six single `get`
words; shuffled emits ten and two; no-request emits twelve `eat food` phrases.
Every noun actually emitted is food, regardless of the target activity. The
remaining stereotyped production and cue-independent first actions are unresolved.
Loss reduction, heard frames and optimizer updates are not competence results.

## Retention, compatibility and validation

All nine trained actors across the three variants pass exact fresh-process full
assessment-trace, private continuation and next-native-Adam checks. Sleep uses
native RecoveryProtocol consolidation and waking; each cue quartet shares one
sleep ancestor per assessment pass. This is induced, not spontaneous sleep.
Training uses fresh real cohorts with retained actor/Adam history and conditional
replay of captured fast/lifetime offsets and context, not a same-individual live
optimizer rebase.

The no-request actor is **bit-identical across V9/V10/V11**, and all three common
baselines are identical. Historical V8 and V10 complete cold/private/next-Adam
recoveries reproduce exactly under the final code. The new assessment selector
is default-off and omitted from legacy experimental recovery serialization.
No accepted game-save schema changes. All previous negative artifacts, source
actors, measured executables and detailed optimizer/private recoveries remain
intact locally.

Eleven normal pilot tests, strict Clippy including world/app tests, formatting,
headless compilation and Python syntax checks pass. The initial food-choice
fixture failures are preserved as diagnostic evidence, not counted as passing
checks. Reusable trace audits record narration lags and incorrect repeated-Get
labels; the final audit records zero such labels.

## Reproduction

```sh
target/debug/experiment_retained_language --disposable-public-founder \
  --experimental-word-bridge --factorial-grounding /absolute/measured-v8 \
  --output /absolute/new-v11 --seed 847501 --rounds 2 --epochs 16 --decisions 24
python scripts/audit_retained_language_teaching.py /absolute/new-v11/native-trace.jsonl.zst
python scripts/summarize_retained_language_speech.py /absolute/new-v11
```

V9 and V10 are separate preserved source commits and measured runs. The final
variant forks their same original V8 source; it does not discard or reset their
newly measured step-288 actors. Raw checkpoint assets and traces are not published.

## Luna diagnostic and remaining access blocker

One explicitly authorized changed CLI invocation used the prior public seed-847503
simulation observation and privately captured stdout/stderr, with a 30-second
timeout. No credential, inherited guard, transport or routing setting changed.
It exited 1 after 0.065 seconds, with zero stdout/answer bytes and 197 stderr bytes.
The safe whitelist discarded too much of the error; its exact message was not
retained before the temporary directory was removed. The specific cause of that
exec exit therefore remains unidentified.

Separate official redacted `codex doctor --json` diagnostics establish that
ChatGPT auth is configured and HTTP endpoints are reachable, while the
**ChatGPT-authenticated Responses WebSocket probe returns 401 Unauthorized**.
HOME/CODEX_HOME are read-only, and default log/SQLite locations are under that
read-only directory. Official local validation recognizes writable `log_dir` and
`sqlite_home` options; neither HOME nor CODEX_HOME was relocated. Those output
options were not applied to another inference call after the known unauthorized
probe. HTTP reachability does not establish model access, and the filesystem
findings are not asserted as the exact original exit cause.

No inference was attempted after that known unauthorized result. No credentials
were read manually or changed; no access restriction, proxy or security guard was
bypassed. The native provider-to-encoded-lane test still has zero delivered model
outputs. All measured training here is scripted, not genuine SLM-assisted learning.
[Sanitized diagnostic receipt](retained-language-luna-readiness-diagnostic.json).
