# Neutral contact: opt-in CPU comparison

This tooling reproduces a narrow intake improvement **and its repeated nonfood
failure cost**. It does not promote a candidate, replace a default brain, change
sensing/legality, or read an acquired life. Fed-state eating is a measured
tradeoff, not automatically a defect; no appetite suppression was added.

## Qualified experiment

The frozen 384-trial study used main `1c74ee33528f452895ae255caaac916b722d909c`
and the two reviewed source files from commit
`845cee6f6b39b588d81e0de3f2a190ad5f297144`. Original source-patch SHA256:
`2880dde9759c5dad72a22ef4916a8ffb6b34b394e65f24114c5503afc86442c0`.
The original comparison remains pinned; the published helper corrects proposal
comments without changing the authored numeric recipe.

All arms use the **old experimental N2048 graph, GroundedTerrainVisionV1**.
They are not AnchoredV2, ObjectPortsV2 or signed MotorProposal V3. The publication
port is based on main `c1faa4f9604a7dcec0f95b66921c99cfb7bf6a4b` after #325;
golden canonical-hash tests check that all three historical candidates remain
identical. The full 384 trials were not rerun on that later commit.

The factorial is three arms × two body states × neutral/bitter × Food/ActivityToy
× contact/distant × eight sampling seeds, with 48 decisions per fresh clone.
Founder/world seed: 2026111701. Sampling seeds: 2026111901–2026111908. These are
sampling repetitions of one founder and matched physical scenes, not founder
or scene generalization. CPU results are not GPU qualification.

Food and toy have identical explicit RGB, material, shape, radius 0.5 and physical
chemistry. Nutrition is world-owned hidden truth. Initial distances are 0.25 and
2.5. Physical chemical0 is 0 or -0.85; hazard pain is zero. Actual calibrated
bitter contact taste is -0.862749994, and precontact taste is zero. Legacy
GroundedObjectSlotsV1 does not apply that contact gate in the pinned source and
is explicitly excluded from the qualified matrix.

Matched 1,200 ordinary preparation ticks produce:

| State | Energy | Hunger | Encoded hunger |
|---|---:|---:|---:|
| Hungry | 0.178907737 | 0.865803301 | 0.233464941 |
| Fed | 0.998374760 | 0.251193911 | 0.055804439 |

Fed means near-full body energy and the species' baseline hunger, not zero
hunger. Actual initial state, contact and chemistry are checked/recorded.

## Results and limits

Counts are per eight trials; energy change covers all 48 decisions. Consumption
latency is a one-based decision number, conditional on consumption.

| Measure | Baseline | Contact-only | Full-v2 |
|---|---:|---:|---:|
| Hungry neutral-contact food consumed | 2/8 | 8/8 | 8/8 |
| Hungry mean energy change | +0.030051 | +0.131767 | +0.131781 |
| Hungry mean consumption decision | 15 | 3 | 2.875 |
| Fed neutral-contact food consumed | 1/8 | 6/8 | 6/8 |
| Fed mean energy change | -0.007162 | -0.006947 | -0.006931 |
| Hungry neutral-toy failed Ingest selections | 2 | 57 | 57 |
| Hungry longest consecutive failed Ingest run | 1 | 21 | 16 |
| Fed neutral-toy failed Ingest selections | 1 | 14 | 17 |
| Fed longest consecutive failed Ingest run | 1 | 3 | 3 |

Bitter food consumption is 0/8 in every arm/state. This does not mean zero
attempts: fed bitter food has one Ingest selection per arm. No initially distant
target is contacted or consumed. All 64 matched distant baseline/contact-only
pairs have identical serialized decision rows and final body state. Mean
progress toward the distant target is negative: hungry -0.4902/-0.4902/-0.4927;
fed -1.8089/-1.8089/-1.7559. The full-v2 appearance changes do not establish
navigation improvement here.

Nonfood failures have sealed world failure outcomes and manipulation Touch,
not Consumed. Legality alone prevents nonfood consumption, so that zero count is
not selectivity. Initial exploration of indistinguishable neutral objects can be
reasonable; repeated failure after feedback remains the qualification limit.
Contact-only's mean extra hungry nonfood energy cost is small (about 0.000057),
but that does not erase the repeated attempts.

Contact-only changes exactly two of 32,768 weights: index 27168/source 1824,
Ingest lane18, 0 to +8; index 27183/source 1839, 0 to -42. Full-v2 changes 1,092
weights, including generic RGB/material/shape lanes 6–14 and memory readouts.
These are confounds, not independent improvements. Signed activations mean raw
weight signs do not establish an effective distance slope. Memory remains
family-isolated, with the existing +/-0.5 contribution clamp and same-object
fallback caveat. No creation-order cause or cross-family knowledge is claimed.

All 384 trials pass exact candidate import/re-export, unchanged trainer/Adam
checkpoint, and exact cold continuation. An independent audit checked 8,574
precontact target-candidate observations and 4,092 contact observations. Resident
neural credit and memory observe ordinary sealed world feedback; the founder
trainer is not optimized during this study.

## Reproduce from checked-in sources

No private candidate, tensor, checkpoint, or life file is needed. The example
constructs a fresh founder from the repository's checked-in built-in N2048 asset
and authors the three candidates using the included source recipe. Coordinate
checks reject incompatible graphs. Outputs require a new directory and include
local authored candidates and traces; do not upload them as source-only patches.

```sh
cargo test --locked -p alife_training --features cpu-backend \
  --example probe_neutral_contact_review

cargo run --locked -p alife_training --features cpu-backend \
  --example probe_neutral_contact_review -- \
  output-hungry 48 0 neutral-matrix 8 2026111901 \
  neutral-food,bitter-food,neutral-nonfood,bitter-nonfood,far-neutral-food,far-bitter-food,far-neutral-nonfood,far-bitter-nonfood \
  0.18 1200 0
```

Repeat with a distinct output directory and initial energy `1` instead of `0.18`
for fed. For practical CPU speed, set `CARGO_PROFILE_DEV_OPT_LEVEL=2`,
`CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_INCREMENTAL=0`.

The focused tests preserve exact candidate identities and a deterministic
neutral-intake/nonfood-failure witness. The failure witness documents a research
limit, not desired production behavior. No threshold-based suppression or
semantic food oracle is introduced. Broader scenes/founders, longer lives, GPU
parity and investigation of same-object failed-ingestion persistence remain
separate work.
