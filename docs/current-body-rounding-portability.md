# Explicit saved body rounding and bounded CPU policy continuation

The ordinary Current reader remains strict. `WorldOrganismRecord` normally
requires its entire saved phenotype to equal `genome.express()`. Some saved
cross-platform bodies differ in `metabolic_turnover`, whose expression uses
the platform's `f32::exp2`. A generic `InvalidId` can therefore report a numerical
phenotype mismatch without a literal zero identity.

The optional `SavedBodyRoundingCompatibilityV1` is an explicit admission proof,
not a replacement phenotype. It identifies one organism and genome, hashes the
entire saved and locally expressed genetics/phenotype, and records the exp2
input and both output bit patterns. Proposal requires positive finite adjacent
f32 outputs, a non-integral exp2 input, and exact serialized equality of every
other phenotype field. Native identity, subsystem digests, compiler topology,
heads, same-individual counters, private assets and sampler checks still run.
No saved scalar, digest, neural state or raw source byte is rewritten.

The proof runs in a lexical thread-local source-admission scope. It cannot nest
or survive unwinding. It does not change ordinary body construction or the
production assessment comparator. A proof is accepted only if native validation
actually uses it. The request binds it to the explicitly selected raw Current
SHA256. It is not a claim of Windows/GPU numerical parity or a reconstruction of
the saved platform's math implementation.

```text
export_native_training_handoff --propose-current-body-rounding CURRENT RAW_SHA256
export_native_training_handoff --inspect-current-body-rounding CURRENT PROPOSAL_JSON
```

The proposal is not native admission. The second command performs actual native
admission and reports the explicit compatibility provenance. For assessment,
include the proposal's `proof` as `saved_body_rounding_compatibility` in the
ordinary Current request. Keep the original source and the exact executable.
Run full/bare actual-parent parity before scoring. Both policies still require
identical executable and initial controls for a matched comparison. The saved
private life is retained in the source and is not transferred into a new life.

The same executable also qualifies an untouched native CPU parent without
converting its source format. Use the existing `CpuFounderEvaluationRequest`
and actual native source asset/anchor/history for both requests; only the absent
output directories differ:

```text
evaluate_cpu_current --founder-spec BARE_NATIVE_CPU_REQUEST.json
evaluate_cpu_current --full-founder-spec FULL_NATIVE_CPU_REQUEST.json
evaluate_cpu_current --founder-parity BARE_OUTPUT FULL_OUTPUT
```

The full control requires the actual source policy, carries its actor/value and
complete Adam read-only, and validates rebinding before inference. Its separate
schema3 receipt binds full-head preservation. Strict parity verifies identical
initial controls/executable, actual first inputs, action traces and entire new
private checkpoints. Different parents, candidates, cases or binary identities
are rejected. Original native CPU source files are read-only and no new CPU
training receipt is created. Run both policies' bare cases with this executable
and require `--compare LEFT RIGHT` to return `matched:true` before interpreting
paired differences.

