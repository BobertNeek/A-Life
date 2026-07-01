// True 2.5D player-view stylization postprocess.
//
// Display-only shader stack:
// - low-resolution pixel-step sampling;
// - four-band toon quantization;
// - Sobel outline from depth with luminance fallback.
//
// This shader is not a neural/runtime kernel and carries no action authority.

struct True25dStylizationSettings {
    pixel_grid: vec2<f32>,
    toon_bands: f32,
    outline_threshold: f32,
    outline_strength: f32,
    depth_outline_strength: f32,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var source_color: texture_2d<f32>;
@group(0) @binding(1) var source_depth: texture_depth_multisampled_2d;
@group(0) @binding(2) var source_sampler: sampler;
@group(0) @binding(3) var<uniform> settings: True25dStylizationSettings;

fn clamp_px(px: vec2<i32>, dims: vec2<i32>) -> vec2<i32> {
    return clamp(px, vec2<i32>(0, 0), dims - vec2<i32>(1, 1));
}

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}

fn depth_at(px: vec2<i32>, dims: vec2<i32>) -> f32 {
    return textureLoad(source_depth, clamp_px(px, dims), 0);
}

fn color_luma_at(px: vec2<i32>, dims: vec2<i32>) -> f32 {
    let uv = (vec2<f32>(clamp_px(px, dims)) + vec2<f32>(0.5, 0.5)) / vec2<f32>(dims);
    return luma(textureSample(source_color, source_sampler, uv).rgb);
}

fn sobel_depth(px: vec2<i32>, dims: vec2<i32>) -> f32 {
    let d00 = depth_at(px + vec2<i32>(-1, -1), dims);
    let d10 = depth_at(px + vec2<i32>(0, -1), dims);
    let d20 = depth_at(px + vec2<i32>(1, -1), dims);
    let d01 = depth_at(px + vec2<i32>(-1, 0), dims);
    let d21 = depth_at(px + vec2<i32>(1, 0), dims);
    let d02 = depth_at(px + vec2<i32>(-1, 1), dims);
    let d12 = depth_at(px + vec2<i32>(0, 1), dims);
    let d22 = depth_at(px + vec2<i32>(1, 1), dims);
    let gx = -d00 - 2.0 * d01 - d02 + d20 + 2.0 * d21 + d22;
    let gy = -d00 - 2.0 * d10 - d20 + d02 + 2.0 * d12 + d22;
    return abs(gx) + abs(gy);
}

fn sobel_luma(px: vec2<i32>, dims: vec2<i32>) -> f32 {
    let c00 = color_luma_at(px + vec2<i32>(-1, -1), dims);
    let c10 = color_luma_at(px + vec2<i32>(0, -1), dims);
    let c20 = color_luma_at(px + vec2<i32>(1, -1), dims);
    let c01 = color_luma_at(px + vec2<i32>(-1, 0), dims);
    let c21 = color_luma_at(px + vec2<i32>(1, 0), dims);
    let c02 = color_luma_at(px + vec2<i32>(-1, 1), dims);
    let c12 = color_luma_at(px + vec2<i32>(0, 1), dims);
    let c22 = color_luma_at(px + vec2<i32>(1, 1), dims);
    let gx = -c00 - 2.0 * c01 - c02 + c20 + 2.0 * c21 + c22;
    let gy = -c00 - 2.0 * c10 - c20 + c02 + 2.0 * c12 + c22;
    return abs(gx) + abs(gy);
}

fn toon_quantize(rgb: vec3<f32>) -> vec3<f32> {
    let bands = max(settings.toon_bands, 2.0);
    let stepped = floor(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * bands) / (bands - 1.0);
    return clamp(stepped, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color_dims_u = textureDimensions(source_color);
    let color_dims = vec2<i32>(i32(color_dims_u.x), i32(color_dims_u.y));
    let pixel_grid = max(settings.pixel_grid, vec2<f32>(1.0, 1.0));
    let stepped_uv = (floor(clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)) * pixel_grid) + vec2<f32>(0.5, 0.5)) / pixel_grid;
    let src = textureSample(source_color, source_sampler, stepped_uv);
    let toon = toon_quantize(src.rgb);

    let px = clamp_px(vec2<i32>(floor(stepped_uv * vec2<f32>(color_dims))), color_dims);
    let depth_edge = smoothstep(
        settings.outline_threshold,
        settings.outline_threshold * 2.4,
        sobel_depth(px, color_dims),
    ) * settings.depth_outline_strength;
    let luma_edge = smoothstep(
        settings.outline_threshold * 0.35,
        settings.outline_threshold * 1.1,
        sobel_luma(px, color_dims),
    ) * settings.outline_strength;
    let edge = clamp(max(depth_edge, luma_edge), 0.0, 1.0);
    let outline_color = vec3<f32>(0.015, 0.022, 0.020);
    return vec4<f32>(mix(toon, outline_color, edge), src.a);
}
