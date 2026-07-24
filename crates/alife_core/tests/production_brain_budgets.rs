//! Contract tests for the single production capacity and compiled-budget authority.

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
    RouteBudgetReceipt, SensorProfile, Tick,
};
use serde_json::Value;

fn compile_populated_fixture(
    capacity: BrainCapacityClass,
    seed: u64,
) -> alife_core::BrainPhenotype {
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

#[test]
fn production_capacity_ids_have_exact_logical_ceilings() {
    let rows = [
        (
            BrainCapacityClass::n512(),
            512,
            8_192,
            64,
            6_144,
            1_024,
            1_024,
        ),
        (
            BrainCapacityClass::n1024(),
            1_024,
            16_384,
            128,
            12_288,
            2_048,
            2_048,
        ),
        (
            BrainCapacityClass::n2048(),
            2_048,
            32_768,
            192,
            24_576,
            4_096,
            4_096,
        ),
    ];
    for (capacity, neurons, synapses, tiles, recurrent, action_decoder, memory_decoder) in rows {
        let execution = capacity.execution();
        assert_eq!(execution.max_neurons(), neurons);
        assert_eq!(execution.max_total_synapses(), synapses);
        assert_eq!(execution.max_recurrent_synapses(), recurrent);
        assert_eq!(execution.max_action_decoder_synapses(), action_decoder);
        assert_eq!(execution.max_memory_decoder_synapses(), memory_decoder);
        assert_eq!(execution.max_active_tiles(), tiles);
        assert_eq!(execution.max_candidates(), 32);
        assert_eq!(execution.max_object_slots(), 16);
        assert_eq!(execution.max_decoder_input_lanes(), 64);
        assert_eq!(execution.max_compact_readback_bytes(), 64);
        assert_eq!(execution.microstep_range(), (2, 4));
        capacity.validate_contract().unwrap();
    }
}

#[test]
fn compiled_route_and_global_receipts_cover_every_payload_once() {
    for capacity in BrainCapacityClass::production_classes() {
        let phenotype = compile_populated_fixture(capacity, 4_404);
        let budgets = phenotype.budgets();
        budgets.validate_against(&capacity).unwrap();
        assert_eq!(budgets.routes.len(), phenotype.projections().len());
        assert_eq!(
            budgets.sum_route_synapses().unwrap(),
            phenotype.synapses().len() as u32
        );
        assert!(budgets.global.within(capacity.execution()));
        assert!(budgets
            .routes
            .iter()
            .all(RouteBudgetReceipt::within_ceiling));
    }
}

#[test]
fn execution_abi_rejects_every_independent_limit_violation() {
    for capacity in BrainCapacityClass::production_classes() {
        let canonical = serde_json::to_value(capacity).unwrap();
        let paths = numeric_leaf_paths(&canonical);
        assert!(paths.len() >= 40, "capacity ABI unexpectedly lost fields");
        for path in &paths {
            let mut forged = canonical.clone();
            increment_numeric_leaf(&mut forged, path);
            assert!(
                serde_json::from_value::<BrainCapacityClass>(forged).is_err(),
                "accepted forged capacity field {}",
                path.join(".")
            );
        }

        let execution = canonical["execution"].as_object().unwrap();
        for field in execution.keys() {
            let mut omitted = canonical.clone();
            omitted["execution"].as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<BrainCapacityClass>(omitted).is_err(),
                "accepted capacity with missing execution field {field}"
            );
        }
    }
}

#[test]
fn production_capacity_source_contains_no_tier_dispatch_or_byte_guess() {
    let source = include_str!("../src/phenotype/capacity.rs");
    let production = source.split("impl BrainCapacityClass").nth(1).unwrap();
    assert!(!production.contains("production_for_tier"));
    assert!(!production.contains("BrainScaleTier"));
    assert!(!production.contains("max_gpu_bytes"));
}

fn numeric_leaf_paths(value: &Value) -> Vec<Vec<String>> {
    fn visit(value: &Value, path: &mut Vec<String>, output: &mut Vec<Vec<String>>) {
        match value {
            Value::Number(_) => output.push(path.clone()),
            Value::Object(fields) => {
                for (name, child) in fields {
                    path.push(name.clone());
                    visit(child, path, output);
                    path.pop();
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    path.push(index.to_string());
                    visit(child, path, output);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    let mut output = Vec::new();
    visit(value, &mut Vec::new(), &mut output);
    output
}

fn increment_numeric_leaf(value: &mut Value, path: &[String]) {
    let mut cursor = value;
    for component in &path[..path.len() - 1] {
        cursor = if let Ok(index) = component.parse::<usize>() {
            &mut cursor.as_array_mut().unwrap()[index]
        } else {
            cursor.as_object_mut().unwrap().get_mut(component).unwrap()
        };
    }
    let leaf = &path[path.len() - 1];
    let number = if let Ok(index) = leaf.parse::<usize>() {
        &mut cursor.as_array_mut().unwrap()[index]
    } else {
        cursor.as_object_mut().unwrap().get_mut(leaf).unwrap()
    };
    let current = number.as_u64().unwrap();
    *number = Value::from(current.saturating_add(1));
}
