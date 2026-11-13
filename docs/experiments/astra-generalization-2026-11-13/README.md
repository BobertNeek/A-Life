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

The main-compatible working child is `pursuit-twohead-bearing4.alife-foundation`
**together with its exported construction genome**. The independent reload
reproduced the candidate digest and every trial metric for a complete twelve-scene
case. The next readout-scale generation starts from that exported child, with
zero additional targeting deltas. This is cumulative progress, not a reset.

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

The channel also has a limitation: the existing sampler draws an interaction
whenever its mask is nonempty; it has no separate empty-channel draw. Toy retry
counts therefore rise. Later experiments soften the Ingest readout now that it
need not win the global contest. No sampler, world legality or reward rule was
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
working child's export. `bearing-sensitivity.json` and `analyze_native_trace.py`
show PyTorch calculations on frozen native states, not whole-life predictions.

```sh
cargo build --release -p alife_training --features cpu-backend --example evolve_pursuit
python docs/experiments/astra-generalization-2026-11-13/reproduce.py \
  grid-bearing-0 target/release/examples/evolve_pursuit \
  docs/experiments/astra-pursuit-2026-11-13/pursuit-memory16.alife-foundation \
  --output /tmp/bearing-case.json
```

Use `winner-reload` with the exported two-head asset to verify direct continuation.
The request includes `motor_heads:2`; without that gene, loading the weights alone
would test a different creature. `--trace` retains neural/action diagnostics.
The search script supports fixed `--motor-heads`, `--bearing-gain`,
`--readout-scale` and `--variation`; further weight mutations should load the
selected exported asset rather than reconstruct an independent founder.

All six focused native tests and strict Clippy passed. Both normal and cap8
research variants passed the six tests. Python scripts compile. GPU execution,
long-life care, hazards and the full repository suite were not tested here.
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
