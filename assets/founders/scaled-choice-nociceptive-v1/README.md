# Experimental scaled-choice founder

Select it explicitly for a new game:

```text
production-voxel --new-game --seed 20260920 --population 1 --founder scaled-choice-nociceptive-v1
```

Use `--ui-settings PATH` for a fresh save directory. The default founder is
`builtin-nano512`; loading reconstructs the saved individuals.

`candidate.alife-foundation` rebinds the retained historical scaled weights to the
current fixed Nano512 ABI. All 1,799 weight bit patterns are preserved. The
inherited Damage-to-Pain expression floor remains 1.0 on both alleles. Rebinding
performs no training and supplies no current feeding, preference, terrain-care or
performance measurements.

The current asset uses zero completed training stages. Its digest is in
[candidate-digest.txt](candidate-digest.txt); exact original/current manifest bytes
and hashes are in [rebind-receipt.json](rebind-receipt.json).

Reproduce the checked conversion from the repository root:

```text
cargo run --locked -p alife_tools --example rebind_scaled_choice_founder -- .
```

The converter hash-pins the original file and extracts only its known weight
payload. It does not relax runtime decoding of the obsolete ABI. The original
asset and source-specific observations remain unchanged in
[historical/2026-09-08-scaled-choice-32416](historical/2026-09-08-scaled-choice-32416/README.md).
The detailed external receipts cited there are unavailable in this checkout.
