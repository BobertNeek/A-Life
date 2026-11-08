# Derived identity audit before V5 allocation

V4's preserved failure came before any primary action. Its comparator omitted CreatureSaveState.weights.genetic_fixed_digest, which initial_creature_save derives from full genome bytes. V5 adds explicit validation and projection of that identity, not a change to treatment/body/world.

## Fields projected and their source checks

- Subject foundation weights: every bit is compared against the actual trainer weights; genetic_pair requires exactly16 declared lane53 changes between arms and identical remaining bits/graph.
- Foundation asset digest and manifest.weight_asset.digest: FoundationWeightAsset.validate_against(actual phenotype) validates canonical asset consistency. The record asset digest must equal the actual trainer checkpoint's admitted source digest. Native own-restore also validates exact record/founder asset identity.
- Genetics-development state-graph digest: explicitly recomputed from typed actual genome and phenotype using the production canonical digest domain and byte order, then compared before projection. Record restore validation separately checks it.
- Private brain state-graph digest: the unmodified native own-genotype restore validates the complete resident_authority/private-state digest and sampling state; its complete checkpoint must round-trip byte-identically. Full private resident bytes, after removing only the validated phenotype binding, are compared between arms.
- Creature summary genetic_fixed_digest: recomputed with PortableAssetDigest::for_bytes(serde_json genome bytes), the same production FNV1a64 algorithm. Subject organism/genome IDs and single-founder scope are validated before projection. A fixed fixture forges this field and requires rejection before it can be removed.
- Resident and top-level checkpoint phenotype hashes: checked against actual compiled phenotypes by unchanged native own-restore and explicit pair checks. No cross-genotype restore occurs.
- Top-level founder weight digest: unchanged native own-restore checks it against the actual founder weights; between-arm full weights receive exact16 verification.
- Optimizer phenotype/source digest/weights: originate from the actual trainer checkpoint. Phenotype/weights are checked directly; source digest is checked against the validated record asset. Moments, ages, step, mask, configuration and schema remain byte-compared.

## Other identity-bearing summaries retained

The full portable save remains compared outside those explicit fields, including creature IDs, mind, appearance, learning summary, layer mutability/counts and all record metadata. Host checkpoint generation clears optional composite-genetics/GPU/lifetime references and uses an empty external asset manifest; these are not extra projections. Generated-weight references, genome IDs and all unrelated hashes remain compared. No generic removal of keys called hash/digest is used.

## Real comparator fixture

Use fixed constructor identity2026110901, not proposed26xx. Construct one OFF and one gain1/16 fresh host with zero preparation and no actions/ticks. Run the exact production pair comparator with expected tick0, including both native self-restores. Then change only the body's physical position in a detached fixture checkpoint, restore it under its own genotype, and require the real comparator's complete-world gate to reject. Restore the pristine snapshot, test a forged summary digest, and finish with another pristine full pass. No modified fixture state is used as a live experiment or published.

Separate existing tests reject changed nonidentity eligibility/sequence fields and an extra gene coordinate. The real fixture result, not this design description, determines whether qualification passed. Any unexpected difference is investigated before reservation; no study allocation is consumed by this zero-action construction fixture.
