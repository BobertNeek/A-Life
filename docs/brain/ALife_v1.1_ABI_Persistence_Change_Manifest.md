# A-Life v1.1 ABI and persistence change manifest

Status: source repair complete; independent architecture review pending. No behavioral or performance validation was run for this manifest.

Authority: `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`  
Repair base: `19b4c0af272f41cb640e9cfd0ee93582408c6dc6`  
Repair branch: `codex/v11-repair-only-intelligent-animal`

## Changed contracts

| Contract | Old | New | Reason and compatibility behavior |
| --- | ---: | ---: | --- |
| Cognitive context schema | 1 | 2 | Adds bounded target-specific topology and predecision prediction context. Old context is not accepted as the repaired live ABI. |
| GPU closed-loop layout | 4 | 5 | Carries repaired context, factorized motor, outcome, dendritic, structural, and work state through the production dispatch. Host and WGSL constants advance together. |
| GPU topology-context ABI | absent | 1 | Adds bounded concept/gap candidate context. It supplies neural context, never a host desirability score or direct action choice. |
| Semantic state schema / ABI | 1 / 1 | 2 / 2 | Replaces same-length/different-meaning vectors with one stable grounded state schema. |
| Prediction target schema | 2 | 3 | Separates grounded successor state from explicit grounded outcome and binds the categorical joint motor condition. |
| Predictor state schema / ABI | implicit legacy | 2 / 1 | Persists interaction-capable predictor/value state and its semantic bindings. |
| Successor feature ABI | 1 | 2 | Uses the stable grounded successor meanings rather than legacy mixed semantics. |
| Joint motor condition schema / ABI | 1 / 1 | 2 / 2 | Binds six parallel channel commands and categorical primitive embeddings into one causal action identity. |
| Grounded outcome schema / ABI | absent | 1 / 1 | Names measured body, homeostatic, reward, value, RPE, pain, injury, novelty, residual, and social outcome fields. |
| Motor category schema / ABI | absent | 1 / 1 | Represents primitive identity categorically; numeric or Hamming proximity has no semantic meaning. |
| Species-specific motor channel | absent | v1 | Adds one versioned bounded species channel without making the current species layout permanent architecture. |
| Cognitive-work cost policy | hard-coded conversion | schema 1 | Hardware-independent work is mandatory; optional energy, fatigue, and heat conversion is a versioned world/species policy. |
| GPU brain save state | 3 | 4 | Requires the repaired exact cognitive asset and passive life statistics for exact resume. |
| Exact cognitive checkpoint | 1 | 2 | Binds organism/world/foundation/phenotype/sensor/runtime/policy/tick identities and persists repaired acquired cognition. |
| Durable founder cognition | 1 | 2 | Explicitly projects inheritable founder material while excluding transient attention, motor intent, pending transactions, sleep state, work receipts, and world-local identity. |

## Exact-resume ownership

| State | Durable owner | Restore rule |
| --- | --- | --- |
| Position, body, homeostasis, chemistry, age, lifecycle, genome, archive identity | canonical world organism record | Replaced atomically as world truth; cognition only consumes it. |
| Foundation, phenotype, brain class, sensor profile, runtime profile, activity policy | checkpoint identity binding | Every identity must match before mutation. |
| Attention and focal continuity | exact cognitive checkpoint | Tick-bound; missing state rejects exact resume. |
| Concepts, unresolved gaps, and candidate context | exact cognitive/topology assets | Stable IDs and active context are restored; no diagnostic-only reconstruction. |
| Predictor, value state, and semantic bindings | exact cognitive checkpoint | Schema and semantic ABI must match. |
| Six-channel motor selection, eligibility, outcome, fast and lifetime learning | GPU checkpoint assets | Restored before the next authoritative decision. |
| Sparse accepted synapses, structural reservoir, pending growth, dendritic branches | exact v1.1 sparse state and topology assets | Bounds and cross-identity are checked before GPU publication. |
| Replay order, memory banks, sleep stage, staged/committed transaction | replay, sleep, and pending-transaction assets | Sequence and phase identity must match; partial continuity is rejected. |
| Cognitive work and optional biological cost policy | exact work receipt plus phenotype/world policy identity | Work remains hardware-independent; conversion remains configurable. |
| Passive life statistics | required life-statistics asset | Exact resume fails if absent. |

## Migration and rejection

- Exact resume does not silently default missing acquired state.
- Old version-1 exact and durable-founder payloads are rejected because their omitted state cannot be reconstructed exactly.
- Rejection occurs before partial live mutation and returns a typed schema or `ExactResumeUnavailable` error.
- Durable founder construction first validates an exact source checkpoint, then performs the explicit founder projection.
- Current Rust structs, N512/N2048 layouts, and GPU placement remain implementation choices. The persisted semantic identities and locked invariants are the contract.

## Source commits

- `16fbf112`, `514f95c6`: predictor/state/outcome semantics.
- `8dfc94cc`, `70954f9f`: factorized motor and outcome-learning ABI.
- `66bf689c`, `5677fd8e`: structural/dendritic and atomic sleep state.
- `6347fc80`: evolvable architecture and cognitive economics policy.
- `e29ad762`: exact checkpoint/save/founder schema integration.

