//! Equivalence against the original independently scanning accounting function.

use alife_core::{BrainCapacityClass, ScaffoldContractError, SensorProfile};
use alife_gpu_backend::derive_executed_work;

use super::{decision_for, support, work_scan_reference};

#[test]
fn immutable_route_receipts_match_scan_for_changing_schedules() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        for (seed, maturation, profile) in [
            (47_101, 0.0, SensorProfile::GroundedObjectSlotsV1),
            (47_102, 1.0, SensorProfile::PrivilegedAffordanceV1),
        ] {
            let phenotype =
                support::phenotype_for_capacity_at_maturation(capacity, seed, maturation, profile);
            let routes = phenotype
                .projections()
                .iter()
                .map(|projection| projection.route_index())
                .collect::<Vec<_>>();
            let mut schedules = vec![routes.clone()];
            for route in &routes {
                schedules.push(vec![*route]);
                schedules.push(routes.iter().copied().filter(|id| id != route).collect());
            }
            schedules.push(routes.iter().copied().step_by(2).collect());
            schedules.push(routes.iter().copied().skip(1).step_by(2).collect());
            for enabled in schedules.iter().filter(|ids| !ids.is_empty()) {
                for microsteps in [1, 2, 3, 4, 7] {
                    for (candidates, memory) in [(0, 0), (3, 7), (u32::MAX, u32::MAX)] {
                        assert_eq!(
                            derive_executed_work(
                                &phenotype, microsteps, enabled, candidates, memory,
                            ),
                            work_scan_reference::derive_executed_work(
                                &phenotype, microsteps, enabled, candidates, memory,
                            ),
                            "class={} enabled={enabled:?} microsteps={microsteps}",
                            capacity.id().raw(),
                        );
                    }
                }
            }
            assert_eq!(
                derive_executed_work(&phenotype, u8::MAX, &routes, 0, 0),
                work_scan_reference::derive_executed_work(&phenotype, u8::MAX, &routes, 0, 0),
            );
        }
    }
}

#[test]
fn policy_selected_work_matches_scan() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            capacity,
            47_201,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        for (index, inputs) in [
            (0, 0, 0, 65_535),
            (2_000_000, 1, 32_768, 49_151),
            (4_000_000, 2, 49_152, 32_767),
            (8_000_000, 4, 58_982, 16_383),
        ]
        .into_iter()
        .enumerate()
        {
            let decision = decision_for(&phenotype, 3, 7, index as u64 + 1, inputs);
            for (candidate_count, memory_count) in [(0, 0), (3, 7), (32, 8)] {
                let actual = derive_executed_work(
                    &phenotype,
                    decision.microsteps,
                    &decision.enabled_route_ids,
                    candidate_count,
                    memory_count,
                )
                .unwrap();
                let expected = work_scan_reference::derive_executed_work(
                    &phenotype,
                    decision.microsteps,
                    &decision.enabled_route_ids,
                    candidate_count,
                    memory_count,
                )
                .unwrap();
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn malformed_schedules_keep_the_original_error() {
    let phenotype = support::n512_phenotype(47_301);
    let routes = phenotype
        .projections()
        .iter()
        .map(|projection| projection.route_index())
        .collect::<Vec<_>>();
    for (microsteps, enabled) in [
        (0, routes.clone()),
        (0, vec![]),
        (3, vec![]),
        (3, vec![routes[0], routes[0]]),
        (3, vec![routes[1], routes[0]]),
        (3, vec![u16::MAX]),
        (3, vec![routes[0], u16::MAX]),
    ] {
        let actual = derive_executed_work(&phenotype, microsteps, &enabled, u32::MAX, u32::MAX);
        assert_eq!(
            actual,
            work_scan_reference::derive_executed_work(
                &phenotype,
                microsteps,
                &enabled,
                u32::MAX,
                u32::MAX,
            ),
        );
        assert_eq!(
            actual,
            Err(ScaffoldContractError::BrainActivitySequenceMismatch)
        );
    }
}

#[test]
fn decoder_routes_do_not_become_recurrent_work() {
    let phenotype = support::n512_phenotype(47_401);
    let mut decoder_routes = 0;
    for receipt in phenotype
        .budgets()
        .routes
        .iter()
        .filter(|route| route.recurrent_synapses == 0)
    {
        assert!(receipt.action_decoder_synapses != 0 || receipt.memory_decoder_synapses != 0);
        let actual = derive_executed_work(&phenotype, 4, &[receipt.route_index], 3, 7).unwrap();
        assert_eq!(actual.synapse_ops, 0);
        assert_eq!(actual.decoder_candidate_ops, 3);
        assert_eq!(actual.memory_context_ops, 7);
        decoder_routes += 1;
    }
    assert!(decoder_routes >= 2);
}
