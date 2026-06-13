// P25 static forward passes 0-2.
//
// Dispatch policy:
// - Workgroup size: 64 invocations.
// - pass 0 dispatches ceil((neuron_count + 8 diagnostic words) / 64).
// - pass 1 dispatches ceil(packed_synapse_count / 64).
// - pass 2 dispatches ceil(neuron_count / 64).
//
// Buffer assumptions:
// - Activations are signed Q32767 i32 values.
// - Effective weights are precomputed signed Q4096 i32 values for P25 parity.
// - Accumulators are signed i32 atomics in activation scale.
// - Diagnostics are eight u32 atomics in the P24 counter order.
// - Dense16x16 and COO tiles are already flattened into packed synapse records.
// - Plasticity, row/column-run payload execution, P27 routing, and P28
//   recompaction are intentionally unsupported here.

const WORKGROUP_SIZE: u32 = 64u;
const DIAGNOSTIC_WORDS: u32 = 8u;

const DIAG_OVERFLOW_FLAGS: u32 = 0u;
const DIAG_OVERFLOW_COUNT: u32 = 1u;
const DIAG_RANGE_REJECTIONS: u32 = 2u;
const DIAG_NAN_REJECTIONS: u32 = 3u;
const DIAG_ACTIVE_TILES: u32 = 4u;
const DIAG_ACTIVE_SYNAPSES: u32 = 5u;
const DIAG_MASK_SKIPPED_TILES: u32 = 6u;
const DIAG_UNSUPPORTED_TILES: u32 = 7u;

struct StaticForwardParams {
    neuron_count: u32,
    synapse_count: u32,
    tile_count: u32,
    supertile_mask_count: u32,
    weight_scale: i32,
    activation_clamp_min_q: i32,
    activation_clamp_max_q: i32,
    accumulator_abs_limit_q: i32,
}

struct TileMetadata {
    projection_index: u32,
    microtile_row: u32,
    microtile_col: u32,
    tile_type: u32,
    nonzero_count: u32,
    synapse_offset: u32,
    synapse_count: u32,
    flags: u32,
}

struct SupertileMask {
    projection_index: u32,
    supertile_row: u32,
    supertile_col: u32,
    active_microtile_mask_lo: u32,
    active_microtile_mask_hi: u32,
    flags: u32,
}

struct PackedSynapseIndex {
    target_index: u32,
    source_index: u32,
    weight_index: u32,
    tile_metadata_index: u32,
}

@group(0) @binding(0)
var<storage, read> params: StaticForwardParams;
@group(0) @binding(1)
var<storage, read> tile_metadata: array<TileMetadata>;
@group(0) @binding(2)
var<storage, read> supertile_masks: array<SupertileMask>;
@group(0) @binding(3)
var<storage, read> packed_indices: array<PackedSynapseIndex>;
@group(0) @binding(4)
var<storage, read> effective_weight_q: array<i32>;
@group(0) @binding(5)
var<storage, read> activation_read_q: array<i32>;
@group(0) @binding(6)
var<storage, read_write> accumulators_q: array<atomic<i32>>;
@group(0) @binding(7)
var<storage, read_write> activation_write_q: array<i32>;
@group(0) @binding(8)
var<storage, read_write> diagnostics: array<atomic<u32>>;

fn abs_i32(value: i32) -> i32 {
    if (value < 0) {
        return -value;
    }
    return value;
}

fn div_round_signed(numerator: i32, denominator: i32) -> i32 {
    let half = denominator / 2;
    if (numerator >= 0) {
        return (numerator + half) / denominator;
    }
    return (numerator - half) / denominator;
}

fn clamp_i32(value: i32, min_value: i32, max_value: i32) -> i32 {
    return min(max(value, min_value), max_value);
}

fn tile_is_active(tile: TileMetadata) -> bool {
    if (params.supertile_mask_count == 0u) {
        return true;
    }
    let supertile_row = tile.microtile_row / 8u;
    let supertile_col = tile.microtile_col / 8u;
    let local_row = tile.microtile_row % 8u;
    let local_col = tile.microtile_col % 8u;
    let local_bit = local_row * 8u + local_col;
    var index = 0u;
    loop {
        if (index >= params.supertile_mask_count) {
            break;
        }
        let mask = supertile_masks[index];
        if (mask.projection_index == tile.projection_index &&
            mask.supertile_row == supertile_row &&
            mask.supertile_col == supertile_col) {
            if (local_bit < 32u) {
                return (mask.active_microtile_mask_lo & (1u << local_bit)) != 0u;
            }
            return (mask.active_microtile_mask_hi & (1u << (local_bit - 32u))) != 0u;
        }
        index = index + 1u;
    }
    return false;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn clear_accumulators(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index < params.neuron_count) {
        atomicStore(&accumulators_q[index], 0);
    }
    if (index < DIAGNOSTIC_WORDS) {
        atomicStore(&diagnostics[index], 0u);
    }
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn sparse_projection_spmv(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let synapse_index = global_id.x;
    if (synapse_index >= params.synapse_count) {
        return;
    }

    let packed = packed_indices[synapse_index];
    if (packed.tile_metadata_index >= params.tile_count) {
        atomicAdd(&diagnostics[DIAG_UNSUPPORTED_TILES], 1u);
        return;
    }
    let tile = tile_metadata[packed.tile_metadata_index];
    if (tile.tile_type != 1u && tile.tile_type != 2u) {
        if (synapse_index == tile.synapse_offset) {
            atomicAdd(&diagnostics[DIAG_UNSUPPORTED_TILES], 1u);
        }
        return;
    }
    if (!tile_is_active(tile)) {
        if (synapse_index == tile.synapse_offset) {
            atomicAdd(&diagnostics[DIAG_MASK_SKIPPED_TILES], 1u);
        }
        return;
    }

    if (synapse_index == tile.synapse_offset) {
        atomicAdd(&diagnostics[DIAG_ACTIVE_TILES], 1u);
    }
    atomicAdd(&diagnostics[DIAG_ACTIVE_SYNAPSES], 1u);

    let source_q = activation_read_q[packed.source_index];
    let weight_q = effective_weight_q[packed.weight_index];
    let delta_q = div_round_signed(source_q * weight_q, params.weight_scale);
    let previous_q = atomicAdd(&accumulators_q[packed.target_index], delta_q);
    let next_q = previous_q + delta_q;
    if (abs_i32(next_q) > params.accumulator_abs_limit_q) {
        atomicOr(&diagnostics[DIAG_OVERFLOW_FLAGS], 1u);
        atomicAdd(&diagnostics[DIAG_OVERFLOW_COUNT], 1u);
    }
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn activation_finalize(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.neuron_count) {
        return;
    }
    let raw_q = atomicLoad(&accumulators_q[index]);
    if (abs_i32(raw_q) > params.accumulator_abs_limit_q) {
        atomicOr(&diagnostics[DIAG_OVERFLOW_FLAGS], 1u);
        atomicAdd(&diagnostics[DIAG_OVERFLOW_COUNT], 1u);
    }
    let clamped_q = clamp_i32(
        raw_q,
        params.activation_clamp_min_q,
        params.activation_clamp_max_q,
    );
    if (clamped_q != raw_q) {
        atomicAdd(&diagnostics[DIAG_RANGE_REJECTIONS], 1u);
    }
    activation_write_q[index] = clamped_q;
}
