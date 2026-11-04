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
cargo test -p alife_game_app --features cpu-training --lib \
  diagnose_native_cue_payload_and_voice_factors -- --ignored --nocapture
```

The source must be this pilot's disposable measured V17, and output creation
refuses to replace existing evidence. This diagnostic has two worlds, one trained
founder and no new sleep/retention or blind-validation claim. The original V17
cold/sleep evidence remains intact. No teacher, semantic prior, forced evaluation
action, learned SLM request, Luna retry, default-brain migration or live/PC work.
