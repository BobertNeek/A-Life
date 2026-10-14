# Newborn admission fixture correction — 2026-10-04

Source: main `482948ec116bad505cee0225cad74c1ab110acb7`, plus this focused
test-only change. The same CPU fixture is also checked with contact correction
`c04f2e8b020513eba4f95fd7f42ba6e99d29aa70` in an isolated validation worktree.

## Cause and correction

The old GPU fixture creates physical agents without registering canonical
organism records, then looks for candidate family names containing `Repro` or
`Mate`. Neither family exists. Reproduction occurs in the passive world step,
not through that invented candidate. Its retry test also assumes a child
survives a failed staged tick, contrary to world/resident rollback.

Register two compatible N512 founder genomes and their habitat memberships.
Only the fixture's fertility locus is set to one for a deterministic conception
trial; production genetics are unchanged. Mature parents through ordinary body
and inherited chemical updates with rest input, then wake them one interval
before the first reproduction boundary after puberty. Without rest, neural ATP
falls below the actual readiness threshold. No cached readiness, drive, hormone,
or maturity fields are patched. Birth then uses actual proximity, canonical
readiness, inherited reproduction and parental reserve transfer.

The GPU fixture requests bounded recovery sleep so parent movement cannot
change proximity before the passive boundary. A no-progress sleep driver keeps
that recovery phase pending; it does not supply neural decisions or perform
consolidation. The normal staged runtime tick still advances biology and
archives/adopts the child.

The failure test injects admission failure after the durable birth archive is
written. It requires exact world and resident rollback, including parent
reserves, clock, all four ID/sequence allocators, registry and habitat state,
memory/topology sidecars, published manifest links and handles. GPU live
membership must remain two. Retry executes the entire tick, recreates exactly
one child, reuses the retained manifest and publishes all child state together.
The success test retains archive/body/handle/sidecar assertions and additionally
checks synchronized homeostasis, empty newborn personal memory and exact live
membership.

## Validation and limits

An isolated CPU reproduction of the old candidate lookup failed at its original
`deterministic conception candidate` expectation. The corrected two CPU tests
prove natural passive birth and production world/resident staging rollback and
whole-tick retry. The rollback test uses the same rest events as the GPU fixture.
Both pass on main and on the latest contact correction overlay.

Optional `gpu-tests` all-targets Cargo check and strict Clippy pass without
executing tests. Formatting, static core boundaries and documentation assertions
pass. Physical-GPU admission/archive retry execution remains **Unknown**: no GPU
workload, training, or graphical workload was run. Ordinary CI does not enable
`gpu-runtime` or `gpu-tests`, so it cannot prove either optional test path; the
CPU fixture evidence comes from the explicitly enabled local test command.

No production behavior, genetics, GPU policy, save/load codec, or runtime cost
changes. Fixture maturation uses bounded test-only maps and advances 1,199
intervals for each two-parent setup; it adds test work rather than a production
scan. The two CPU tests take about 0.65 seconds together in this unoptimized
cloud validation environment. That elapsed test time is not a gameplay or GPU
performance benchmark.

## Architecture trace and overlap

- AOA-REP-001 / AOA-REP-006: actual canonical readiness, proximity and parental
  investment produce the child.
- AOA-GEN-007 / AOA-REP-002: fresh child identity and empty personal memory.
- AOA-AUTH-003 / AOA-BIO-015 / AOA-BIO-026: one advancing body/chemistry authority;
  compare actual parent physiology and synchronized newborn state.
- AOA-SLEEP-003: failure compares staged resident and sidecar authority instead
  of accepting partially advanced recovery state.

Changes are confined to the app test module, its test-only module declaration,
and this dated report. PR #4 also changes `gpu_live_runtime.rs` at a separate
module declaration; the contact overlay applies cleanly and passes the two CPU
checks. There is no overlap with GeneForge PR #3, the legacy test prerequisites,
or the island/art files. The controlling architecture is unchanged.
