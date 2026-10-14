# Opt-in decoder lifetime-rate policy candidate

Date: 2026-10-14 UTC  
Architecture: A-Life v2.0 CONTROLLING  
Base: PR #40, `d1ee947fad980d4adcdfd08cb85b0068a73aef80`

## Scope and defaults

`PlasticityGenomeParameters` has an optional `DecoderLifetimeRatePolicyV1`.
Absent policy preserves the old JSON genome serialization and compiler identity
and keeps every decoder scale at 1.0. Founder constructors and their rate constants
are unchanged. This candidate adds a control, not a selected founder/training
policy or a claim that basic motor competence is separately represented.

The policy uses the semantic `DecoderHeadKind` and `CandidateActionFamily`
enums. Four head scales and at most eight optional ActionCandidate-family
scales are finite and bounded in [0, 1]. A family override replaces the
ActionCandidate head scale; it cannot affect the same family in speech, memory,
or cognitive heads. `None` removes a family override. SpeechPayload,
MemoryContext, CognitiveContext, and Other ActionCandidate choices retain scale
1.0 unless explicitly configured. Other includes several affordances; it is
not a speech-only selector.

The policy is a coherent inherited block at the existing named credit-profile
position. Expression and gamete construction reuse the same parent/homolog
selection already used for the credit profile. No selector, mutation/training
algorithm, GPU layout, ABI revision, or generic policy language was added.

## Receptor semantics

Only the decoder branch of the existing receptor-scale resolver changes.
The chosen scale enters the existing developmental multiplier and receptor
construction: `effective_scale = clamp(scale * critical_period_multiplier, 0, 1)`.
Thus critical periods can increase a positive configured scale (0.5 may become
1.0 at the maximum boost), while an explicit zero stays zero. Learning,
normalization, and replay rates share that existing effective scale. Zero rows
use the existing disabled receptor and are excluded from replay capture.
Eligibility decay and fast-weight bounds do not change. Reduced positive scales
change those local rates; they do not scale the global sleep staging or fast-decay
parameters.

Recurrent receptor construction, alpha, inherited weights, topology, and
genetic/acquired weight mixing remain unchanged. No live CPU neural update or new runtime dispatch is
introduced. The policy is bound into compiler-input identity; compiled rates
remain bound into the existing plasticity-plan and phenotype identities.

## Evidence and limits

Focused CPU/compiler tests cover exact-base default serialization/phenotype
hashes for N512, N1024, and N2048; zero/reduced selected families; independent
ActionCandidate, SpeechPayload, and MemoryContext head selection; unchanged
recurrent/Other/speech/memory rows; replay exclusion; critical periods; validation;
JSON round trips; stale-input rejection; coherent expression and reproduction.

CognitiveContext exists only in the optional strict Nano512 candidate ABI in
this base. That public admission reconstructs an exact default genome, so it
currently rejects a customized decoder policy. A private core unit test
qualifies CognitiveContext through receptor construction using test-only
coordinate substitution. This does not establish public candidate admission
for that opt-in and does not relax the existing admission boundary.

The isolated fresh-core qualification reuses audited cached third-party
libraries. It is not a full Cargo/workspace/dependency rebuild. Full saved
creature behavior, GPU numerical/transactional execution, retention of motor
competence, runtime cost, and causal gameplay are **Unknown**. Current readout
coordinates do not separate generic motor competence from direct personal
stimulus/action associations within an ActionCandidate head.

This combined draft also includes the fixed-receptor sleep guard. Sleep skips
promotion and decay only when the immutable receptor row has zero learning,
normalization, and replay rates, matching the compiler disabled-row contract.
The receipt baseline leaves those fast coefficients unchanged, and malformed
metadata rejects the transaction. Enabled-row arithmetic is unchanged.

A separate N2048 CPU/compiler/source composition test qualifies 384 selected
zero-rate Contact decoder rows and unchanged unselected receptor plans. It
validates WGSL with Naga and binds the test-only classifier to production
source; it does not execute WGSL. Combined GPU/transactional/performance
qualification and actual creature skill/association retention are **Unknown**.

## Requirement trace

- AOA-GEN-001 and AOA-INV-007: the inherited block contains only construction
  rate parameters; it does not inherit lifetime weights or personal memories
- AOA-LEARN-009 and AOA-BRAIN-009: STRUCTURAL compiler control for bounded local
  learning classes; useful adaptation/retention remains behaviorally unqualified
- AOA-LEARN-005 and AOA-BRAIN-007: preserve distinct update/consolidation and
  genetic/acquired weight semantics rather than using alpha as a freeze switch
- AOA-DEV-006: defaults retain mature learning; explicit zero is not reopened by
  developmental boost
- AOA-ADM-006: compiler/unit evidence is not a certificate of cognitive capability
