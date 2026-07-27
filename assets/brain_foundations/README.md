# N2048 foundation assets

These immutable payloads are the registered N2048 Foundation V1 assets loaded by production
birth, checkpoint capture, and checkpoint restore. The grounded asset contains the curriculum-
trained, evolution-hardened weights and an embedded promotion receipt. The privileged-affordance
asset remains an explicitly unpromoted bootstrap/ablation foundation.

The bootstrap exporter is for test or scratch assets only. Do not run it over this directory: it
would replace the promoted grounded payload with an untrained procedural baseline.

```powershell
cargo run -p alife_core --example export_n2048_foundations -- target/bootstrap-foundations
```

Canonical BLAKE3-256 payload identities:

- Privileged affordance V1: `54231ca95e6b9f65b96e8b09682d726a856068c6b7afe40a31a6f1da4f76ba1a`
- Grounded object slots V1: `d5c69f365b83f46abbe6004326042b7805cf000d0e7d0a63f919d6284dc66e11`
