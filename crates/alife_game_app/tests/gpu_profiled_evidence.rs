#![cfg(feature = "gpu-runtime")]

use alife_core::TopologyCounts;
use alife_game_app::TopologyCapacityReceipt;

#[test]
fn topology_capacity_receipt_bounds_counts_and_bindings() {
    let capacity = TopologyCapacityReceipt {
        max_concepts: 4,
        max_edges: 5,
        max_simplexes: 6,
        max_unresolved_gaps: 2,
        max_bindings_per_kind: 3,
    };
    assert!(capacity.contains(
        TopologyCounts {
            concepts: 4,
            edges: 5,
            simplexes: 6,
            unresolved_gaps: 2,
        },
        3,
    ));
    assert!(!capacity.contains(
        TopologyCounts {
            concepts: 5,
            edges: 5,
            simplexes: 6,
            unresolved_gaps: 2,
        },
        3,
    ));
}
