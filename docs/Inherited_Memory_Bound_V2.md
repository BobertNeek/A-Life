# Inherited memory contribution bound (opt-in)

The legacy candidate memory bound remains 0.5. A fresh genome may explicitly opt into architecture schema 4 with `with_memory_candidate_bound_v4(bound)`. This preserves its existing motor layout and changes only the bound on the combined target/family memory contribution. CognitiveContext stays bounded at 0.5. It does not add punishment, a new learner, a sensory channel, or an isolated action reward.

The bound is a finite nonnegative genetic parameter. Inheritance maps it in f32 log1p space; no behavioral upper ceiling is imposed. Either parent can supply the opt-in, with a legacy allele contributing 0.5 and the existing receiver-selected motor layout retained. Exact identity/selected alleles preserve their original bits; intermediate crossover/mutation values remain quantized by the existing f32 mapper. Larger bounds are not necessarily useful or safe. Finite parameters do not guarantee finite arithmetic: plan2 rejects nonfinite raw auxiliary sums and ordered composed logits rather than hiding overflow behind a clamp; GPU training guards the existing accumulation/optimizer writes. Legacy arithmetic remains unchanged. Zero disables the memory contribution while retaining its representation and ordinary learning. Saturated decoder terms have zero local adjoint under the existing strict interior derivative convention. Genetic mutation, rather than the neural weight optimizer, changes this parameter.

Schema 4 retains the previous motor layout (1, 2 or 3). The compiled memory channel uses version 2 with the existing lane layout and packed field. Legacy schemas emit no new fields or digest additions. Genome and phenotype identities change for the opt-in. Existing replay-bound and compiled-identity checks must agree; a weight-only live update cannot silently change the architecture. Old readers reject the new versions. Do not edit an acquired checkpoint to activate the feature. The frozen N4096 growth recipe rejects the extension rather than discarding it.

CPU and WGSL forward/adjoint paths use the inherited memory bound separately from cognitive influence. Runtime context headers retain version 1. The compiled memory plan is versioned; packed training uses its existing version word to distinguish the new split from legacy behavior, without adding a field or changing the word layout. Legacy no-memory-channel cognition remains disabled. Actual GPU execution must be qualified separately from shader parsing and host-side validation.

## Experimental status

Focused qualification covers public reciprocal genome expression, compiled wiring and cold identities, CPU forward/adjoints and overflow rejection, GPU host packing and WGSL validation, and native plan-2 row roundtrip through matching replay and a cloned scratch optimizer update. Retained original founder/Adam state is unchanged. The full CPU library suite passed with its existing ignored tests. This is not a full-workspace, hosted-CI or GPU-device execution claim. No default has changed and no fresh body result establishes improvement. A held-state diagnostic on the current experimental parent found that a bound of 1.0 reduced modeled toy Ingest selection while preserving modeled Play/Grab opportunities; it does not establish fewer real attempts, better meals, or specific causal action learning. Fresh matched controls must include unfamiliar edible objects, unsuccessful reach, toy interaction, hazard, and all four player feedback pathways before adoption. Joint-outcome memory remains joint evidence, not a decomposition of the physiological consequences of each participating action.

## Collected-row identity

Plan-2 CPU rows use a `memory_plan2_record_v1` JSON envelope carrying the
compiled phenotype hash and memory bound alongside the original row. The new
reader admits a matching envelope; old flat-row readers reject it structurally,
even when the bound is 0.5 or all auxiliary contributions are zero. Both raw
training-window construction and sealed-shard admission check the recorded
neural phenotype and plan metadata against the supplied founder. Matching
plan-2 rows remain training inputs, not permanently diagnostic-only records.
Legacy rows retain their flat serialization and field omissions. Plan-2 raw-row
binary serialization is explicitly unsupported and returns an error; callers
must use the versioned JSON wire. This does not alter checkpoint formats.

The boundary is necessary: a retained pre-feature reader accepted two actual
plan-2 native rows with unchanged inputs, actions and selected logits, then
stamped their training window with a different supplied legacy founder identity.
The original witness is retained locally. This is an identity-admission defect,
not evidence of a behavioral gain or of an optimizer update having occurred.
An actual pre-change bincode-2-standard check on a native legacy row encoded
successfully but its old derived reader failed (`UnexpectedVariant` for an
optional field). Therefore this change does not claim a working legacy binary
row roundtrip. It preserves the legacy serialization behavior and makes new
plan-2 binary serialization explicitly unsupported instead of silently emitting
an old interpretation.
