# A-Life v1.1 repaired stimulus-to-response trace

Status: source trace ready for independent review. It is not behavioral proof.

```text
world + canonical organism biology
        |
        v
grounded peripheral sensing
        |
        v
stable-ID attention -> bounded focal reacquisition
        |
        v
canonical interoception + memory + target concepts/gaps
        |
        v
candidate-specific grounded successor predictions
        |
        v
GPU neural activation
  + dendritic conjunction gates
  + active topology/memory/prediction context
        |
        v
six parallel motor-channel selections + shared eligibility
        |
        v
one atomic registered-world motor transaction
        |
        v
measured body/biology/world successor
        |
        v
grounded outcome + value + true RPE + one joint modulator
        |
        v
GPU fast plasticity + memory + predictor + topology observation
        |
        v
bounded structural growth/pruning and dendritic allocation when due
        |
        v
hardware-independent cognitive work receipt
        |
        v
optional world/species energy/fatigue/heat conversion
        |
        v
sleep/replay atomic consolidation when biologically due
        |
        v
exact checkpoint + stable-ID live presentation
```

## Ordered production path

1. **Canonical sync.** Gameplay and Era1 supply the same registered `WorldOrganismRecord`; no runner-local homeostasis or development clock is authoritative.
2. **Peripheral sensing.** The world produces grounded broad object facts and legal action proposals. Ordinary construction uses grounded object slots, not privileged affordance scoring.
3. **Focal attention.** Stable target identities choose a phenotype-bounded subset for richer grounded reacquisition. Focal width and target count come from phenotype policy and consume work.
4. **Cognitive context.** Canonical hunger, fatigue, pain/injury, temperature stress, sleep pressure, energy, and brain ATP join episodic memory plus target-specific concept/gap context.
5. **Prediction before choice.** `GroundedSuccessorPredictor` evaluates bounded candidate motor bundles against one stable semantic state schema. It supplies consequences and uncertainty, not desirability.
6. **GPU decision.** Active context reaches the learned GPU path. Sparse dendritic conjunctions gate neural activation. Six motor channels select commands and parameters in parallel.
7. **World consequence.** `HeadlessWorld::apply_registered_neural_command` validates and applies the joint bundle atomically to body, biology, speech, resources, and world state.
8. **Outcome seal.** The shared step compares canonical before/after state and records grounded successor, homeostatic change, raw reward, value expectation, true RPE, pain, injury, novelty, residual, and social outcome.
9. **Learning commit.** One joint neuromodulator applies to every causally active channel eligibility. Fast plasticity remains separate from slow normalization.
10. **Memory, prediction, and topology.** The sealed patch updates episodic memory, predictor/value state, concepts, contradictions, unresolved gaps, and bounded structural evidence.
11. **Structural work.** Event-nominated sparse candidates compete for fixed capacity. Accepted edges and useful nonlinear branches change later GPU computation; concept induction remains a separate system.
12. **Cognitive economics.** Work counts attention, prediction, topology, multi-channel motor, plasticity, structural maintenance/growth/pruning, and sleep. Optional biological conversion is transactional policy.
13. **Multirate world tail.** Each due metabolism, ecology, development, lifecycle, and sleep subsystem advances once on its configured cadence; the shared cognition step does not force one global clock.
14. **Sleep.** A biologically scheduled immutable replay snapshot stages memory, predictor, concepts/gaps, structural/dendritic work, and GPU consolidation. CPU/GPU publication commits together or restores the checkpoint.
15. **Persistence and presentation.** Exact save/load binds all acquired cognition to organism/world/phenotype/runtime identities. Live snapshots publish the same stable organism IDs to transforms, labels, selection, and inspectors.

## Shared authority boundary

- `crates/alife_runtime/src/causal_step.rs`: `ProductionCausalStage`, `run_production_causal_transaction`, and `run_production_causal_step` define the ordered semantic spine.
- `crates/alife_game_app/src/gpu_live_runtime.rs`: gameplay calls the shared transaction and owns cadence, persistence, presentation, and runtime lifecycle.
- `crates/alife_training/src/era1_trials.rs`: Era1 calls the same transaction and owns only scenario/control configuration and evidence collection.
- `crates/alife_world/src/headless.rs`: the world owns grounded sensing, legality, and authoritative body/biology consequence.
- `crates/alife_runtime/src/checkpoint_assets/state_codec.rs`: exact checkpoint restore validates identity and acquired-state completeness before publication.

## Architecture checks for the reviewer

- Confirm no host callback chooses an action, supplies a desirability score, or fabricates per-channel reward.
- Confirm prediction and concepts/gaps affect predecision neural context rather than post-hoc diagnostics.
- Confirm acquired state never enters ordinary genetic inheritance.
- Confirm sleep and exact load cannot partially commit CPU, GPU, world, or schedule state.
- Confirm every bounded search remains sparse and non-quadratic in neuron count.

