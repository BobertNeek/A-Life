//! WGSL sensory encoding, recurrent dynamics and layout checks.

#[cfg(feature = "gpu-tests")]
#[path = "closed_loop_wgsl/hardware.rs"]
mod hardware;
#[cfg(feature = "gpu-tests")]
mod support;

use std::collections::BTreeMap;

use alife_core::SchemaVersions;
use alife_gpu_backend::{
    validate_dispatch_dimensions, GpuBufferAccess, GpuClassBucketBufferRole, GpuClassBucketBuffers,
    GpuClosedLoopError, GpuClosedLoopPipelines, CLOSED_LOOP_ABI_WGSL,
    CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL, CLOSED_LOOP_DECODE_WGSL, CLOSED_LOOP_ELIGIBILITY_WGSL,
    CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_MEMORY_CONTEXT_WGSL, CLOSED_LOOP_PLASTICITY_WGSL,
    CLOSED_LOOP_RECURRENT_WGSL, GPU_ACTIVE_DISPATCH_ROW_WORDS, GPU_ACTIVE_SIDE_DIAGNOSTIC_LANE,
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

fn assert_shader_constant(source: &str, name: &str, expected: u32) {
    let module = validated_module(source);
    let (_, value) = module
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))
        .expect("required ABI constant");
    assert!(
        matches!(module.global_expressions[value.init], Expression::Literal(naga::Literal::U32(actual)) if actual == expected)
    );
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
        let expected_entry_count = if entry == "recurrent_microstep" {
            let clear = module
                .entry_points
                .iter()
                .find(|point| point.name == "clear_v11_work")
                .expect("recurrent shader must clear v1.1 work counters before microsteps");
            assert_eq!(clear.stage, ShaderStage::Compute);
            assert_eq!(clear.workgroup_size, [1, 1, 1]);
            2
        } else {
            1
        };
        assert_eq!(
            module.entry_points.len(),
            expected_entry_count,
            "unexpected entries for {entry}"
        );
        let point = module
            .entry_points
            .iter()
            .find(|point| point.name == entry)
            .expect("required compute entry");
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
    for count in 2..=4 {
        assert!(GpuClosedLoopPipelines::validate_microstep_count(count).is_ok());
    }
    assert!(GpuClosedLoopPipelines::validate_microstep_count(1).is_err());
    assert!(GpuClosedLoopPipelines::validate_microstep_count(5).is_err());
}

#[test]
fn learning_state_schema_matches_the_wgsl_validation_contract() {
    assert_shader_constant(
        CLOSED_LOOP_ABI_WGSL,
        "GPU_LEARNING_SCHEMA_VERSION",
        u32::from(SchemaVersions::CURRENT.learning.raw()),
    );
}

#[test]
fn naga_reflection_matches_the_exact_seven_heap_bindings_and_access_modes() {
    let expected_manifest = GpuClassBucketBuffers::neural_binding_manifest();
    assert_eq!(expected_manifest.len(), 7);
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_DECODE_WGSL,
        CLOSED_LOOP_ELIGIBILITY_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
        CLOSED_LOOP_PLASTICITY_WGSL,
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
        CLOSED_LOOP_DECODE_WGSL,
        CLOSED_LOOP_ELIGIBILITY_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_PLASTICITY_WGSL,
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
    assert_eq!(GPU_ACTIVE_DISPATCH_ROW_WORDS, 332);
    for source in [
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_DECODE_WGSL,
        CLOSED_LOOP_ELIGIBILITY_WGSL,
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
    ] {
        assert_shader_constant(
            source,
            "ACTIVE_DISPATCH_ROW_WORDS",
            GPU_ACTIVE_DISPATCH_ROW_WORDS as u32,
        );
    }
}

#[test]
fn encode_has_reachable_dispatch_input_loads_and_mutable_state_writes() {
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
}

#[test]
fn recurrent_shader_has_reachable_graph_weight_and_state_access() {
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
fn neural_heap_binding_manifest_has_expected_roles_and_minimum_sizes() {
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
}
