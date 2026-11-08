# Source publication and reproduction

Branch: `experiment/grounded-drive-basis-v1-source` in `BobertNeek/A-Life`.
This is an additive source/text/finite-scalar evidence archive, not a PR, default promotion or gameplay qualification. No new native run or study was performed to publish it.

## Exact identities

- Original bounded packet: `5997ce7c890b6495e787142fe553e63ed595f46f`.
- Public additive parent: `8ddf89f64311a5d6c8e76e5b219d6665c9ddac4e`, tree `dc37336a9a23538f05efa340545d0c314a6b087b`.
- Priced direct execution: `1d39f179790234c5de2ec011811af352f7060a05`, tree `a3a8890a28457f32ff70fcac51d912227bb85635`.
- Later basis documentation only: `973aad37c5056637bef43d64b41f452c910635ec`.
- Software-kernel executed source: `67e26f53009c9b925b94cac1af5421894b059481`, tree `d0281b30428a0f4ed1b48e568ab1da3cfdbd5e55`.
- Software fixture report-only final: `7bb38aadce688ef0ad68d1599bb7bae2ce6c7e07`, tree `dbe8852900963a6b44184912c8f1e363e86d0d82`.

The publication applies only the required `5997ce7c → 1d39f179` source delta to the public parent, then the frozen software fixture. Every runtime source file matches the frozen priced source, except the declared 12-line `gpu-tests`-gated software research constructor from the executed software fixture. Ordinary production/hardware constructors are unchanged. The 573-line integration test is exact executed source. No ongoing host/genetic integration edits are included.

The only inherited source difference against the executed software snapshot is `crates/alife_training/src/cpu/host/bounded_founder_tests.rs`, retained exactly from public parent `8ddf89f`. It is test-only and contains the separately measured nonzero-history supplement. Original bounded source functions and evidence, and all five supplement files, remain intact. The original bounded publication note predates that supplement; the supplement's own report, rather than the original note, is its authority. Neither separate historical qualification proves the new class's whole-organism continuation.

The root design document retains historical stage-by-stage language and the `973aad37` measured addendum. Its prospective prototype and earlier “unverified” paragraphs are not present-tense evidence. The example prototype files are retained exactly as frozen research source, not promoted runtime guidance. [RESULTS.md](RESULTS.md) states the current finite claims.

## Hash and allowlist closure

- `SOURCE-CLOSURE.tsv` records every tracked Rust, WGSL, Cargo manifest and Cargo lock file, with Git blob, SHA-256, byte size and its match to the executed software source. Only the bounded test difference is allowed.
- `FROZEN-DELTA.json` records every added/modified source path and exact frozen origin. No source conflict was silently resolved.
- `PRESERVED-BOUNDED.json` records unchanged bounded evidence and the five-file nonzero-history supplement.
- `FILES.sha256` is the complete add/change publication allowlist and hash manifest, excluding only itself to avoid self-reference. Its own identity is the enclosing Git tree's blob. There are no deletion entries.
- `EVIDENCE-PROVENANCE.json` retains SHA-256 and byte count of every original evidence file alongside the corresponding published file. Logs are full, path-sanitized derived text where indicated. They are not mislabeled as original bytes. No numerical result or failure is removed.

Publication differs from both executed trees through additive parent documents, the inherited test-only supplement and publication documentation. **Do not describe the publication tree as executed or merge-ready.** Source/hash equality is not a new native test.

## Executable and adapter identity

Software test executable: `grounded_drive_gpu_parity-8c5623dd9357d217`.
SHA-256 `24a644cc8988b2ca3c0142cabe3e188a56e9726426f8c618c04e8d1863fee32f`.
SwiftShader library SHA-256 `0370ba973a74faa50a2d191fcc14e697b51e8485ec778bafb6539064e66b9a3a`.
ICD SHA-256 `c9eef28b6b984fec220ef0abfecc40b502d46946706e47bfe97707027cb818bd`.
The full adapter descriptor, per-frame receptor/step receipts and finite phenotype/asset digests are in `gpu/run-01.log`; these are not learned arrays. Native fixture runtime was 24.51 seconds including setup, not a performance benchmark.

Core/CPU executable filenames are recorded in their logs, but **contemporaneous executable hashes were not captured**. Current build products were not retroactively labeled as historical executables.

Rust 1.99.1, commit `58cf8f9c35b355199727627d07ae34a4e2846d67`, LLVM 23.1.1, `x86_64-unknown-linux-gnu`. Use one build job, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` and a separate target directory. The original environment used explicit CARGO_HOME/RUSTUP_HOME. Reproduction requires the dependency cache for `--offline` and a separately installed compatible Vulkan adapter. No executable, dependency bundle, learned bank, checkpoint or raw state array is distributed here.

## Exact priced commands

Run from the priced source checkout, not as evidence that this additive tree has already run:

```sh
cargo check --offline -p alife_core -p alife_gpu_backend -p alife_training --features alife_training/cpu-backend --lib --tests -j1 &&
cargo test --offline -p alife_core --test grounded_drive_pricing --test n2048_grounded_drive_basis --test n2048_bounded_founder --test n2048_foundation_abi --test n512_foundation_abi --test signed_decoder_foundations -j1 &&
cargo test --offline -p alife_training --features cpu-backend --test grounded_drive_transfer --test cpu_grounded_drive_basis -j1 &&
cargo clippy --offline -p alife_training --features cpu-backend --test grounded_drive_transfer --test cpu_grounded_drive_basis --no-deps -j1 -- -D warnings
```

The original command chain used `set -o pipefail`, with combined stdout/stderr saved to the priced log. The earlier unpriced command tested `cpu_grounded_drive_basis`, then ran strict Clippy for that same test, with the same environment. Early default-feature compilation is not substituted for explicit CPU-feature evidence.

## Exact software-fixture commands

Run from executed source `67e26f53` with the toolchain/environment above. The preserved path-sanitized runner is `gpu/run-fixture.sh.txt`; `<workspace>` and `<artifacts>` are placeholders, not literal paths.

```sh
cargo test --offline -p alife_training --features cpu-backend,gpu-tests --test grounded_drive_gpu_parity --no-run
VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json VK_ICD_FILENAMES=/usr/lib/chromium/vk_swiftshader_icd.json cargo test --offline -p alife_training --features cpu-backend,gpu-tests --test grounded_drive_gpu_parity -- --ignored --exact priced_grounded_drive_software_kernels_match_cpu --test-threads=1 --nocapture
cargo clippy --offline -p alife_training --features cpu-backend,gpu-tests --test grounded_drive_gpu_parity -- -D warnings
```

This archive provides exact source blobs and historical local snapshot identities. Those local execution commits are not promised as remote refs. The published source has exact frozen content with the explicitly listed inherited test difference; obtain the historical snapshots separately if a byte-for-byte full-tree reconstruction is required.

Original method: https://rikiyat.github.io/blog/weight-space-programming/ . See the measured-versus-proposed limits in RESULTS.md before using these results.
