# Cumulative pursuit, motor channels and memory

The common parent is the PR341 `pursuit-memory16` foundation, SHA-256
`8f84afd6ac2ec575a8df61d18a0a18ac17296615a9ef7af7d22df416b3dd0ad7`.
Every later candidate below edits that parent, or the explicitly exported winner.
Original checkpoints and founders remain intact. These are fresh research lives
on the native CPU host, not production defaults or a trained-PC-brain migration.

Following [Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/),
I read weights, inspected native activations and action masks, calculated decoder
sensitivities in PyTorch, proposed edits, and tested their physical consequences.
The experiment includes feedback-driven selection; it is not a replication of
the paper's restricted one-shot experiments.

## Latest cumulative continuation

The current working child is **`pursuit-twohead-contact20.alife-foundation` plus
its exported count2 construction genome**. It starts from the exported bearing4
child and changes one additional genetic weight: +20 on Ingest source1839,
contact lane18. Its distance penalties and all memory/recurrent weights stay intact.

On ten further paired cases (2026112700–2709), both parent and child ate98/110
food targets with the same per-scene success counts. Unsuccessful Ingest-containing
bundles across all scenes fell3062→2093 (31.6%); toy Ingest selections fell1596→922
(42.2%). Total time-to-meal across the same98 successful cases rose5470→5822
intervals (6.4%). The exact export was reloaded for a full matched case.

The preceding broad Ingest scaling experiment was rejected: scaling all readouts
to1/8 also retained98/110 meals and reduced toy attempts1574→1199, but weakened
distance penalties and increased all unsuccessful Ingest bundles3040→4579.
This is why the accepted child edits contact weights alone.

New hazard controls are separate exploratory tests, not included in the food
success table. Exposure begins after ordinary600-tick maturation; placing hazards
beside unattended newborns during preparation killed them before any neural
trial, so that invalid setup was corrected before reporting behavior. All three
brains survived the short revised controls but accumulated substantial injury.
The contact edit improved hazard→food outcome in one of four cases and reduced
injury in those mixed cases. A carried painful object remains a weakness:
Inspect/Approach/Flee/Eat/Grab/Play provide no explicit voluntary Drop action.
No world behavior or reflex was modified here. This finding was relayed to Bob.

Assay v5 records survival and stops at death. Seven focused native tests and
strict Clippy pass, including actual hazard exposure after preparation. The older six-test statements
below describe the already-merged first stage.

## Verified improvement

Ten fresh paired cases (sampling seeds 2026112500–2509), four appearances, five
headings and three motion speeds, 256 decision intervals each. There are eleven
food scenes and one nonfood scene per case. This is a small varied test set with
one inherited genotype, not ten independent species or a maze benchmark.

| Result | Previous parent | Two motor heads only | Heads + targeting edit | Also cap8 + memory32 |
| --- | ---: | ---: | ---: | ---: |
| Nearby + stationary food | 20/20 | 20/20 | 20/20 | 20/20 |
| Five moving-food scenes | 38/50 | 42/50 | 50/50 | 50/50 |
| Partial obstacle | 0/10 | 2/10 | 6/10 | 6/10 |
| Moving target behind obstacle | 0/10 | 0/10 | 3/10 | 3/10 |
| Food among a nearby toy | 0/10 | 0/10 | 8/10 | 8/10 |
| Toy experience, then new food in same life | 10/10 | 10/10 | 10/10 | 10/10 |
| **Total food meals** | **68/110** | **74/110** | **97/110** | **97/110** |
| Toy Ingest selections (lower is better) | 700 | 1956 | 1594 | 1324 |

The first-stage child is `pursuit-twohead-bearing4.alife-foundation`
**together with its exported construction genome**. The independent reload
reproduced the candidate digest and every trial metric for a complete twelve-scene
case. The subsequent contact-weight generation above started from that exported
child, with zero additional targeting deltas. This is cumulative progress.

The gain has a cost: nonfood eating attempts increased. The child wins the
meal-dominant selection objective, but is not a universal behavioral improvement.
It remains a research candidate; the parent is retained as a control. The cap8
combination is an isolated research variant, not merged runtime behavior.

## What the edits actually do

The old parent has `motor_head_count=1`: independent locomotion only. Manipulation
occurs only when an Ingest/Contact/Other candidate wins the global representative
sample against Inspect and Approach. Native traces confirmed zero manipulation
mask. Setting both inherited architecture homologs to v1 count2 enables the
already-implemented Manipulation channel. Every genetic weight and synapse
coordinate is retained by this chromosome edit. The foundation bytes alone do
not encode this head-count difference; the exported genome is essential.

The targeting edit cancels our earlier Approach appearance-sum addition, changes
its source1776 distance addition from -1.1101835 to +48, cancels our added
source1791 contact penalty, and adds +4 to source1791 bearing cosine. Existing
Ingest, MemoryContext and recurrent weights remain intact. This reduces the
preference for bright obstacles and gives a directional preference during
pursuit. It does not insert a path planner or target label.

Bearing edits alone were insufficient. Fresh earlier controls gave only69/100
food meals versus65/100 for the parent, and lost contact eating behind the body.
Tracing showed suppressed Approach and dominant Inspect logits. Stronger Ingest
contact weights and reduced Inspect weights repaired parts of this but did not
solve the wider target-choice problem. Enabling the existing manipulation head
made the combined targeting edit substantially more useful.

The enabled channel draws a candidate whenever its mask is nonempty, including
the existing targetless NO_MANIPULATION option. The higher toy retry count comes
from relative readout scores, not a missing no-op (an earlier explanation was
incorrect). Later experiments soften the Ingest contact readout now that it need
not win the global contest, while preserving distance penalties. No sampler, world legality or reward rule was
changed to make these scores succeed.

## Memory cap experiment

The memory readout is capped at0.5 in the current compiler. Raising genetic
memory weights alone saturated that cap: total gains16/32/64 gave identical
three-life sequential controls. A separate worktree changed the cap to8 and
rebound the same32768 weights after comparing every source, target, kind and
route. This changes compiler identity; it does not bypass asset validation.

With unchanged motor weights, three toy lives produced159 unsuccessful eating
bundles under the old cap, versus110/48/9 with cap8 and gains16/32/64. All15 food
controls succeeded, including toy→new-food lives. Strong suppression delayed
new-food eating in two lives. A wider110-trial comparison retained65 food meals
while toy failures fell674→579→419 for old cap, cap8/gain16 and cap8/gain32.

These weights use **family valence/danger**, lanes32/34. They are not target-specific
knowledge. Target memory lives on lanes24–31 and must stay distinct from present
sensory input. Bob's separate terminal-consequence context work addresses that
more specific problem.

`memory-cap8.patch` and `pursuit-memory16-cap8.alife-foundation` preserve the
isolated experiment. Apply the patch only in a separate checkout of this source.
It deliberately changes the compiler's expected memory identity and is **not** a
compatible global update for existing builtins/saves. Its focused tests use the
rebound research fixture. No production/default cap change is included.

## Evidence and reproduction

`results.jsonl.gz` contains completed requests and compact native outcomes,
including rejected pilots; it omits bulky neural traces. Each result identifies
its actual source and candidate digest, parameters, scenario settings and runtime
variant. `artifacts.json` identifies exported files. `export.json` records the
bearing child's export; `contact-export.json` records its contact20 successor.
`bearing-sensitivity.json` and `analyze_native_trace.py`
show PyTorch calculations on frozen native states, not whole-life predictions.

```sh
cargo build --release -p alife_training --features cpu-backend --example evolve_pursuit
python docs/experiments/astra-generalization-2026-11-13/reproduce.py \
  grid-bearing-0 target/release/examples/evolve_pursuit \
  docs/experiments/astra-pursuit-2026-11-13/pursuit-memory16.alife-foundation \
  --output /tmp/bearing-case.json
```

Use `winner-reload` with the bearing4 asset, or `contact-winner-reload` with
contact20, to verify direct continuation.
The request includes `motor_heads:2`; without that gene, loading the weights alone
would test a different creature. `--trace` retains neural/action diagnostics.
The search script supports fixed `--motor-heads`, `--bearing-gain`,
`--readout-scale` and `--variation`; further weight mutations should load the
selected exported asset rather than reconstruct an independent founder.

The first-stage six focused native tests and strict Clippy passed. Both normal
and cap8 research variants passed those six tests. Python scripts compile. GPU
execution, long-life care and the full repository suite were not tested here.
The later hazard controls are described above and do not establish injury avoidance.
Hosted Actions cannot currently run because the private repository's included
minutes are exhausted and its spending budget is zero, per the owner's board
report; these are local validation results, not a CI-green claim.

Moving Food remains a prey surrogate. Orbiting, fleeing, partial-obstacle,
food/toy choice and continuous toy→food scenes broaden the original assay but do
not establish live-prey ecology, full maze solving or general intelligence.
The old navigation reference from #343 requires additional legacy host modules
not present on main. Its blob has not been misrepresented as a fair current-host
comparison. Matching source recovery was requested; other experiments continue.

The parent monitors Codex allowance. This worker cannot directly query it.
Parent-maintained allowance<=40 or the stop file stops new candidates between
requests. No banked reset is redeemed.
