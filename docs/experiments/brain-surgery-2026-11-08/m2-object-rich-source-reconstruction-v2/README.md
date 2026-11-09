# Complete frozen source reconstruction

Apply **fbf2a517-to-object-rich-frozen-v2.patch** once to a fresh, clean checkout of exactly:

- Commit: `fbf2a517f93098b5d936878a3d26bbb8d0391c38`
- Tree: `020d7a4b036d0309a4ffdd762536c36e945f7496`

This is a complete, flattened source patch. It does not require the earlier optional-Vocal patch, any observer patch, the local source tar archive, or the 7f905465 driver commit to be available. Do not layer `PASSIVE-WORLD-RECEIPTS.patch` or any other historical patch onto it. The superseded public-field observer approach is not used. Only the final passive research wrapper is included.

## Contents and exact result

The patch changes 18 paths: 13 additions and 5 modifications. It includes the qualified optional-Vocal implementation and tests, final separate passive observer return/copy seam, object-rich scene/driver/qualification helpers, final attempt-record transport correction, frozen protocol and runtime-source manifest. It also includes the old reference driver because that file is part of the frozen source inventory.

After application, all 101 files in `SOURCE-FREEZE-EXPECTED.json` have the exact frozen SHA256 and length, totaling 2,283,261 source bytes. The other repository files remain at the exact base commit. The reconstructed full repository tree is:

`846aec07d106138cc092ca560dc675b73f60e5e5`

Frozen 101-file source closure:

`0a444a493f97fdcea1bcd438fa50cdb7340e7067fe9ba418c8efcfd7ea41e0d1`

Patch: **374,496 bytes**

Patch SHA256:

`bdccb29beb346d916ec0f71fcecbebe4fe46466454aaa3371aa2de7d41e7c824`

## Application and verification

Use a new source checkout, never the prior frozen runner or any acquired-life directory. Make the four delivery files available together: the patch, this README, `SOURCE-FREEZE-EXPECTED.json`, and `verify_frozen_source.py`.

From the fresh repository root, after selecting the exact base and confirming a clean index/worktree:

```sh
git rev-parse HEAD
git rev-parse HEAD^{tree}
git apply --check /absolute/path/fbf2a517-to-object-rich-frozen-v2.patch
git apply --index /absolute/path/fbf2a517-to-object-rich-frozen-v2.patch
python3 /absolute/path/verify_frozen_source.py --source-root .
git write-tree
```

The first two identities must equal the base above. The source verifier must report 101 verified files and the stated closure. With no unrelated changes, the final tree must equal `846aec07d106138cc092ca560dc675b73f60e5e5`. Applying the patch does not commit it and does not execute an experiment.

The base commit must be present in the recipient's authorized repository. This artifact does not fetch it, supply repository credentials, or assert that an unauthenticated public endpoint serves it.

## Independent application proof

`PROOF.json` records an application in an independent bare Git repository, index and object store. Base objects were borrowed read-only from the exact existing commit; no old repository object/index/worktree was changed. `git apply --cached --check` and `git apply --cached` both passed. All 101 reconstructed blobs were independently read from the resulting index and matched to the frozen file hashes and lengths. The resulting tree changes exactly the 18 listed paths and nothing else. No full checkout was created.

`isolated.git` and `isolated.index` retain local proof evidence. They are not standalone source deliverables: their unchanged base objects were supplied through a read-only alternate. Remote recovery requires only the exact base plus the complete patch. Neither the proof object store nor the historical tar archive is an application dependency.

`verify_frozen_source.py --git-index` can also verify a reconstructed index selected using the caller's standard Git environment. It is read-only and invokes no A-Life code.

## Source prerequisites outside the library

There are **no omitted non-library source prerequisites from another commit**:

- `probe_grouped_m2_durable_v1.rs` originated in driver commit `7f905465b79291fd52d1fd65568a9d10e716576a`, not the base. Its complete frozen bytes are included as an addition. It is reference source, not the object-rich study entry point and not permission to continue the historical experiment.
- The copied raw/journal helpers also originated in that driver line. Their complete final object-rich revisions are included, so that commit and old helper patches are not needed.
- The object-rich driver, scene, policy wrapper and qualification source are included in full. `PROTOCOL.md` and `RUNTIME-SOURCE.json` are included in full.
- The recovery-sleep recipe, grounded fatigue/motor helpers, multidrive helper and three recipe JSON documents already exist in the exact base. Their unchanged hashes were verified as part of the 101-file proof.
- The optional-Vocal source and final observer edits are fully incorporated, not references to local files or prior patches. Core/world/GPU/runtime dependency sources remain the exact base versions.

## Build and experiment limitations

This is source reconstruction, not a new build, qualification, host admission or executable reproduction. The original build used Rust 1.99.1, CPU/default/receipt-lineage configuration, direct dev0 root compilation and verified prebuilt dependencies. Toolchain/dependency binaries are not supplied by this source patch. A future builder must separately establish its permitted build inputs and executable binding. Existing base Cargo/workspace files remain unchanged.

The source still contains the original study's fixed seed, reservation paths and historical frozen protocol. Do **not** run its preparation/primary commands to reconstruct source or start another comparison. A fresh comparison needs a new owner/seed/driver/protocol binding while treating the reconstructed library as its exact baseline. The patch grants no permission to replay a consumed interval or import an acquired checkpoint.

No checkpoints, journals, source input payloads from a life, acquired weights/optimizer/memory, executables, credentials or private study state are included. The frozen source closure, existing executable, source archive and original patches were verified unchanged. No runtime changes, builds, fixtures, world/neural calls, private-state reads or publication occurred during this reconstruction task.
