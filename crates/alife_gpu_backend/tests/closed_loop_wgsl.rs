//! Contract-first tests for GPU-authoritative sensory encoding and recurrent dynamics.

#[cfg(feature = "gpu-tests")]
#[path = "closed_loop_wgsl/hardware.rs"]
mod hardware;
#[cfg(feature = "gpu-tests")]
mod support;

use std::collections::BTreeMap;

use alife_gpu_backend::{
    validate_dispatch_dimensions, GpuBufferAccess, GpuClassBucketBufferRole, GpuClassBucketBuffers,
    GpuClosedLoopError, GpuClosedLoopPipelines, CLOSED_LOOP_ABI_WGSL,
    CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL, CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_RECURRENT_WGSL,
    GPU_ACTIVE_DISPATCH_ROW_WORDS, GPU_ACTIVE_SIDE_DIAGNOSTIC_LANE,
};
use naga::{
    AddressSpace, Arena, Binding, BuiltIn, Expression, Function, Handle, ScalarKind, ShaderStage,
    Statement, StorageAccess, TypeInner,
};

fn validated_module(source: &str) -> naga::Module {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap();
    module
}

fn compact(source: &str) -> String {
    source.split_whitespace().collect()
}

fn entry_body<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let start = source.find(&marker).expect("required WGSL function");
    &source[start..]
}

#[derive(Default)]
struct IrEffects {
    loads: BTreeMap<(u32, u32), usize>,
    writes: BTreeMap<(u32, u32), usize>,
    atomics: usize,
}

fn reachable_ir_effects(module: &naga::Module, entry_name: &str) -> IrEffects {
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == entry_name)
        .unwrap();
    let mut functions = Vec::new();
    collect_calls(&entry.function.body, &mut functions);
    let mut cursor = 0;
    while cursor < functions.len() {
        let handle = functions[cursor];
        collect_calls(&module.functions[handle].body, &mut functions);
        cursor += 1;
    }

    let mut effects = IrEffects::default();
    collect_function_effects(module, &entry.function, &mut effects);
    for handle in functions {
        collect_function_effects(module, &module.functions[handle], &mut effects);
    }
    effects
}

fn collect_calls(block: &naga::Block, output: &mut Vec<Handle<Function>>) {
    for statement in block.iter() {
        match statement {
            Statement::Call { function, .. } => {
                if !output.contains(function) {
                    output.push(*function);
                }
            }
            Statement::Block(block) => collect_calls(block, output),
            Statement::If { accept, reject, .. } => {
                collect_calls(accept, output);
                collect_calls(reject, output);
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    collect_calls(&case.body, output);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                collect_calls(body, output);
                collect_calls(continuing, output);
            }
            _ => {}
        }
    }
}

fn collect_function_effects(module: &naga::Module, function: &Function, out: &mut IrEffects) {
    for (_, expression) in function.expressions.iter() {
        if let Expression::Load { pointer } = expression {
            if let Some(binding) = expression_binding(module, &function.expressions, *pointer) {
                *out.loads.entry(binding).or_insert(0) += 1;
            }
        }
    }
    collect_statement_effects(module, &function.expressions, &function.body, out);
}

fn collect_statement_effects(
    module: &naga::Module,
    expressions: &Arena<Expression>,
    block: &naga::Block,
    out: &mut IrEffects,
) {
    for statement in block.iter() {
        match statement {
            Statement::Store { pointer, .. } => {
                if let Some(binding) = expression_binding(module, expressions, *pointer) {
                    *out.writes.entry(binding).or_insert(0) += 1;
                }
            }
            Statement::Atomic { pointer, .. } => {
                out.atomics += 1;
                if let Some(binding) = expression_binding(module, expressions, *pointer) {
                    *out.writes.entry(binding).or_insert(0) += 1;
                }
            }
            Statement::Block(block) => collect_statement_effects(module, expressions, block, out),
            Statement::If { accept, reject, .. } => {
                collect_statement_effects(module, expressions, accept, out);
                collect_statement_effects(module, expressions, reject, out);
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    collect_statement_effects(module, expressions, &case.body, out);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                collect_statement_effects(module, expressions, body, out);
                collect_statement_effects(module, expressions, continuing, out);
            }
            _ => {}
        }
    }
}

fn expression_binding(
    module: &naga::Module,
    expressions: &Arena<Expression>,
    handle: Handle<Expression>,
) -> Option<(u32, u32)> {
    let global = match &expressions[handle] {
        Expression::GlobalVariable(global) => *global,
        Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => {
            return expression_binding(module, expressions, *base);
        }
        _ => return None,
    };
    module.global_variables[global]
        .binding
        .as_ref()
        .map(|binding| (binding.group, binding.binding))
}

fn entry_uses_global_invocation_id(module: &naga::Module) -> bool {
    module.entry_points[0]
        .function
        .arguments
        .iter()
        .any(|argument| {
            matches!(
                argument.binding.as_ref(),
                Some(Binding::BuiltIn(BuiltIn::GlobalInvocationId))
            )
        })
}

#[test]
fn closed_loop_wgsl_parses_validates_and_exposes_only_the_required_entries() {
    for (source, entry, workgroup) in [
        (
            CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
            "clear_diagnostics",
            [1, 1, 1],
        ),
        (CLOSED_LOOP_ENCODE_WGSL, "encode_perception", [64, 1, 1]),
        (
            CLOSED_LOOP_RECURRENT_WGSL,
            "recurrent_microstep",
            [64, 1, 1],
        ),
    ] {
        let module = validated_module(source);
        assert_eq!(module.entry_points.len(), 1);
        let point = &module.entry_points[0];
        assert_eq!(point.name, entry);
        assert_eq!(point.stage, ShaderStage::Compute);
        assert_eq!(point.workgroup_size, workgroup);
    }
}

#[test]
fn recurrent_shader_has_four_host_specialized_microstep_variants() {
    let module = validated_module(CLOSED_LOOP_RECURRENT_WGSL);
    let overrides = module
        .overrides
        .iter()
        .filter_map(|(_, value)| value.name.as_deref())
        .collect::<Vec<_>>();
    assert!(overrides.contains(&"microstep_index"));
    assert_eq!(GpuClosedLoopPipelines::recurrent_variant_count(), 4);
    assert_eq!(
        GpuClosedLoopPipelines::recurrent_variant_microstep_indices(),
        [0, 1, 2, 3]
    );
    for count in 2..=4 {
        assert!(GpuClosedLoopPipelines::validate_microstep_count(count).is_ok());
    }
    assert!(GpuClosedLoopPipelines::validate_microstep_count(1).is_err());
    assert!(GpuClosedLoopPipelines::validate_microstep_count(5).is_err());
}

#[test]
fn each_shader_concatenates_the_canonical_abi_prefix_exactly_once() {
    let abi_anchor = "struct GpuBrainSlotRecord";
    assert_eq!(CLOSED_LOOP_ABI_WGSL.matches(abi_anchor).count(), 1);
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
    ] {
        assert!(source.starts_with(CLOSED_LOOP_ABI_WGSL));
        assert_eq!(source.matches(abi_anchor).count(), 1);
        for declaration in [
            "struct GpuPerceptionHeader",
            "struct GpuBrainSlotRecord",
            "struct GpuPhenotypeIdentityRecord",
            "struct GpuCandidateRecord",
            "struct GpuSelectionRecord",
            "struct GpuEncoderPlanRecord",
            "struct GpuEncoderAssignmentRecord",
            "struct GpuNeuronDynamicsRecord",
            "struct GpuProjectionRecord",
            "struct GpuRouteMetadataRecord",
            "struct GpuDecoderPlanRecord",
            "struct GpuDecoderFamilyRecord",
            "struct GpuDecoderWeightIndexRecord",
        ] {
            assert_eq!(source.matches(declaration).count(), 1, "{declaration}");
        }
    }
}

#[test]
fn naga_reflection_matches_the_exact_seven_heap_bindings_and_access_modes() {
    let expected_manifest = GpuClassBucketBuffers::neural_binding_manifest();
    assert_eq!(expected_manifest.len(), 7);
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
    ] {
        let module = validated_module(source);
        let mut reflected = BTreeMap::new();
        for (_, global) in module.global_variables.iter() {
            let Some(binding) = global.binding.as_ref() else {
                continue;
            };
            let AddressSpace::Storage { access } = global.space else {
                panic!("bound global must be storage: {:?}", global.name);
            };
            assert!(reflected
                .insert((binding.group, binding.binding), access)
                .is_none());
        }
        assert_eq!(reflected.len(), 7);
        for expected in expected_manifest {
            let reflected_access = reflected
                .get(&(expected.group, expected.binding))
                .expect("missing reflected neural binding");
            let expected_access = match expected.access {
                GpuBufferAccess::ReadOnly => StorageAccess::LOAD,
                GpuBufferAccess::ReadWrite => StorageAccess::LOAD | StorageAccess::STORE,
            };
            assert_eq!(*reflected_access, expected_access, "{:?}", expected.role);
            assert!(!expected.role.is_staging_or_readback());
            assert!(expected.neural_pipeline_bindable);
        }
        assert!(!reflected.values().any(|access| access.is_empty()));
    }
}

#[test]
fn mutable_heap_is_atomic_u32_while_dispatch_headers_remain_read_only() {
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
    ] {
        let module = validated_module(source);
        let (_, mutable) = module
            .global_variables
            .iter()
            .find(|(_, global)| {
                global
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.group == 0 && binding.binding == 6)
            })
            .unwrap();
        let TypeInner::Array { base, .. } = &module.types[mutable.ty].inner else {
            panic!("mutable heap must be a runtime array")
        };
        let TypeInner::Atomic(scalar) = &module.types[*base].inner else {
            panic!("mutable heap elements must be atomic")
        };
        assert_eq!(scalar.kind, ScalarKind::Uint);
        assert_eq!(scalar.width, 4);

        let (_, headers) = module
            .global_variables
            .iter()
            .find(|(_, global)| {
                global
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.group == 0 && binding.binding == 4)
            })
            .unwrap();
        assert_eq!(
            headers.space,
            AddressSpace::Storage {
                access: StorageAccess::LOAD
            }
        );
    }
    assert_eq!(GPU_ACTIVE_SIDE_DIAGNOSTIC_LANE, 3);
}

#[test]
fn dispatch_dimension_boundaries_are_checked_without_truncation() {
    assert_eq!(
        validate_dispatch_dimensions(512, 65_535, 65_535).unwrap(),
        [8, 65_535, 1]
    );
    assert_eq!(
        validate_dispatch_dimensions(512, 65_536, 65_535),
        Err(GpuClosedLoopError::CapacityExceeded)
    );
    assert_eq!(
        validate_dispatch_dimensions(u32::MAX, 1, 65_535),
        Err(GpuClosedLoopError::CapacityExceeded)
    );
    assert_eq!(GPU_ACTIVE_DISPATCH_ROW_WORDS, 272);
}

#[test]
fn encode_uses_gid_y_as_the_active_batch_row_and_current_canonical_frame() {
    let module = validated_module(CLOSED_LOOP_ENCODE_WGSL);
    assert!(entry_uses_global_invocation_id(&module));
    let effects = reachable_ir_effects(&module, "encode_perception");
    for required in [(0, 0), (0, 2), (0, 4), (0, 5)] {
        assert!(
            effects.loads.contains_key(&required),
            "encode entry has no reachable load from {required:?}"
        );
    }
    assert!(effects.writes.contains_key(&(0, 6)));
    let body = compact(entry_body(CLOSED_LOOP_ENCODE_WGSL, "encode_perception"));
    for required in [
        "gid.y",
        "slot_generation",
        "target_offsets_offset",
        "sensory_offset",
        "encoded_input_offset",
    ] {
        assert!(
            body.contains(required),
            "missing encode causal fragment: {required}"
        );
    }
    assert!(body.contains("clamp("));
}

#[test]
fn encoder_source_groups_are_bounds_checked_and_concatenated_exactly_once() {
    let body = compact(entry_body(
        CLOSED_LOOP_ENCODE_WGSL,
        "resolve_encoder_source_lane",
    ));
    for required in [
        "source_group_raw==1u",
        "source_index<encoder.sensory_lane_count",
        "source_group_raw==2u",
        "source_index<encoder.body_lane_count",
        "encoder.sensory_lane_count+source_index",
        "source_group_raw==3u",
        "source_index<encoder.homeostasis_lane_count",
        "encoder.sensory_lane_count+encoder.body_lane_count+source_index",
    ] {
        assert!(
            body.contains(required),
            "missing source-group fragment: {required}"
        );
    }
    assert!(!body.contains("+encoder.sensory_lane_count+encoder.sensory_lane_count"));
}

#[test]
fn recurrent_shader_uses_target_major_csr_and_never_projection_provenance_for_weights() {
    let module = validated_module(CLOSED_LOOP_RECURRENT_WGSL);
    assert!(entry_uses_global_invocation_id(&module));
    let effects = reachable_ir_effects(&module, "recurrent_microstep");
    for required in [(0, 0), (0, 2), (0, 3), (0, 4), (0, 6)] {
        assert!(
            effects.loads.contains_key(&required),
            "recurrent entry has no reachable load from {required:?}"
        );
    }
    assert!(effects.writes.contains_key(&(0, 6)));
    assert!(effects.atomics > 0);
    let body = compact(entry_body(
        CLOSED_LOOP_RECURRENT_WGSL,
        "recurrent_microstep",
    ));
    for required in [
        "gid.y",
        "target_offsets_offset",
        "source_indices_offset",
        "route_indices_offset",
        "route_metadata_offset",
        "genetic_weight_offset",
        "alpha_offset",
        "lifetime_weight_offset",
        "fast_weight_offset",
    ] {
        assert!(
            body.contains(required),
            "missing recurrent CSR fragment: {required}"
        );
    }
    for forbidden in [
        "projection.synapse_start",
        "projection.synapse_count",
        "brain.projection_offset+cursor",
    ] {
        assert!(
            !body.contains(forbidden),
            "provenance span used by kernel: {forbidden}"
        );
    }
}

#[test]
fn recurrent_shader_applies_cadence_zero_delay_and_exact_effective_weight() {
    let body = compact(entry_body(
        CLOSED_LOOP_RECURRENT_WGSL,
        "recurrent_microstep",
    ));
    assert!(body.contains("route.delay_microsteps!=0u"));
    assert!(body.contains("route_fires("));
    assert!(body.contains("microstep_index"));
    assert!(body.contains("genetic+lifetime+alpha*fast"));
    assert!(!body.contains("genetic+fast+alpha*lifetime"));
}

#[test]
fn every_microstep_keeps_current_input_and_updates_dynamics_homeostasis_and_diagnostics() {
    let body = compact(entry_body(
        CLOSED_LOOP_RECURRENT_WGSL,
        "recurrent_microstep",
    ));
    for required in [
        "brain.encoded_input_offset+target",
        "brain.neuron_dynamics_offset+target*8u",
        "brain.neuron_homeostasis_offset+target*2u",
        "homeostatic_gain",
        "metabolic_load",
        "activity_ema_decay",
        "metabolic_decay",
        "abs(output)",
        "output*output",
        "clamp(",
        "is_finite",
        "brain.diagnostic_offset",
    ] {
        assert!(
            body.contains(required),
            "missing recurrent dynamics fragment: {required}"
        );
    }
    for activation_raw in ["0u", "1u", "2u", "3u"] {
        assert!(
            body.contains(activation_raw),
            "missing activation raw {activation_raw}"
        );
    }
}

#[test]
fn shaders_do_not_use_float_atomics_or_cpu_neural_authority() {
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
    ] {
        let lower = source.to_ascii_lowercase();
        for forbidden in [
            "atomic<f32>",
            "cpuneuralstate",
            "neuralprojectionschema",
            "cpu_shadow",
            "cpu fallback",
        ] {
            assert!(
                !lower.contains(forbidden),
                "forbidden shader authority: {forbidden}"
            );
        }
    }
    assert!(CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL.contains("atomicStore"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("atomicLoad"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("atomicStore"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("atomicAdd"));
}

#[test]
fn ping_pong_side_is_exact_for_two_three_and_four_microsteps() {
    for initial in [0, 1] {
        assert_eq!(
            GpuClosedLoopPipelines::final_activation_side(initial, 2).unwrap(),
            initial
        );
        assert_eq!(
            GpuClosedLoopPipelines::final_activation_side(initial, 3).unwrap(),
            initial ^ 1
        );
        assert_eq!(
            GpuClosedLoopPipelines::final_activation_side(initial, 4).unwrap(),
            initial
        );
    }
    assert!(GpuClosedLoopPipelines::final_activation_side(2, 3).is_err());
    assert!(GpuClosedLoopPipelines::final_activation_side(0, 1).is_err());
    assert!(GpuClosedLoopPipelines::final_activation_side(0, 5).is_err());
}

#[test]
fn production_pipeline_surface_has_no_decode_select_or_cpu_neural_types() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/closed_loop_pipeline.rs"
    ))
    .expect("Task 5 production pipeline source");
    for forbidden in [
        "CpuNeuralState",
        "NeuralProjectionSchema",
        "pub fn decode",
        "pub fn select",
        "pub async fn decode",
        "pub async fn select",
        "pub fn encode(",
        "pub fn dispatch_microsteps(",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden Task 5 API: {forbidden}"
        );
    }
    for required in [
        "pub struct GpuClosedLoopPipelines",
        "pub fn submit_encode_and_microsteps",
        "Task 6",
        "diagnostic lane 3",
        "closed-loop-clear-diagnostics-pass",
    ] {
        assert!(
            source.contains(required),
            "missing production API: {required}"
        );
    }
    let manifest = GpuClassBucketBuffers::neural_binding_manifest();
    assert_eq!(manifest[0].role, GpuClassBucketBufferRole::BrainSlots);
    assert_eq!(
        manifest[6].role,
        GpuClassBucketBufferRole::MutableStateWords
    );
    assert_eq!(
        manifest.map(|entry| entry.minimum_binding_size_bytes),
        [144, 32, 4, 4, 4, 4, 4]
    );
    let submit = &source[source.find("pub fn submit_encode_and_microsteps").unwrap()..];
    let signature_end = submit.find(") -> Result").unwrap();
    let signature = &submit[..signature_end];
    assert!(!signature.contains("microsteps: u32"));
    assert!(!signature.contains("max_neurons: u32"));
}
