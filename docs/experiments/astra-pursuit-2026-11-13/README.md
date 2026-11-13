# Astra: native pursuit and weight-space evolution

These are fresh experimental founders, evaluated by the actual CPU brain, world,
biology and somatic-learning path on main `e5649c9e`. No saved creature, acquired
brain, optimizer history, production default or Muse branch was changed.

The starting asset is `assets/foundations/experimental-anchored-v2-research.alife-foundation`.
It uses **GroundedObjectSlotsV1**, not the separate owner's newer
GroundedTerrainObjectPortsV2 navigation prototype. These results do not establish
that this candidate beats that prototype or a trained PC brain.

## What changed and why

Following [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/),
I inspected the actual weights and native activations, proposed a circuit edit,
measured physical behavior, then revised it. This is iterative search with
PyTorch, not a replication of the paper's restricted one-shot experiments.

The shipped asset has 32,768 weights, with only 42 nonzero ActionCandidate weights.
My initial six-direction search added opposite weights to signal/reference motor
pairs. Native traces showed the Approach pair had nearly identical activation:
the first-frame appearance sensitivity was only 0.0032 for the paired edit,
versus 1.4932 for the signal alone. The initial search was largely ineffective.
`decoder-analysis.json` contains the PyTorch calculations on frozen native data;
these are local sensitivities, not predictions of whole-life behavior.

The revised six directions edit Approach appearance and distance, its reference
contact coefficient, Ingest reference contact and distance, and Inspect
appearance. A hand proposal followed by two generations of eight antithetic
Gaussian challengers selected **22 changed genetic weights**. All other genetic
weights remain those of the starting founder. Native compiler/trainer assertions
verify that the edited values actually reach execution.

Selection uses two common sampling seeds, 128 intervals, five scenes, meals and
catches as the main score, plus time-to-meal and a penalty for unsuccessful
bundles containing Ingest in the toy scene. This is an evaluator fitness, never
a creature reward. `selection.json` identifies the winner. All 17 search results,
including losing candidates, are in `results.json`.

The first exploratory assay used a receding speed greater than the observed
creature stride. Before the reported search I corrected the task to catchable
motion and removed all taste cues. The initial exploratory scores are not mixed
into the reported comparison. Current target speeds are 0.012 units/interval
receding, 0.020 crossing, and 0.008 forward plus a sinusoidal lateral component
for zigzags. Capture/consumption stops scripted target movement.

## Completed held-out comparison

Ten new sampling seeds, 2026111400–1409, each have seven scenes and a 256-interval
limit. Both arms use the same founder genotype and biology preparation. These are
stochastic repetitions of specified layouts, not ten different species or ten
unseen mazes. Food and toy initially have identical physical features and neutral
taste; nutrition is discovered through actual interaction.

| Scene | Starting asset: meals | Pursuit edit: meals |
| --- | ---: | ---: |
| Nearby food | 10/10 | 10/10 |
| Distant stationary food | 0/10 | 10/10 |
| Receding moving food | 0/10 | 10/10 |
| Crossing moving food | 0/10 | 10/10 |
| Zigzag moving food | 0/10 | 10/10 |
| Food behind an obstacle | 0/10 | 0/10 |
| Nonfood toy | 0/10 | 0/10 |

The regression matters: the toy produced **680 unsuccessful Ingest-containing
bundles versus 52** for the starting asset. This is not a general-purpose
champion. It is a useful pursuit component with excessive persistence on nonfood.
The count describes the complete native bundle, not an invented per-channel
credit signal. Raw selected actions/outcomes are available with `trace:true`.

Consecutive actual positions and world ticks certify movement in the additional
motion controls. Before capture, receding/crossing/zigzag targets traveled
1.560/1.920/2.520 units; stationary and frozen-crossing controls traveled zero.
All five were consumed by the candidate. Targets follow predetermined paths
independent of the hunter until physical capture. Only ordinary grounded
perception reaches the brain; evaluator coordinates do not select its actions.

**Moving Food is a prey surrogate.** These results demonstrate pursuit and
capture/consumption, not implemented live-prey ecology, predictive interception,
maze solving, general intelligence or GPU validation. Target speed, appearance,
founder diversity and competing drives need broader tests. No default is promoted.

## Additive memory experiment

The next edit adds positive remembered-valence and negative remembered-danger
readout weights to the Ingest family: 80 MemoryContext weights, separate from
current sensory inputs. It leaves the existing runtime memory cap unchanged.
Gains 0, 4, 16 and 64 were screened on two selection seeds; gain 16 reached the
same retry reduction as 64 without losing meals. This gain was then fixed before
testing ten further seeds, 2026111500–1509, at 256 intervals.

| Second held-out comparison | Pursuit alone | Pursuit + memory |
| --- | ---: | ---: |
| Food meals across five scenes | 50/50 | 50/50 |
| Unsuccessful Ingest-containing toy bundles | 689 | 533 |

That is a **22.6% reduction** in futile toy bundles, still far too persistent.
The memory candidate has 102 edited weights in total. It has not been tested
against obstacles, competing needs or delayed praise/scolding in this batch.
Both candidates are retained as research components; neither replaces the
starting asset. `artifacts.json` records SHA-256 hashes of all three assets.

## Reproduce or continue

```sh
cargo build --release -p alife_training --features cpu-backend --example evolve_pursuit
python scripts/evolve_pursuit.py target/release/examples/evolve_pursuit \
  assets/foundations/experimental-anchored-v2-research.alife-foundation \
  /tmp/pursuit-search --generations 2 --population 8 --seeds 2 --ticks 128 \
  --start-params 4,0,-6,-24,24,0 --stop-file /tmp/pursuit-stop
```

The Python environment needs PyTorch; this run used CPU PyTorch 2.15.0+cpu.
The native executable also accepts one JSON request per stdin line. For the
selected weights, copy `params` from `selection.json` and use `seed_start`,
`seeds`, `ticks`, `scenes`, optional `trace`, optional `memory_aversion`, and an
optional **new** `export` path. It keeps no full-brain replay files. `--inspect`
after the foundation path exports weights and synapse coordinates for PyTorch.

For example, this reproduces the second held-out memory arm in one request:

```sh
echo '{"params":[3.5628414154052734,-1.1101834774017334,-10.744718551635742,-23.066162109375,23.09345245361328,0.6747460961341858],"memory_aversion":16,"seed_start":2026111500,"seeds":10,"ticks":256,"scenes":["contact","stationary","receding","crossing","zigzag","toy"]}' | \
  target/release/examples/evolve_pursuit assets/foundations/experimental-anchored-v2-research.alife-foundation
```

Use `memory_aversion:0` for its paired control. For the first comparison, use
seeds 2026111400–1409, add the `obstacle` scene, and compare these pursuit
parameters against six zeros with memory gain zero. To evaluate an exported
candidate directly, supply that asset and use zero perturbations.

The stop file and optional `--allowance-file` (JSON `remaining_percent`) are
checked between candidate evaluations. A parent-observed value at or below 40
stops new candidates and retains the best parameters and all completed results.
This worker cannot read Codex allowance directly; absent fresh parent data, this
mechanism is not an independently verified quota monitor. No reset is redeemed.

Focused validation: three native tests cover all six real mutation directions,
compiled weight admission, consequence-only memory edits, and matched initial
food/toy perception. Strict Clippy passes. The full suite and GPU tests were not
run because this addition is an offline example and search tool.

After rebasing onto `18ba1e20` (#340), the example was rebuilt and all three tests
and strict Clippy passed again. The seven baseline and seven pursuit trials for
seed 2026111400, plus six memory trials for seed 2026111500, reproduced every
recorded trial metric and candidate digest exactly. Python compilation and a
parent-allowance-file smoke test passed: 40% remaining launched zero evaluations.
