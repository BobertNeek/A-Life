# Offline Manipulation probability audit

Status: offline diagnostic, 2026-10-17. Native replay adaptation and actual
Grab/Eat replay results remain Blocked. No GPU validation or training change
is included.

## Purpose and source boundary

`scripts/audit_manipulation_probabilities.py` computes the policy probability of
selecting caller-identified Grab/Eat candidates in motor slot 2. It uses every
legal candidate logit, the supplied legal masks, and representative forcing.
It does not estimate a probability from a top-two margin, change temperature
in a collected rollout, evaluate a model, or predict physical action success.
Python standard library only; Python 3.10 or newer is sufficient.

The contract is pinned to A-Life main commit
`1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a`:

- [`training_rollout.rs`](https://github.com/BobertNeek/A-Life/blob/1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a/crates/alife_gpu_backend/src/training_rollout.rs#L175-L322):
  kind mapping, receipt masks, f64 log-softmax from f32 inputs, and forced/empty
  factors. Blob `cbeb25761c092b22767f31ef4df0c381a6919535`.
- [`training_rollout.wgsl`](https://github.com/BobertNeek/A-Life/blob/1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a/crates/alife_gpu_backend/shaders/training_rollout.wgsl#L11-L77):
  sampling eligibility, fixed caller temperature, and demonstration validation.
  Blob `15962a5455abcceabbc638152082b590570f6094`.
- [`closed_loop_decode.wgsl`](https://github.com/BobertNeek/A-Life/blob/1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a/crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl#L374-L469):
  channel mapping and the representative override after independent motors.
  Blob `6f63a8e8eb1c49b0ac9755cf5c14fe1035b7da41`.

This is an implementation diagnostic, not architecture or a capability claim.
It neither selects live actions nor reads activations. No CPU neural fallback
is introduced. The utility adds independent files only and does not touch the trainer, critic
objectives, evaluator, replay schema, or runner. It can be integrated separately
from continued-critic qualification.

## Probability decomposition

For temperature T, let R be the masked representative softmax, M2 the masked
independent Manipulation softmax, and f(i) the motor slot forced by candidate i.
For a caller-defined set G of Manipulation candidates:

    P(motor2 in G) = sum(R[g], g in G)
                    + sum(R[r], f(r) != 2) * sum(M2[g], g in G)

The terms are disjoint: representatives that force slot 2 suppress its
independent draw. An empty independent motor mask contributes zero, never a
uniform default. The report also gives the probability that motor2 is empty.
Candidate probabilities plus the empty probability sum to one within floating
point rounding. Groups may overlap; do not add overlapping groups together.

Relevant edge cases verified against source:

- All finite candidates are representative-eligible, independent of enabled
  motor channels. NaN, positive infinity, negative infinity, and the NaN invalid
  sentinel are excluded from every mask.
- Move maps to 0, Look to 1, Interact/Write to 2, Vocalize to 3, and
  Hold/Rest/Inspect to 4. Idle/Gesture force 4 when representative but have no
  independent motor membership. No current ActionKind populates slot 5.
- Independent membership requires the corresponding enabled bit. A
  representative still forces its motor when that channel is disabled.
- A nonzero u32 enabled mask is required, exactly as in `build_receipt`.
  High enabled bits do not create otherwise nonexistent motor candidates.
- The receipt takes temperature and logits as f32, computes max-subtracted
  exp/sum/log in f64, then rounds each selected factor log probability to f32.
  The utility mirrors that narrowing and arithmetic, including sequential f64
  summation. Marginals use normalized f64 softmax weights without rounding them
  through the receipt's f32 log likelihood. Optional selected-factor outputs
  separately reproduce the receipt's f32 log probabilities and f32 joint sum.
- These are the factorized policy probabilities represented by receipt math.
  They are not a claim of bit-exact GPU exp/reduction behavior, finite 23-bit
  RNG frequency, independence of a deterministic seed family, or seed replay.

## Explicit neutral JSON contract

This input is newly defined for the utility. It is **not** an existing A-Life
native replay/checkpoint format. A future adapter must inspect actual source
and replay fields, bind candidate identities/order to one receipt, and export
this contract without guessing. A raw native replay must be rejected rather
than treated as a compatible file. No actual replay was accessible for this
deliverable.

The checked-in `fixtures/manipulation_probabilities.synthetic.json` is a
complete example. Every row is explicitly synthetic.

Top-level fields:

- `schema`: `alife-manipulation-probability-input-v1`
- `policy_source_commit`: the exact audited commit above. This asserts which
  policy contract to use; it does not verify the capture's provenance. If the
  capture comes from another revision, inspect the policy code before adapting
  it. Do not relabel an unverified revision as this commit.
- `temperatures`: nonempty array of objects with unique nonempty `label` and
  numeric `value`. Labels are preserved verbatim. Values are narrowed to f32
  and must be finite and at least f32(0.0001)
- `rows`: nonempty array of independent decision rows
- `metadata`: optional JSON metadata, preserved in the output

Required row fields:

- `row_id`: unique nonempty string
- `evidence_kind`: `synthetic` or `exported_replay`. The latter is labeled
  caller-supplied and not independently verified
- `behavior`: `OnPolicy` or `Demonstrator`, copied from the behavior evidence
- `sampling_temperature`: original receipt temperature, never the requested
  diagnostic temperature. Equal effective f32 temperatures are labeled
  collected sampling; other temperatures are explicitly counterfactual
- `enabled_channels`: original nonzero u32 enabled-channel mask
- `logits`: 1 to 32 entries, in original candidate order. A finite number is
  narrowed once to f32. For exact raw bits, including nonfinite values and
  signed zero, use an object such as `{"f32_bits":"0x7fc00001"}`. The value can
  also be a u32 integer. Only that key is allowed in a bit object. Bare JSON
  NaN/Infinity, null, booleans, and finite numbers outside f32 range are errors
- `candidate_kinds`: equal-length array of exact current ActionKind names
- `candidate_labels`: equal-length array of nonempty descriptive strings.
  These do not control classification or action identity
- `representative_mask`: original u32 bitmask; bit i means candidate index i
- `motor_masks`: six original u32 masks in slot order 0 through 5
- `forced_motor_slots`: one integer slot per candidate, including masked ones
- `queries`: nonempty array of objects with unique `label` and
  `candidate_indices`. Each list contains unique zero-based candidate indices
  whose kind maps to Manipulation. Use an empty list for an absent action.
  Identify Grab/Eat by the actual captured action payload, not by substring,
  action kind alone, decoder family, or this utility's labels. Multiple targets
  may be deliberately grouped. A masked candidate contributes zero

Optional row field `selection` contains the captured zero-based
`representative_index` and six `motor_indices`. An empty channel is null in
this neutral contract. A native adapter must translate receipt u16::MAX
(65535) to null, or encoded GPU byte zero to null and nonzero bytes to
index-minus-one, according to the actual source field it reads. Do not confuse
those two native encodings. The forced channel must contain the representative
even when disabled; other channels must select from their masks or be empty.
The utility checks these conditions and rejects a selected likelihood that
cannot be represented as finite f32, including at a requested counterfactual
temperature. It also deliberately rejects a nonfinite final f32 joint sum.
This last rule is stricter than `build_receipt`, which checks individual
factors but not its final sum; the separate OnPolicy gate rejects that
nonfinite joint. The diagnostic fails closed for either behavior kind rather
than emitting nonstandard JSON. Invalid input fails the whole report without
a partial success.

All supplied masks/forcing are recomputed from kinds, enabled bits, and
finite logits and must match exactly. There is no silent filtering of a stale
mask, extra bit, unknown kind, or contradictory force slot. An all-nonfinite
representative pool is invalid, not an empty/uniform policy. Duplicate JSON
keys, duplicate row IDs, and duplicate query/temperature labels are rejected.

Raw per-row input and normalized f32 logit bit patterns are retained. The CLI
also records SHA-256 of the exact input bytes. This preserves caller evidence;
it does not prove those bytes came from a genuine native replay.

## Run and interpret

From the package/repository root:

```sh
python3 -m unittest discover -s scripts -p 'test_audit_manipulation_probabilities.py' -v
python3 scripts/audit_manipulation_probabilities.py fixtures/manipulation_probabilities.synthetic.json --output synthetic-report.json
```

Omit `--output` to write JSON to stdout. The input is never changed. Output
files use exclusive create; an existing report or the input itself cannot be
overwritten. Exit code 2 indicates an input or I/O error. There is no network,
model loading, subprocess, installation, training, or GPU dependency.

For each query at each labeled temperature the report separates:

1. representative-forced probability
2. independent motor2 conditional probability
3. independent probability contribution, weighted by other-channel forcing
4. total Manipulation policy probability

For an optional selection, Demonstrator rows always have null behavior
likelihood. OnPolicy behavior likelihood is supplied only at the original
sampling temperature. Counterfactual selected policy likelihood is never
presented as collected behavior likelihood. No PPO or evaluation data is
rewritten.

## Qualification and remaining work

The repository regression suite contains 34 CPU-only tests. It uses synthetic
fixtures and exhaustive enumeration of 80 small factorized policies, including
disabled motors, empty channels, nonfinite logits, bit 31, temperature labels,
and demonstrations. Passing these checks proves this diagnostic arithmetic
and input handling within its test coverage; it does not prove a learned
behavior, native parser, replay provenance, or hardware sampler.

At preparation, the three source blob identities above were rechecked against
main `1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a`. The separately packaged
extracted-Rust qualification compared 3,027 synthetic result/error cases with
the exact receipt log-probability function; all matched at f32 bit level. That
comparison covers CPU receipt arithmetic, not hardware execution.

The dedicated `Offline Manipulation probabilities` workflow runs the 34 tests
and the synthetic CLI on Python 3.10 and 3.12. Its green result does not qualify
a native replay or the GPU sampler. When adapting another source revision,
reinspect all three sampler/receipt sources above before changing the pin;
unchanged test results alone do not establish semantic compatibility.

Actual Grab/Eat replay probabilities remain **Blocked** pending a source-bound
export of the PC replay evidence. The next authorized step is to inspect a
copied actual replay and implement a separate explicit adapter after verifying
its format, candidate ordering, semantic action identities, masks, enabled
channels, forcing, behavior kind, and original sampling temperature.
