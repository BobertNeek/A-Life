// P24 contract-only WGSL stub.
//
// This file documents shader-visible buffer expectations shared by P25/P26
// compute entry points. It is intentionally not a runtime kernel.
//
// Binding contract:
// - Tile metadata records are 32 bytes and page-relative.
// - Supertile masks are 24 bytes and split the 64-bit microtile mask into
//   two u32 words.
// - Packed synapse indices are 16 bytes: target, source, weight, tile index.
// - Routing descriptors are 64 bytes and carry lobe/routing/cadence metadata.
// - Weight layers are separate fixed-point buffers:
//   W_genetic_fixed, W_lifetime_consolidated, alpha, H_operational, H_shadow.
// - Activations are ping-pong buffers. Accumulators are i32 atomic buffers.
// - Diagnostics carry overflow/range/NaN and active tile/synapse counters.
// - Action summary staging is compact and double-bufferable; it is not the
//   public structured action ABI.
//
// Pass contract:
// pass 0 clear_accumulators:
//   Clear i32 accumulators and reset diagnostic counters.
//
// pass 1 sparse_projection_spmv:
//   Read routing descriptors, supertile masks, tile metadata, packed indices,
//   fixed genetic weights, lifetime weights, alpha, and H_operational. Write
//   fixed-point deltas through atomic accumulator adds. Set overflow flags.
//
// pass 2 activation_finalize:
//   Clamp accumulator values to the fixed activation range, write the next
//   activation ping-pong buffer, and preserve diagnostic counters.
//
// pass 3 plasticity_update:
//   Update H_shadow according to the CPU oracle after pass-2 final activations
//   are stable. Genetic fixed, lifetime consolidated, and H_operational layers
//   are immutable in this pass. The executable diagnostic shader lives in P26.
//
// Later hooks:
//   Super-tile culling refinement and sleep/offline structural recompaction
//   are deferred to P27/P28 and must preserve page-relative offsets.
