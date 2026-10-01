# Experimental scaled-choice founder

Explicit production launch selection:

```text
production-voxel --new-game --seed 20260920 --population 1 --founder scaled-choice-nociceptive-v1
```

Use a fresh save directory through `--ui-settings PATH`. The default founder stays
`builtin-nano512`. Loading reconstructs each saved genome and cognition.

The current `candidate.alife-foundation` binds the retained historical scaled
weights to the current fixed Nano512 ABI. All 1,799 floating-point weight bit
patterns are preserved exactly. The generator checks unchanged layout, routes,
plasticity, persistent address mapping, and remaining manifest bytes before
writing the current decoder and provenance bindings. No random weights are
generated, and no optimizer or GPU execution occurs.

The current manifest uses `TrainingStageManifest::bootstrap()`: curriculum
version, evaluation version, and completed-stage count are all zero. The fixture
is unpromoted. Current feeding, preference retention, terrain care, and performance
evidence are **Unknown**. Historical measurements do not certify this identity.

- Current canonical digest: see [candidate-digest.txt](candidate-digest.txt).
- Archived manifest bytes, current manifest, and file hashes: [rebind-receipt.json](rebind-receipt.json).
- Candidate ABI: `Nano512ActionCreditCandidateV2`, `SignedChoiceReadouts`.
- Inherited Damage-to-Pain developmental expression floor remains 1.0 on both
  alleles. This makes pain sensing available from birth without writing reward
  or live chemical state.

Reproduce from the repository root using the current core constructor:

```text
cargo run --locked -p alife_tools --example rebind_scaled_choice_founder -- .
```

Current canonical decoding rejects the archived obsolete ABI. The one-off
generator pins the complete original file BLAKE3
`570e3faa82f1f6fccdf1018b7736e057dce4c700d3b549e49747982dce4d0c77`
and length 7,673, checks both count prefixes and the original trailing canonical
digest, then extracts only the known 1,799 little-endian weight words at bytes
445..7641. It constructs the current asset through the strict fixed-graph API.
Only action-decoder bytes 100..132, training stage 262..306, promotion receipt
306..405, asset digest 405..437, and trailing digest 7641..7673 may differ; all
other bytes must match. This does not add historical runtime admission.

The original asset and its README are preserved byte-for-byte in
[historical/2026-09-08-scaled-choice-32416](historical/2026-09-08-scaled-choice-32416/README.md).
Its canonical digest is
`a2cdb86da20f8804c878e85369c299e796b5cac8e6613ba4aef43441a7a6ee1e`; its file SHA256 is
`2d04957b75bda89f73c48c15aed28b9d71c5ec85b408db5bb0f0db1603f922d9`.
That README records arithmetic scaling of ActionCandidate genes by 0.05 from
source digest `28a133ea9eccec463d63d3da4e9a32d06dc2c1716c7127efa64eba26855b80fa`,
zero additional optimizer steps, and bounded September 8 results from source
`7b66a911`. Detailed receipts and the pre-scaling source were cited at an external
Windows evidence directory and are unavailable in this checkout. Their historical
training-stage metadata stays in the archived original; it is not a current claim.

The old source-specific manual GPU probes keep their original digest pins. A
current behavioral claim requires fresh measurements of the regenerated asset on
the exact source and adapter. This explicit selection is not default promotion.
