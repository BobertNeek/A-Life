# Native cue, payload and Vocalize factor diagnostic

This is a read-only diagnostic of the preserved V17 actors from #257. It does not
train or promote a new creature. Two previously observed held-out worlds (cases
0 and 2, seeds 2404507 and 2608889) each fork the same native private/world parent
for EatFood, GetFood, GetToy and silence, across untouched, grounded, shuffled and
no-new-teacher actors. Eight unforced native steps per life give 256 decisions.
Every decision must match the existing V17 full-row digest, and native action
resampling must reproduce the collected choice exactly.

The probe separates the actual native Vocalize choice, selected vocal channel, authoritative cooldown,
conditional native payload, lexical bits, presence/stop/Declare gates, and true
prior physical events. Correct conditional payloads are diagnostics, not actual
spoken outcomes. Proposed channel tokens must match the native decoder; actual emissions must
match that payload after the authoritative cooldown (8 prompted, 32 spontaneous).
The last actual speech tick is checked against the exact world checkpoint.
Factual opportunities use V17's event-confirmed cue-window rule.

For each paired onset, a frozen-row replay swaps only the 160-neuron direct input
bank from the real paired cue, keeping all other recorded replay context fixed.
The bank includes native word and bodily contributions; this is a cue-associated
bank intervention, not a pure semantic intervention or fabricated world action.
Raw encoding, speech-source and speech word-logit deltas distinguish a missing
input route from a payload/gating problem in these actual contexts.

At the first real confirmed opportunity in each life, a disposable shadow actor
copies the full native weights, Adam moments, integer ages and original stage
mask. It takes one native update on that single actual speech example and is
then discarded. Its conditional in-sample loss and native gradient/displacement
alignment diagnose whether that objective can move locally. The gradient is not
the full curriculum batch; this does not predict actual batch training, establish
held-out learning or execute any world action. Frozen coordinates must remain
exact and every original actor/checkpoint remains unchanged.

## Reproduce

```sh
ALIFE_SPEECH_FACTOR_SOURCE=/absolute/path/to/measured-V17-run \
ALIFE_SPEECH_FACTOR_OUT=/absolute/path/to/NEW-result.json \
cargo --config profile.test.package.alife_training.opt-level=2 test -p alife_game_app --features cpu-training --lib \
  diagnose_native_cue_payload_and_voice_factors -- --ignored --nocapture
```

The source must be this pilot's disposable measured V17, and output creation
refuses to replace existing evidence. This diagnostic has two worlds, one trained
founder and no new sleep/retention or blind-validation claim. The original V17
cold/sleep evidence remains intact. No teacher, semantic prior, forced evaluation
action, learned SLM request, Luna retry, default-brain migration or live/PC work.

## Measured result

Primary measured diagnostic source: `99e97e142bc5928cf3dd33b26658f55509e51700`.
Native test executable SHA-256:
`e96aeea1efecb3477e0603a195b12ad8f4c84911f76e34350ae04a0aff7de1b5`.
The corrected probe takes 26.883 native CPU seconds. Every one of the 256 actual
rows matches the saved V17 native digest and exact native action resampling.
The initial receipt and the corrected probe reproduce the same original native
rows, outputs, cue-bank deltas and shadow losses. A diagnostic assertion initially
mistook a selected vocal channel for emitted speech; the corrected probe validates
both the proposed payload and authoritative cooldown before actual emission.
No V17 results or source actors change.

These counts are correlated opportunities within two prior worlds, not
independent trials or new language competence:

| Actor | Confirmed opportunities | Correct conditional payload | Correct and cooldown-eligible but no Vocalize choice | Correct but on cooldown | Actually spoken correct |
|---|---:|---:|---:|---:|---:|
| Untouched | 17 | 0 | 0 | 0 | 0 |
| Grounded | 14 | 8 | 5 | 2 | 1 |
| Shuffled | 9 | 1 | 1 | 0 | 0 |
| No new teacher | 17 | 0 | 0 | 0 | 0 |

The grounded direct input bank changes by L2 4.446–9.166 across the three cue
pairs in each world. Native speech source changes by 2.032–4.201; swapping only
the cue-associated bank changes the first two words' raw bit outputs by
0.796–1.602. Thus the direct cue route is active in these conditions. Increasing
raw cue strength is not the leading supported target here.

Grounded presence, stop and Declare gates are correct at all 14 factual
opportunities. Its eight correct conditional payloads comprise five EatFood and
three GetFood states. Only one is actually spoken. Three GetFood states have
Vocalize probability 0.037–0.067 and two far-world EatFood states 0.206–0.224;
these five are outside cooldown but the native sampler chooses other actions.
The other two correct unspoken EatFood states are within cooldown after the
real correct report; speaking would be illegal if selected. Those should not be counted as avoidable gate
failures or demanded as repeated speech during the same short window.

Every grounded GetToy state (six opportunities) instead decodes `[9,3]` rather
than `[9,2]`: one wrong noun bit. One of those wrong payloads is actually spoken,
matching the proposed native payload. No gate adjustment alone can correct it.
Untouched and no-teacher also have wrong word bits at all 17 opportunities and
fail at least one presence gate in every one; shuffled has correct form gates
but wrong bits at eight of nine opportunities.

All 18 one-example shadow updates lower their conditional native loss and have
negative gradient/displacement dot products, retaining source optimizer history
and frozen-coordinate identity. Grounded's median loss ratio is 0.870, but its
GetToy bit remains wrong after both shadow updates. This demonstrates a local
working gradient, not that the aggregate curriculum objective or training dose
is sufficient. No shadow is saved, assessed as a learned creature or promoted.

The world rejects one shuffled silent vocal proposal on its 32-tick spontaneous
cooldown. The source engine uses 8 ticks for player-prompted speech and 32
otherwise; the CPU host marks only heard Player speech as prompted. Teacher
speech does not set that flag. Three consecutive positive Vocalize labels can
therefore conflict with speech cadence after an actual utterance. This source
rule is unchanged. The diagnostic checks the exact world last-speech tick at
each trial boundary and predicts every actual utterance from payload/cooldown.

The supported next target is the Vocalize choice objective and its legality/
cooldown alignment, with the GetToy lexical bit treated separately. Weak or
missing cue input is not the primary measured bottleneck in these two fixtures.
This diagnostic implements no new corrective training and does not turn
conditional correct payloads into claims of real spoken behavior.

## Receipts

The full read-only data, measured test executable and provenance remain in
`/workspace/language-speech-factor-run`. The initial receipt remains at
`/workspace/language-speech-factor-diagnostic.json`. The source-safe
[summary and per-opportunity evidence](retained-language-speech-factor-diagnostic-results.json)
can be regenerated with:

```sh
python scripts/summarize_speech_factors.py /absolute/path/to/diagnostic-run
```

#257 exact head `a64620c859eed0babbb234e2909fa28ab326d007` has all three
hosted workflows successful. Jules' posted review says IN_PROGRESS and did not
complete; workflow success is not review approval. No hosted failure requiring
a source fix was found. Prior negatives, source checkpoints and default brains
remain intact; no Luna retry or candidate promotion.

Validation: the final-source fresh-process repeat reproduces every original row,
cue-bank measurement, shadow loss and gate factor exactly. The explicit diagnostic
passes; 18 standard native language tests pass with six opt-in diagnostics ignored.
Strict Clippy including tests, fmt and Python compilation pass. No production
runtime or checkpoint format changes are included.
