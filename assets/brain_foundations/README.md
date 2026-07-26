# N2048 foundation assets

These immutable bootstrap payloads are the registered N2048 Foundation V1 assets loaded by
production birth, checkpoint capture, and checkpoint restore. Their embedded manifests mark them
as untrained and unpromoted; Milestone 8 replaces the weight payloads with promoted curriculum
outputs while preserving the frozen ABI and persistent-address map.

Regenerate deterministically from the repository root:

```powershell
cargo run -p alife_core --example export_n2048_foundations -- assets/brain_foundations
```

Canonical BLAKE3-256 payload identities:

- Privileged affordance V1: `54231ca95e6b9f65b96e8b09682d726a856068c6b7afe40a31a6f1da4f76ba1a`
- Grounded object slots V1: `2926b07b0d11348758951168877e47e2208dc742c91c052499203b2cbea14d88`
