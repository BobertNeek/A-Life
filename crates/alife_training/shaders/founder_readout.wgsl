// Offline genetic readout fit. Activations come from production GPU receipts.
struct Parameters { pairs: u32, weights: u32, rate: f32, padding: u32 }
@group(0) @binding(0) var<uniform> parameters: Parameters;
// Positive motor/feature, negative motor/feature, one record per pair/weight.
@group(0) @binding(1) var<storage, read> coefficients: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> biases: array<vec2<f32>>;
@group(0) @binding(3) var<storage, read_write> weights: array<f32>;
@group(0) @binding(4) var<storage, read_write> metrics: array<vec4<f32>>;

@compute @workgroup_size(64)
fn evaluate(@builtin(global_invocation_id) id: vec3<u32>) {
    let pair = id.x;
    if pair >= parameters.pairs { return; }
    var positive = biases[pair].x;
    var negative = biases[pair].y;
    for (var w = 0u; w < parameters.weights; w++) {
        let c = coefficients[pair * parameters.weights + w];
        positive += c.x * c.y * weights[w];
        negative += c.z * c.w * weights[w];
    }
    let margin = positive - negative;
    let loss = max(-margin, 0.0) + log(1.0 + exp(-abs(margin)));
    let gradient = 1.0 / (1.0 + exp(clamp(margin, -80.0, 80.0)));
    metrics[pair] = vec4<f32>(loss, gradient, margin, select(0.0, 1.0, margin > 0.0));
}

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = id.x;
    if w >= parameters.weights { return; }
    var gradient = 0.0;
    for (var pair = 0u; pair < parameters.pairs; pair++) {
        let c = coefficients[pair * parameters.weights + w];
        gradient += metrics[pair].y * (c.x * c.y - c.z * c.w);
    }
    // The fixed source's ActionCandidate routes are MotorProposal projections.
    // Preserve their existing production genetic sign constraint.
    weights[w] = max(0.0, weights[w] + parameters.rate * gradient / f32(parameters.pairs));
}
