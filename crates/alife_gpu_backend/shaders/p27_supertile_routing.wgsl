// P27 supertile culling and active-mask routing contract.
//
// This WGSL module is a compile-checked helper contract for P27. P25/P26
// diagnostic passes may inline equivalent logic, but the indexing rules here
// are the source contract for future shared shader modules.
//
// Portability notes:
// - P25 currently uses nine storage-buffer bindings for diagnostic parity.
// - P26 currently uses ten storage-buffer bindings for diagnostic parity.
// - Passing local diagnostic tests does not prove product WebGPU portability.
// - Diagnostics/export readback is allowed; active gameplay neural readback is
//   not allowed.
// - Dispatch-level culling is deferred; shader early-exit is the P27 baseline.

const P27_SUPERTILE_MICROTILES: u32 = 8u;
const P27_SUPERTILE_MASK_BITS: u32 = 64u;

struct P27SupertileMask {
    projection_index: u32,
    supertile_row: u32,
    supertile_col: u32,
    active_microtile_mask_lo: u32,
    active_microtile_mask_hi: u32,
    flags: u32,
}

fn p27_local_bit(microtile_row: u32, microtile_col: u32) -> u32 {
    let local_row = microtile_row % P27_SUPERTILE_MICROTILES;
    let local_col = microtile_col % P27_SUPERTILE_MICROTILES;
    return local_row * P27_SUPERTILE_MICROTILES + local_col;
}

fn p27_supertile_row(microtile_row: u32) -> u32 {
    return microtile_row / P27_SUPERTILE_MICROTILES;
}

fn p27_supertile_col(microtile_col: u32) -> u32 {
    return microtile_col / P27_SUPERTILE_MICROTILES;
}

fn p27_microtile_is_active(
    mask: P27SupertileMask,
    projection_index: u32,
    microtile_row: u32,
    microtile_col: u32,
) -> bool {
    if (mask.projection_index != projection_index) {
        return false;
    }
    if (mask.supertile_row != p27_supertile_row(microtile_row)) {
        return false;
    }
    if (mask.supertile_col != p27_supertile_col(microtile_col)) {
        return false;
    }

    let local_bit = p27_local_bit(microtile_row, microtile_col);
    if (local_bit >= P27_SUPERTILE_MASK_BITS) {
        return false;
    }
    if (local_bit < 32u) {
        return (mask.active_microtile_mask_lo & (1u << local_bit)) != 0u;
    }
    return (mask.active_microtile_mask_hi & (1u << (local_bit - 32u))) != 0u;
}
