# Two-witness native tape and gradient diagnostic

This follows the frozen V24 assessment without teaching or changing a brain. It
recaptures only pre-sleep seeds 16309347 and 16508317, EatFood/GetFood, on the
unchanged Untreated and MixedGrounded4 sources. Untreated already has grounded
language history; it is untreated only with respect to the later care replay.
These are post-hoc failure witnesses, not new held-out evidence.

The successful native CPU run took 23.783 seconds (23.806 wall), including exact
fresh-process state loading. It executed exactly 16 scored decisions and 16
read-only gradient probes. It made zero Adam or shadow Adam updates. Eight
explicit native host preparation steps and ordinary world/development setup
preceded the episodes. There was no teacher, semantic prior, forced action, new
SLM request, accepted checkpoint change or promotion.

Measured source: `397413f725fd5bf6e064c33a61730ba86b92fb39`. Measured test executable
SHA256: `122ccbe88a3f1af92a5bba173d99affda9886b21a44a61d93d42f52e29e26e06`.
[Receipts](retained-language-bounded-tape-results.json) retain the relevant raw
outputs, action logits, native action/sampling receipts, gradients by head and
mask, preserved preflight failure, state hashes and provenance. Full tapes,
gradient vectors, all candidate/six-step outputs and exact private checkpoints
remain in the separate disposable run directory; private saves and executables
are not published.

## Exact replay and scope

All eight initial full-state hashes match the earlier V24 cold trace, using
V24's JSON-Value representation. All 16 full native row hashes, physical
outcomes, emissions and RNG sampling receipts reproduce V24 exactly. The
diagnostic saves the full later-row neural activation tapes in eight cases.
Eight two-decision prefix checkpoints reconstruct exactly in a fresh process;
that child takes zero decisions and zero probes. Actor steps stay 432/436, with
all weights, moments, integer ages, masks, optimizer configuration and foundation
bindings unchanged. Native private biology and learning remain active within the
disposable episodes, as in V24.

An initial preflight compared a typed checkpoint hash with V24's JSON-Value
hash and stopped before its first cue, scored decision or probe. It used two
preparation steps. The failed executable, source, output and failure receipt
remain intact. The corrected check uses the same serialization as V24 and
requires an exact match before proceeding. An earlier build exhausted disk;
only newly generated compiler cache files were removed. All project artifacts
and measured executables were preserved.

Across both attempts the total remains 16 scored decisions, 16 probes and zero
optimizer steps. All 95 protected prior source/run files, 26 earlier public
evidence files, 56 seed receipts, seven invalid-care-preflight files and 17 V24
run files verify byte-identical. The explicit diagnostic passed; strict CPU
library/test Clippy and formatting checks passed. No other native tests or
simulations were run for this follow-up.

## The held-food verb failure has a usable action gradient

At seed 16508317 both cues naturally Grab at step 0. At step 1 food is held,
Get has already succeeded, the differing Player verb remains heard, and the
natural prompted speech cooldown is ready. Both sources nevertheless select
Eat, actually consume the food and gain 0.12201631 energy. The recorded biological
credit is positive under both verbs. It does not judge compliance with Get.

Native Eat can also pick up reachable safe food directly. The step-0 teaching
choice can therefore already distinguish Eat from Grab; their shared Grab is
a permissible intermediate action for Eat. Step 1 is the first observed
violation of the Get retention criterion after that shared acquisition.

The held/Get teaching target is a legal Vocalize declaration of the real
preceding Get event. It is not an Eat label or premature report. The Eat target
is Eat. The table gives the native Eat-minus-Vocalize margin and its derivative
along the negative existing component gradient, restricted to the retained stage
mask. Negative values move the margin against Eat.

| Source | Heard cue | Eat minus Vocalize | Negative-gradient direction |
|---|---|---:|---:|
| Untreated | EatFood | 3.956103 | +22.446299 |
| Untreated | GetFood | 0.962848 | -410.545258 |
| MixedGrounded4 | EatFood | 6.301031 | +1.435890 |
| MixedGrounded4 | GetFood | 3.405747 | -611.555725 |

Changing the verb lowers this margin by roughly three logits, yet Eat remains
preferred under Get. The native action objective supplies a nonzero gradient
in the correct direction at the failing held/Get state. This rules out total
word disconnection, a missing action target and zero actionable gradient there.
It does not locate the failure in historical batch weighting, replay coverage,
shared parameter interference or private biological credit. The two sources
also have different evolved private states, so their margin difference alone
does not isolate a global-weight effect.

The probe reconstructs the native per-row imitation and factual-speech
components at unit weight. It does not reproduce historical batch
normalization, conditional choice/noun-margin terms or the later whole care
mixture. Its infinitesimal gradient direction is not an Adam step or evidence
that training these rows would retain other behavior.

## Premature speech exposes an absent supervision path and a mask distinction

At seed 16309347 both sources select Idle at step 0, then actually emit Declare
EatFood/GetFood at step 1. Neither object is held; no prior or current physical
event supports the declaration. The words are correctly delivered, decoded and
emitted. Prompted cooldown is ready. First-token raw presence is positive
(0.965–1.404), the first stop output is negative, and the second stop output is
positive. Thus normal payload and cadence gates permit a well-formed but
premature phrase; an inactive decoder or cooldown bypass does not explain it.

The existing action loss targets the physical action, discouraging Vocalize.
The existing missing speech target contributes exactly zero direct speech-head
gradient. Only these four witnessed premature Declare states receive a native
silence target in the diagnostic. Other missing labels, including the held/Eat
row after a real Get, remain unlabeled.

| Source / cue | Raw speech-head silence gradient L2 | L2 allowed by current mask |
|---|---:|---:|
| Untreated / EatFood | 27.518856 | 27.518856 |
| Untreated / GetFood | 31.921606 | 31.921606 |
| MixedGrounded4 / EatFood | 27.820453 | 0 |
| MixedGrounded4 / GetFood | 32.202747 | 0 |

Each silence probe has 64 nonzero direct speech-head coordinates, for presence
and stop. Its direct readout gradient lowers first presence and raises stop.
Untreated retains a mask that permits speech learning; MixedGrounded4's later
action-only mask excludes the entire silence gradient. Neither mask is changed.

This supports testing one narrow correction: explicit witnessed-absence
supervision in a separate gate-only experimental stage, mixed with real timely
positive reports, while freezing lexical/act readouts and retaining all optimizer
history. A gate-only mask would also avoid drifting unrelated payload weights
through retained Adam momentum. This diagnostic does not test the actual Adam
effect, feature separability, positive-report retention, improved physical
behavior or generalization. No correction has been trained or promoted.

## Reproduce only the bounded diagnostic

Use a new directory and the measured V24 disposable source. Do not restart after
a partial scored failure: inspect its durable decision/probe journals and saved
states first. A rerun must not silently duplicate a bounded comparison.

```sh
ALIFE_BOUNDED_TAPE_SOURCE=MEASURED_V24 \
ALIFE_BOUNDED_TAPE_OUT=NEW_DIRECTORY \
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
cargo test -p alife_game_app --features cpu-training \
  --config profile.test.package.alife_training.opt-level=2 --lib \
  foundation_training::retained_language::curriculum::factorial::balanced::bounded_tape_probe::diagnose_two_frozen_witnesses \
  -- --exact --ignored --nocapture
```
