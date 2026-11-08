# Fatigue opponent circuit: preserved negative result

This research publication preserves the exact source and measured synthetic results from runtime commit `744460a59886b0fe8d94fe287671c8648234453c` and frozen result commit `ebfd0053bc91733fe954cbb98ee67384b4ed7f4b`. It is based on public operating-point commit `1f6d105bb3dfc8c316ef907bf30da1c5b134c572` (tree `bde9ed08506eced22d31e1b814456253d2cca4e4`). The fourteen carried-over files are byte-identical to the frozen result commit; this publication note is the only addition beyond them. The later fresh-branch-state proposal is excluded.

Read [RESULTS.md](RESULTS.md) for the findings and [README.md](README.md) for the methods. All 24 arms completed: 624 evaluations, 59,904 frames, 239,616 microsteps, two passing unit tests, 600 exact input-cut assertions, 300 full readout-pair trace assertions and 270 frozen-baseline snapshot checks. Source-cut and relay-cut controls recover the original outputs exactly. Fatigue produces a causal Rest response, but Approach sign inversions in other contexts prevent promotion as a general motor reconciler.

These are detached native CPU inference measurements using freshly generated synthetic state. World steps and learning steps are zero. No retained creature, trained checkpoint, executable or credential is included. The candidate is not promoted, and the accepted gameplay default is unchanged. Existing build and test logs are preserved as historical evidence, not a claim that this publication was rebuilt or rerun.

## Integrity and provenance

[receipt.json](receipt.json) binds the nine measurement artifacts by SHA-256 and byte count, including the raw compressed results and complete summary. The decompressed results are 111,116,333 bytes with SHA-256 `2972a0c652c6a93338a74e2a0d9d251ce2b4fe614d68c00f793cd02f8fff9340`. Publication validation checked those hashes and sizes, parsed all result data, and regenerated an identical summary from the recorded data and frozen baseline without a native build or replay.

The four runtime files also match the runtime commit byte-for-byte:

- `crates/alife_training/Cargo.toml`: Git blob `8e96de037b2bba347cbc912b02f557aac2a7d64a`.
- `crates/alife_training/examples/probe_motor_opponent.rs`: Git blob `2eeb35f74d3b8ed0113b3c73871133561d73a5bf`.
- `crates/alife_training/examples/support/motor_opponent.rs`: Git blob `0e1843f7acb667596179340f67e2bbe8ee18eaeb`.
- `crates/alife_training/examples/support/motor_probe_common.rs`: Git blob `3112a14b00e8f7c3bfe2c5878e3b086489c2c81a`.

The original inspiration remains [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/). This experiment records both the useful causal response and the failed cross-context reconciliation so that later work can build on the evidence without rewriting the result.
