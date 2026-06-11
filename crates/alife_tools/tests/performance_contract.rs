use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
struct BrainClass {
    neurons: u64,
    active_synapses: u64,
    active_tiles: u64,
}

#[derive(Debug, Clone)]
struct Component {
    bytes_per_entry: u64,
    dense_formula: String,
    sparse_formula: String,
    sharing: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools should live under crates/")
        .to_path_buf()
}

fn ledger_lines(markdown: &str) -> Vec<&str> {
    let start = markdown
        .find("PERFORMANCE_LEDGER_V1_BEGIN")
        .expect("performance ledger start marker is required");
    let end = markdown
        .find("PERFORMANCE_LEDGER_V1_END")
        .expect("performance ledger end marker is required");
    markdown[start..end]
        .lines()
        .filter(|line| {
            !line.trim().is_empty()
                && !line.starts_with("PERFORMANCE_LEDGER_V1_BEGIN")
                && !line.starts_with("```")
        })
        .collect()
}

fn formula_entries(formula: &str, class: BrainClass) -> u64 {
    match formula {
        "dense_synapses" => class.neurons * class.neurons,
        "active_synapses" => class.active_synapses,
        "active_tiles" => class.active_tiles,
        "neurons" => class.neurons,
        "dense_tiles" => {
            let tiles_per_edge = class.neurons / 16;
            tiles_per_edge * tiles_per_edge
        }
        "double_buffer" => 2,
        unknown => panic!("unknown ledger formula {unknown}"),
    }
}

#[test]
fn performance_contract_ledger_totals_are_formula_derived() {
    let path = workspace_root().join("docs/architecture/P04_5_performance_contract.md");
    let markdown = fs::read_to_string(path).expect("performance contract should be readable");

    let mut classes = BTreeMap::new();
    let mut populations = BTreeSet::new();
    let mut components = Vec::new();
    let mut declared_totals = BTreeMap::new();
    let mut declared_population_totals = BTreeMap::new();

    for line in ledger_lines(&markdown) {
        let parts: Vec<_> = line.split(',').collect();
        match parts.as_slice() {
            ["CLASS", name, neurons, active_synapses, active_tiles] => {
                classes.insert(
                    (*name).to_string(),
                    BrainClass {
                        neurons: neurons.parse().expect("neurons must be numeric"),
                        active_synapses: active_synapses
                            .parse()
                            .expect("active synapses must be numeric"),
                        active_tiles: active_tiles.parse().expect("active tiles must be numeric"),
                    },
                );
            }
            ["POP", population] => {
                populations.insert(
                    population
                        .parse::<u64>()
                        .expect("population must be numeric"),
                );
            }
            ["COMPONENT", name, bytes, dense_formula, sparse_formula, sharing] => {
                assert!(!name.is_empty(), "component name must be explicit");
                components.push(Component {
                    bytes_per_entry: bytes.parse().expect("bytes must be numeric"),
                    dense_formula: (*dense_formula).to_string(),
                    sparse_formula: (*sparse_formula).to_string(),
                    sharing: (*sharing).to_string(),
                });
            }
            ["TOTAL", class_name, dense, sparse_per_creature, shared] => {
                declared_totals.insert(
                    (*class_name).to_string(),
                    (
                        dense.parse::<u64>().expect("dense total must be numeric"),
                        sparse_per_creature
                            .parse::<u64>()
                            .expect("sparse total must be numeric"),
                        shared.parse::<u64>().expect("shared total must be numeric"),
                    ),
                );
            }
            ["POP_TOTAL", class_name, population, total] => {
                declared_population_totals.insert(
                    (
                        (*class_name).to_string(),
                        population
                            .parse::<u64>()
                            .expect("population total population must be numeric"),
                    ),
                    total
                        .parse::<u64>()
                        .expect("population total must be numeric"),
                );
            }
            _ => panic!("unrecognized ledger row: {line}"),
        }
    }

    assert_eq!(
        classes.keys().cloned().collect::<Vec<_>>(),
        ["Large4096", "Nano512", "Small1024", "Standard2048"],
        "ledger must cover the four required brain classes"
    );
    assert_eq!(
        populations.into_iter().collect::<Vec<_>>(),
        [1, 10, 50, 100, 250, 500],
        "ledger must cover required population counts"
    );
    assert_eq!(
        components.len(),
        11,
        "ledger must cover required memory components"
    );

    for (class_name, class) in &classes {
        let mut dense_total = 0;
        let mut sparse_per_creature = 0;
        let mut shared_template = 0;

        for component in &components {
            let dense_bytes = formula_entries(&component.dense_formula, *class)
                .checked_mul(component.bytes_per_entry)
                .expect("dense component bytes should not overflow");
            let sparse_bytes = formula_entries(&component.sparse_formula, *class)
                .checked_mul(component.bytes_per_entry)
                .expect("sparse component bytes should not overflow");

            dense_total += dense_bytes;
            match component.sharing.as_str() {
                "shared" => shared_template += sparse_bytes,
                "per_creature" => sparse_per_creature += sparse_bytes,
                unknown => panic!("unknown sharing policy {unknown}"),
            }
        }

        assert_eq!(
            declared_totals.get(class_name),
            Some(&(dense_total, sparse_per_creature, shared_template)),
            "declared totals for {class_name} must derive from component formulas"
        );

        for population in [1, 10, 50, 100, 250, 500] {
            let expected = shared_template + sparse_per_creature * population;
            assert_eq!(
                declared_population_totals.get(&(class_name.clone(), population)),
                Some(&expected),
                "population total for {class_name} x {population} must derive from sparse live plus shared template bytes"
            );
        }
    }
}
