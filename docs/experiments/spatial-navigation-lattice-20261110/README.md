# Experimental spatial lattice and terrain-vision probe

This is source-only research, based on the opt-in authored experimental V2 brain. It does not read or replace acquired lives or optimizer histories. Terrain vision intentionally changes sensory semantics and does not expose an innate GPS; genetic weights are transferred to a fresh research phenotype only after verifying every synapse coordinate. This is not a migration.

The probe tests a clear target and mirrored opaque-wall detours. It asserts the food is absent from initial action candidates when hidden. The research candidate adds lateral visual-to-context-to-turn paths, an exploratory visible-object approach bias, and a 4x4 local lattice using a reciprocal predictive/context connection block. This is not yet a learned occupancy map, goal-conditioned pathfinder, or demonstrated maze solver.

A detached replay-only pulse/ablation diagnostic injects a pulse into one neuron and compares the response with the local lattice edges silenced. It does not feed information into a live creature. One smoke trial showed a small response in neighboring context cells that was nearly absent under ablation and decayed over later replay steps. Eight-decision runs are not behavioral validation. A preceding 120-decision test of the simpler trace-only edit found no food consumption in two clear and four wall scenes per arm; the combined and lattice candidates require longer comparison and safety controls.

Reproduce the smoke test from the repository root, using a fresh output directory:

```sh
ALIFE_RESEARCH_NAV_TERRAIN=1 ALIFE_RESEARCH_GRID_PULSE=1 cargo run -p alife_training --features cpu-backend --example probe_spatial_navigation -- <new-output-dir> 8 0 nav-lattice-factor 1 2026111821 maze-clear 0.4 0 0
```

Inspiration: https://arxiv.org/abs/2301.06820 and https://arxiv.org/html/2410.18832v2. Both papers use supplied spatial representations; this probe must derive sensory data from the simulated creature’s view and cannot assume a hidden complete map. CPU probes are not RTX or gameplay qualification.

## Egocentric input and care probes

A later candidate connects eight distinct observed range-ray encoder channels into four adjacent lattice input cells and increases local self-recurrence. A detached pulse/ablation smoke test showed a small, decaying local response. This is an egocentric sensory trace, not an allocated global map, occupancy mask, goal gradient, or validated maze solution. Normal mirrored-wall tests still hide the target from initial visual candidates. A stronger visible-object exploration edit ate far food in one 120-decision paired open trial where the baseline did not, while neither ate the neutral target or solved either wall in that small trial; this is not a reliability or safety claim. Contact-bias alternatives and actual hazardous food are under test. Never promote these results into a trained life or infer learning without controlled retention tests.

**Bilateral correction:** The earlier saved version sampled indices 0–7 from a 16-ray vision fan, covering only half the field. The current probe samples indices 0, 2, 4, 6, 9, 11, 13, and 15, with steering paths from rays 3 and 12. These are direct sensory-to-recurrent weight edits in a fresh research candidate. A 16-decision mirrored-wall smoke test confirmed hidden food stayed out of initial visual action candidates, but did not establish correct detours. Longer and held-out tests remain required. Earlier half-field results do not validate this corrected circuit.

## Connected steering probe

A later check found the first side-steering paths were zeroed by the lattice rewrite. The probe now restricts those paths to edges the lattice preserves and assigns separate side cells. A detached pulse changed left-minus-right turn logits in opposite directions for the two side rays. In one 16-decision mirrored-wall pair, a stronger turn-readout edit displaced the creature toward each open side; neither run reached the target. Longer paired trials and food/hazard controls are running. This is not proof of learned spatial mapping, stable memory, robust maze solving, or hardware/gameplay readiness. The selection only edits fresh genetic weights; it does not load or overwrite a trained life.

## Stronger preserved relays

A fresh research variant raises four lattice-compatible weights across two side-relay pathways from 0.06 to 0.75 while retaining local connectivity. In a detached replay with a 0.25 sensory pulse, the turn-logit response was more than 100 times larger than the comparison in that one case, with opposite signs for opposite rays. In one 16-decision mirrored-wall pair, the stronger variant moved toward the opening in both scenes whereas its paired comparison moved toward the same side in both; neither reached the food. This is a small pilot, not evidence of a map or reliable maze competence. A 120-decision paired comparison with food and actual harmful-item controls is running separately; no outcome is assumed.

## First occluded detour result

In seed 2026111840, the stronger turn-readout candidate reached and consumed food behind the left-gap wall within 120 decisions. The food was not initially visible; it first appeared in the candidate list at decision 72, and the creature crossed x=1.3 at decision 87 at z≈1.005, beyond the wall’s z=0.8 end. The matched lower-gain candidate did not reach it. Neither candidate consumed the target in the mirrored right-gap scene. All four trials reported exact cold-next-step continuation. See [the compact receipt](detour-1840-summary.json); raw fresh-research receipts are retained separately. This is one seed, not evidence of a learned map or a general maze solver; more seeds and random-behavior controls are needed.

## Additional paired trials

In seed 2026111841, the lower-turn-gain comparison reached the left-gap target while the stronger-turn candidate did not; neither reached the right-gap target. Together with seed 1840, each candidate has one left-gap success in two seeds, and neither has a right-gap success. In a separate seed 1842, both the stronger-turn candidate and the boosted-relay variant ate open and neutral food, avoided a bitter item with hazard pain 0.3, and failed to consume either hidden target within 120 decisions. The boosted-relay candidate first saw the right-gap target at decision 113, so a 240-decision follow-up was started to distinguish a timeout from a persistent failure. [Compact results and receipt hashes](detour-1841-1842-summary.json). These are tiny CPU tests, not learned-map or generalization proof.
