# Experimental scaled-choice founder

Explicit production launch selection:

```text
production-voxel --new-game --seed 20260920 --population 1 --founder scaled-choice-nociceptive-v1
```

Use a fresh save directory through `--ui-settings PATH`. The default founder stays
`builtin-nano512`. Loading a save reconstructs its stored genome and cognition;
the launch selection does not replace saved individuals.

The bundled 7,673-byte asset is copied exactly from
`D:/A life/.worktrees/initial-founder-training-20260907/target/founder-training-evidence/scaled-choice-32416/candidate.alife-foundation`.

- Canonical digest: `a2cdb86da20f8804c878e85369c299e796b5cac8e6613ba4aef43441a7a6ee1e`.
- File SHA256: `2d04957b75bda89f73c48c15aed28b9d71c5ec85b408db5bb0f0db1603f922d9`.
- Candidate ABI: `Nano512ActionCreditCandidateV2`, `SignedChoiceReadouts`.
- Inherited body configuration: Damage-to-Pain emitter developmental expression
  floor is 1.0 on both alleles. This matches the retained fixtures and makes pain
  sensing available from birth; it does not write reward or live chemical state.

This asset scales the source's ActionCandidate genetic weights by 0.05. It is
arithmetic scaling with zero additional optimizer steps, not new training.
Source canonical digest:
`28a133ea9eccec463d63d3da4e9a32d06dc2c1716c7127efa64eba26855b80fa`.

Retained September 8 evidence under the same evidence directory:

- `scaled-choice-32416/completion.json`: three flat-ground feeding cases passed;
  two color-swapped food lives chose nutritious food on all final 16 opportunities.
- `choice-retention-32028/completion.json`: acquired preference, exact restore,
  one sleep cycle, and post-wake preference passed. Its candidate copy is identical.
- `choice-retention-polled-live-receipt.json`: source HEAD `7b66a911`, exit 0.
  Restore hardware was NVIDIA GeForce RTX 3050 with Vulkan.

These are bounded experimental receipts. Sleep used the public recovery request;
food-choice fixtures kept nearby food available. They do not establish production
terrain competence, hungry-body recovery, fatigue-driven rest, player teaching,
or current-build performance. This selection is not default promotion.
