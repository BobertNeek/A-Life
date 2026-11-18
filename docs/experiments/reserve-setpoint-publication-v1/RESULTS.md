# Accepted-main protected publication qualification

**48/48 normal compatibility tests passed.** The reserve candidate remains
unused by default. Both finite physiology experiments remain ignored and closed;
the initialization audit binary was not invoked, and the candidate grid was
explicitly reported ignored. No world, neural forward, device or cohort run
occurred during publication qualification.

## Source and scope

Accepted local base: `f83be325d47b435c9e8889d70ea64496f2b382b4`, tree
`da127f8d53fd615f3df18e01a5cbff8c2232738c`, supplied as the exact tree equivalent
of remote main `c775c8c92e8f5e625f8a3e861ef79200199b54df`.
Executed implementation commit: `b05f34620c31b733d126b754a6517820ea56241c`, tree
`e29c7dbc5400a4a2e2d97aabd015f03bca4f5d7d`. Later additions are only qualification
records and package-local captured-log attributes; all tested implementation and
test-source bytes remain unchanged.

Existing implementation changes are exactly 33 added lines in two files:

- `crates/alife_core/src/biochemical_graph.rs`: 15 lines for the validated private
  inherited-threshold setter.
- `crates/alife_core/src/evolutionary_genetics.rs`: 18 lines for its explicit
  allele/index public gene operation.

No existing implementation is deleted or replaced. No world/thermal/neural code,
defaults, schema, ABI or asset changes occur. The accepted-main preimage blobs of
both files exactly match the original 550f79ad source, as recorded in the frozen
plan. The method is called only through explicit construction/tests, not by any
default founder recipe.

Imported artifacts are the three core test files plus baseline, candidate and
composition plans/results/evidence. **All 38 imported files**, including the two
implementation files after the 33-line patch, match sealed addendum-final
`a2081cc6a9a2bebbefed46153787186dc807db84` byte-for-byte. Per-file SHA-256 and size
records are in [SOURCE_BINDING.json](SOURCE_BINDING.json).

The historical identities remain unchanged:

- Baseline audit final `9b41dd90bb44a43ab95cdd788c6e58fd2d194ee8`;
  execution `bc0bd4a0cebfd8301fee0b9827fb8add729b14c1`.
- Candidate final `98fe445493192bc80bb9cbf0b2e16fc16c7df2bf`;
  execution `f3602c25929b099c1a04c10aa2fb7ea4a87eef79`.
- Composition addendum final `a2081cc6a9a2bebbefed46153787186dc807db84`;
  execution `ba409c0336c7d20a78757bdaafcf7fdacb21f9ef`.

## Normal tests and strict checks

| Binary | Passed | Ignored |
|---|---:|---:|
|abi_validation_errors|4|0|
|biochemical_graph_v2|11|0|
|biochemical_valuation|4|0|
|chemistry_homeostasis|9|0|
|n512_foundation_abi|5|0|
|reserve_pressure_composition|4|0|
|reserve_pressure_setpoint|1|1 closed grid|
|sensory_abi_contexts|6|0|
|signed_decoder_foundations|4|0|
|Total|48|1|

The new validation/composition tests perform zero physiology advances. The legacy
compatibility suites exercise their existing small core-chemistry updates,
compiler ABI, inherited sign and serialization guards. They are not new outcomes
for either sealed grid. Checked-in heritable foundation assets remain unchanged;
no acquired creature checkpoint was inspected.

- Whole-workspace formatting: passed.
- Strict `cargo clippy -p alife_core --lib --tests --offline -- -D warnings`:
  passed, including compilation of ignored test code without execution.
- Normal dependency/static core boundary script: passed.
- Original-file byte verification, exact source inventory and preimage checks:
  passed.
- Ordinary full diff whitespace check: passed with the repository's captured-log
  convention in two narrowly scoped package `.gitattributes` files. They disable
  whitespace diagnostics only for captured `*.log` in the imported evidence
  folders. Source, tests, prose and other paths retain ordinary checking.

The initial unqualified diff check's 12 historical log diagnostics are retained
in [initial-diff-diagnostics.json](initial-diff-diagnostics.json), and the exact
resolution is described in [PREFLIGHT_NOTE.md](PREFLIGHT_NOTE.md). No raw-log byte
was stripped or changed to achieve the final check.

## Publication disposition

Ready for the parent's protected draft publication, with the candidate still
explicitly opt-in. This is compatibility qualification on accepted main, not
promotion of a new default breed or proof of autonomous behavior. The prior
reserve/sleep availability limits and composition restrictions continue to apply;
ATP is an availability variable with relaxation, not literal conserved inventory.
There was no retuning, rerun of either finite grid, world/device execution or
expansion into a lived cohort. No public PR/push was performed by this worker.
