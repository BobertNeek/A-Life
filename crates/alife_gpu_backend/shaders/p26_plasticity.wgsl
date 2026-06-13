// P26 pass 3 fixed-point Oja plasticity update.
//
// Dispatch policy:
// - Workgroup size: 64 invocations.
// - pass 3 dispatches ceil(packed_synapse_count / 64).
//
// Buffer assumptions:
// - Previous and finalized activations are signed Q32767 i32 values.
// - H_shadow traces are stored as signed fixed-point weight-scale values.
// - The shader uses i32 intermediates and deterministic LFSR stochastic
//   rounding. Host upload keeps the contract trace layer as INT16.
// - The host initializes diagnostics before dispatch; this pass only increments
//   counters to avoid same-pass clear/update hazards.
// - Each invocation owns exactly one packed synapse/weight slot and writes only
//   h_shadow_write_q[weight_index]. Genetic, lifetime, and H_operational layers
//   are not bound as writable buffers in this pass.
// - P27 routing and P28 recompaction are intentionally unsupported here.

const WORKGROUP_SIZE: u32 = 64u;
const Q16_DENOMINATOR: i32 = 65535;

const DIAG_OVERFLOW_FLAGS: u32 = 0u;
const DIAG_OVERFLOW_COUNT: u32 = 1u;
const DIAG_SATURATION_COUNT: u32 = 2u;
const DIAG_ALPHA_ZERO_SKIPS: u32 = 3u;
const DIAG_ACTIVE_TILES: u32 = 4u;
const DIAG_ACTIVE_SYNAPSES: u32 = 5u;
const DIAG_MASK_SKIPPED_TILES: u32 = 6u;
const DIAG_UNSUPPORTED_TILES: u32 = 7u;

struct PlasticityParams {
    neuron_count: u32,
    synapse_count: u32,
    tile_count: u32,
    supertile_mask_count: u32,
    activation_scale: i32,
    weight_scale: i32,
    learning_rate_q16: u32,
    decay_q16: u32,
    shadow_min_q: i32,
    shadow_max_q: i32,
    stochastic_seed: u32,
    reserved0: u32,
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
var<storage, read> params: PlasticityParams;
@group(0) @binding(1)
var<storage, read> tile_metadata: array<TileMetadata>;
@group(0) @binding(2)
var<storage, read> supertile_masks: array<SupertileMask>;
@group(0) @binding(3)
var<storage, read> packed_indices: array<PackedSynapseIndex>;
@group(0) @binding(4)
var<storage, read> alpha_q16: array<u32>;
@group(0) @binding(5)
var<storage, read> previous_activation_q: array<i32>;
@group(0) @binding(6)
var<storage, read> finalized_activation_q: array<i32>;
@group(0) @binding(7)
var<storage, read> h_shadow_read_q: array<i32>;
@group(0) @binding(8)
var<storage, read_write> h_shadow_write_q: array<i32>;
@group(0) @binding(9)
var<storage, read_write> diagnostics: array<atomic<u32>>;

fn abs_i32(value: i32) -> i32 {
    if (value < 0) {
        return -value;
    }
    return value;
}

fn lfsr32(seed: u32) -> u32 {
    var value = seed;
    if (value == 0u) {
        value = 2738958700u;
    }
    value = value ^ (value << 13u);
    value = value ^ (value >> 17u);
    value = value ^ (value << 5u);
    return value;
}

fn stochastic_round_div_signed(numerator: i32, denominator: i32, seed: u32) -> i32 {
    let magnitude = abs_i32(numerator);
    let base = magnitude / denominator;
    let remainder = magnitude % denominator;
    let threshold = i32(lfsr32(seed) % u32(denominator));
    var rounded = base;
    if (threshold < remainder) {
        rounded = rounded + 1;
    }
    if (numerator < 0) {
        return -rounded;
    }
    return rounded;
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

fn oja_delta_q(pre_q: i32, post_q: i32, current_q: i32, seed: u32) -> i32 {
    let pre_post_activation_q = stochastic_round_div_signed(
        pre_q * post_q,
        params.activation_scale,
        seed ^ 4097u,
    );
    let pre_post_weight_q = stochastic_round_div_signed(
        pre_post_activation_q * params.weight_scale,
        params.activation_scale,
        seed ^ 4098u,
    );
    let post_sq_activation_q = stochastic_round_div_signed(
        post_q * post_q,
        params.activation_scale,
        seed ^ 4099u,
    );
    let post_sq_current_q = stochastic_round_div_signed(
        post_sq_activation_q * current_q,
        params.activation_scale,
        seed ^ 4100u,
    );
    let decayed_current_q = stochastic_round_div_signed(
        post_sq_current_q * i32(params.decay_q16),
        Q16_DENOMINATOR,
        seed ^ 4101u,
    );
    let signal_q = pre_post_weight_q - decayed_current_q;
    return stochastic_round_div_signed(
        signal_q * i32(params.learning_rate_q16),
        Q16_DENOMINATOR,
        seed ^ 4102u,
    );
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn plasticity_update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let synapse_index = global_id.x;
    if (synapse_index >= params.synapse_count) {
        return;
    }

    let packed = packed_indices[synapse_index];
    let weight_index = packed.weight_index;
    h_shadow_write_q[weight_index] = h_shadow_read_q[weight_index];

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

    let alpha = alpha_q16[weight_index];
    if (alpha == 0u) {
        atomicAdd(&diagnostics[DIAG_ALPHA_ZERO_SKIPS], 1u);
        return;
    }

    let seed = params.stochastic_seed ^ (weight_index * 2654435769u);
    let pre_q = previous_activation_q[packed.source_index];
    let post_q = finalized_activation_q[packed.target_index];
    let current_q = h_shadow_read_q[weight_index];
    let unclamped_q = current_q + oja_delta_q(pre_q, post_q, current_q, seed);
    if (unclamped_q < params.shadow_min_q || unclamped_q > params.shadow_max_q) {
        atomicAdd(&diagnostics[DIAG_SATURATION_COUNT], 1u);
    }
    if (unclamped_q < -32768 || unclamped_q > 32767) {
        atomicOr(&diagnostics[DIAG_OVERFLOW_FLAGS], 1u);
        atomicAdd(&diagnostics[DIAG_OVERFLOW_COUNT], 1u);
    }
    h_shadow_write_q[weight_index] = clamp_i32(
        clamp_i32(unclamped_q, params.shadow_min_q, params.shadow_max_q),
        -32768,
        32767,
    );
}
