# Hearthling appearance continuity and shared resources, 2026-10-18

Base: `961cddad6b6a6f850eb47f936b7c9cf01383ab39`.
Integrated naming PR92 at `57e13160ed4918a0f4435d36e2192031d6437275`.
Scope: appearance continuity and bounded material reuse. This is a prerequisite
for Cassidy's selected 9/16/19 individual variation direction, not a completed
selected-model refinement or a native performance qualification.

## Source findings and change

The production path already has a shared skinned Hearthling, six coat palette
buckets, shared fur normal and animation graph, inherited body scaling, and
animation-safe ear/head/tail bone multipliers. The retired generated-part coat
atlas has incompatible semantic UV regions and is not reconnected.

Live newborn projection formerly generated appearance from IDs, while the first
checkpoint inherited cosmetic genes from parents and conception seed. The two
paths could display different ear/tail/coat details before and after reload.
A small app-only cache now preserves exact saved appearances and derives new
entries once through the existing inheritance/body projection. Checkpoints keep
raw saved cosmetics intact. Both initial/restored and live presentation apply
the same existing read-only body-phenotype projection for palette and mass; the
canonical body retains ownership of those two traits. All other saved cosmetic
genes pass through unchanged. Entries bind organism and genome IDs;
live projection also checks the frame's world-entity binding. Retired parents
remain available through newborn inheritance, then the cache is pruned to the
resident set when its count exceeds the residents already admitted to it.
Steady-state reconciliation does not scan the cosmetic cache. Curated cohort
replacement stages its appearance cache before GPU
submission and publishes it only with committed resident maps.

Cosmetic parents are still identified by the genome IDs already carried by the
child. If cloned individuals share a genome but have different cosmetics, lookup
uses the lowest organism ID deterministically. Adding exact clone-parent
provenance is outside this change.

The historical `brows and tuft shadows` material is revision 9's polished nose;
it no longer receives coat tint. Neutral coat palettes reuse the source handles.
Only the two existing coat material sources can create palette variants.

## Structural resource evidence

- Shipping asset check passes: one GLB mesh, six material primitives, 32,694
  triangles, one skin, 34 joints, three authored clips.
- GLB SHA256 is `60e972f6ba691f347703306d089d801ec47aa97cc2b7b41b962a56bf0a2ea6d7`;
  authoring blend SHA256 is `602186f3c03a6a3f52e9fda369f4d1b74296787e2358a4f2e1c50295b93cae0e`.
  Both files are unchanged.
- The coat cache ceiling falls from three sources times six cloned palettes
  (18) to two sources times five non-neutral palettes (10). This is the tint
  cache only, not a count of all renderer/highlight materials.
- No mesh, skin, texture, shader or animation asset is added. Material selection
  occurs at scene readiness; existing residents do not recompute appearance or
  allocate its resources each render frame.
- Six primitives describe the submitted asset structure. Actual draw calls,
  batching efficiency, GPU memory and RTX frame timing have not been measured.

## Architecture and preservation

Relevant controlling requirements are `AOA-AUTH-001` (identity binding),
`AOA-AUTH-002` (read-only cached views), `AOA-GEN-001` (genetic construction state),
`AOA-GEN-007` (new child identities), `AOA-INV-007` (personal state is not inherited)
and `AOA-BRAIN-007` (separate acquired synaptic state). The cache is an app-only
projection keyed to immutable genome identity and the saved appearance schema.
It does not edit canonical genomes, learned weights, memories, names, or save
schemas. This report does not certify unrelated architecture requirements or
GPU learned-state persistence.

## Validation and limits

All 40 targeted CPU tests passed after integrating PR92: 17 Hearthling renderer,
10 appearance, and 13 naming tests. Appearance coverage includes eight cache
tests, the real natural-birth checkpoint/production-save reopen regression, and
the existing saved-gene population test. Strict all-target frontend Clippy also
passed. Two existing test-only lint warnings were fixed without behavioral
changes. Formatting, whitespace, static core boundaries, all
77 documentation assertions, and the shipping GLB check passed. A dedicated
Hearthling workflow runs focused renderer and appearance tests; pinned art
source receipts remain unchanged. The PR records frontend and hosted CI results
at their exact revisions.
The learning-summary sentinel check is CPU data-preservation evidence; it does
not exercise a real GPU learned-weight checkpoint.

Independent R2 review found and resolved curated-reset publication, stale
frame identity, and inconsistent live/restored body-projection gaps. Native committed/error/unknown reset execution remains
unmeasured. Revert the change as one unit if continuity regressions are found;
no save migration or asset rollback is required.

## Explicit blockers and coordination

All four known Library references were resolved, but this executor's supported
materialization and retry returned signed transfers with no local workspace
paths; every helper failed with `library file transfer failed: download failed`.
DNS resolution also failed for the returned host. Native image reads returned
image pointers and captions, with no image-content blocks available to inspect.
No selected reference pixels have been viewed here, and no image-dependent asset
edits or new palette/eye/marking/paw refinements are claimed. Exact transfer
records and identity metadata are retained privately in this workspace; a
supported attachment handoff is needed for the selected art pass.

PR84's existing training owner retains the native PC workload. Coordination was
posted on PR84 and PR92; no PC workload, training seed, checkpoint change or
interruption was started by this task. Native appearance/performance acceptance
still needs an owner-coordinated safe slot. Naming/composer/HUD work remains with
PR92's owner.
