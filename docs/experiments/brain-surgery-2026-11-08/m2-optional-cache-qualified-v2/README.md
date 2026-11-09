# Qualified optional cache: curated source-only handoff

This packet records a completed bounded qualification. It is not permission to run experiments, build, publish, migrate or replay any life. No native work was performed while preparing this handoff. An in-memory static reconstruction check matched every V1 instrumented and V2 source-manifest hash for both variants. Original packets remain unchanged. This is not a complete repository or a binary distribution.

## Result and limits

26/26 fixed V2 executions passed. Uncached/cached optional output was directly byte-equal across all seven frames, 9,505,949 framed bytes per output. The original five legacy witnesses matched 7,116,876 logical bytes (7,116,916 framed bytes). Digests and frame sizes are metadata only; raw witnesses are excluded. Results are curated from the original RESULTS-v2.json, whose SHA256 is recorded in RESULTS-CURATED.json.

V1 control compilation failed with E0277 and had zero source/world exposure. That failure remains part of the history, not a hidden retry. V2 fixed only the private receipt serializer, adding its field/null/signed-zero/framing test and module routing. Receipt serialization includes historical codec bytes, source/toolchain-pinned Debug and 11 body-event IEEE bit words; it is not a new public wire format.

Actual exposure was 43 source forwards, 35 world advances, 15 recipes, 3,600 body-only preparation ticks, and eight arithmetic/synthetic constructions. Learning stayed closed. Coordinated cache composition remains source-only and NOT natively qualified. There was NO timing comparison for this composition. Prior 95.8% standalone administrative timing is NOT its speed result. No end-to-end speed, learning, or welfare benefit is established.

## Reconstruct source only

Use fresh disposable repository copies of BobertNeek/A-Life commit fbf2a517f93098b5d936878a3d26bbb8d0391c38 (tree 020d7a4b036d0309a4ffdd762536c36e945f7496). Respect repository AGENTS.md. Do not modify original snapshots or existing build targets.

1. Apply production/full-from-fbf2a517.patch with strip level 1 to create the optional cached candidate. Its SHA256 is 9412792ee0b3ed9a1ac0f68bc83350a1721cd6db96df7415031731164a75ce67. This exact full patch retains historical source documentation and local path constants as provenance; they are not instructions to launch a driver or evidence that this candidate is unqualified.
2. For the uncached control, create a separate copy of that reconstructed candidate and reverse production/incremental.patch (strip level 1). The incremental patch applies forward only to the exact original uncached optional base. Check production-source.json against the resulting source files.
3. In each independent copy, apply qualification/{candidate,control}-instrumentation.patch as appropriate (strip level 1). These contain exact private test selection, counters and witness assertions. Copy qualification/qualification_sink.rs to crates/alife_training/src/qualification_sink.rs and append a newline followed by #[cfg(test)] and mod qualification_sink; to crates/alife_training/src/lib.rs. This sink/lib addition was separate from the historical instrumentation patches.
4. Copy the corresponding qualification/{candidate,control}-v2/ directory contents over that independent copy. These are the exact V2 new source files, including lib_v2.rs, its module-routing ancestors and the complete corrected receipt codec/test source. Do NOT also apply the private-delta patch when using these exact V2 files. The private deltas and codec-v2-extra.rs are included for review and alternative in-place reconstruction only.
5. Compare all files against the matching manifests/{candidate,control}-SOURCE.json and -v2-SOURCE.json. V2 manifest symlink flags describe the historical layout only; ordinary copied source files with matching bytes are equivalent. Manifests list the full repository-relative source closure; unchanged files come from the base repository. No trained data is needed to reconstruct source.

## Fixed test reproduction specification (not executed by this handoff)

The V2 manifests contain the exact registered inventory: candidate 25, control 1. RESULTS-CURATED.json records exact test order, names, counters and per-case resource samples. Compile each selected test root once with the specified Rust/dependency identities and settings; verify exact inventory before fixtures. Execute each listed test with --exact and --test-threads=1 in that order, stop on first failure. Do not expand fixtures, seeds or repeat runs to seek a pass. Reproduction needs separately approved disk/RSS/time budgets, fresh dependency outputs and a fresh private output directory.

Tests writing witnesses require DUAL_WITNESS_PATH set to a fresh nonexisting private file and DUAL_WITNESS_CAP set to an approved bounded byte cap (the completed outputs were 644, 7,116,916, or 9,505,949 bytes). Preserve framing: each complete payload is prefixed with its u64 little-endian length. Future full equality verification must compare complete bytes privately, not hashes alone. Do not upload generated witnesses or source-derived life/checkpoint data. The original successful comparison is reported here, not rerun.

Dependencies are identified, not bundled. Rebuilding them may not yield the exact historical artifact hashes. Build-settings.json is descriptive; no build or executable launch occurred in this packaging task. Resource samples were sampled, not enforced OS quotas.

## Safety and contents

Only source patches, exact test/codec source, manifests and aggregate summaries are included. No raw witnesses, trained checkpoints, banks, acquired lives, binaries, compiler/runtime logs or credentials were read or copied for this handoff. File names mentioning checkpoint/bank in source are source code, not those data. Nothing was uploaded, pushed, merged or published. SHA256SUMS.txt covers all content files except itself; the separately returned whitelist includes its own digest.
