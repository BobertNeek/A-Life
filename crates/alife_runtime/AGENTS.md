# alife_runtime Instructions

Root [AGENTS.md](../../AGENTS.md) applies. These local rules are current
implementation guardrails; they do not amend the controlling v2.0 architecture.

This crate owns the GPU-authoritative neural session used by gameplay, GPU
training, evolution, and challenge worlds. Device-free canonical preparation may
also serve the explicitly authorized offline CPU training backend.

- Own GPU backend lifetime, fail-stop authority, sealed checkpoint boundaries,
  and the latest durable checkpoint reference.
- Keep game state, Bevy, renderer, UI, and world legality out of this crate.
- Device loss or backend unavailability must stop neural actions; never add a
  live CPU neural shadow, fallback, or parity handoff. Explicit offline native
  founder/optimizer transfer is separate from live individual/GPU authority.
- Bulk capture and restore may occur only at explicit sealed boundaries.
- Exact-resume checkpoints preserve typed passive life statistics alongside
  the existing bounded cognitive sidecars.
- Keep consumer-specific orchestration outside this crate so every consumer
  drives the same session contract.
