use super::native::*;
use super::qualified_store::*;
use super::scalar_reference::{self as scalar, Recipe, Role, SyntheticEvidence};
use super::sealed_fixture::{fixture, Fixture};
use alife_core::*;

const TOLERANCE: f32 = 2.0e-7;
fn evidence(delta: [f32; N], confidence: f32) -> SyntheticEvidence {
    SyntheticEvidence::MeasuredFixture {
        delta,
        confidence,
        age_ticks: 1,
        similarity: Some(0.99988),
        arithmetic_supported: true,
    }
}
fn state() -> [f32; N] {
    [0.25, 0.2, 0.1, 0.0, 0.15, 0.45, 0.75, 0.0, 0.1, 0.0, 0.0]
}
fn close(a: f32, b: f32) -> f32 {
    let error = (a - b).abs();
    assert!(
        error <= TOLERANCE,
        "native={a:?}, scalar={b:?}, error={error:?}"
    );
    error
}
fn compare(circuit: &Circuit, current: [f32; N], evidence: SyntheticEvidence) -> (Output, f32) {
    let output = circuit.run(current, evidence).unwrap();
    let reference = scalar::evaluate(circuit.recipe, current, evidence).unwrap();
    let mut error: f32 = 0.0;
    for (lane, &current_reading) in current.iter().enumerate() {
        error = error.max(close(
            output.successor[lane],
            reference.local_successor[lane],
        ));
        error = error.max(close(
            output.association[lane],
            reference.signed_association[lane],
        ));
        let pressure = |x: f32| match circuit.recipe.roles[lane] {
            Role::Burden | Role::TypedBoredomFixture => x,
            Role::Availability { target } => (target - x).max(0.0),
            Role::Informational => 0.0,
        };
        for (index, raw) in [
            (lane, current_reading),
            (N + lane, reference.local_successor[lane]),
        ] {
            error = error.max(close(output.pressure[index], pressure(raw)));
            error = error.max(close(
                output.burden[index],
                circuit.recipe.burden(pressure(raw)),
            ));
        }
    }
    error = error.max(close(
        output.diagnostic_supported,
        reference.readout_before_confidence,
    ));
    error = error.max(close(
        output.diagnostic_confidence_gated,
        reference.readout_after_confidence,
    ));
    assert_eq!(output.known, reference.known);
    assert_eq!(output.supported, reference.arithmetic_supported);
    assert_eq!(output.confidence, reference.confidence);
    assert_eq!(output.current, reference.current);
    assert_eq!(output.delta, reference.raw_delta);
    assert_eq!(output.age, reference.age_ticks);
    assert_eq!(output.similarity, reference.similarity);
    assert_eq!(output.readout_clipped, reference.readout_clipped);
    assert!(!reference.admissible_for_exposure);
    assert_eq!(output.work.projections, 5);
    assert_eq!(output.work.active_synapses, 112);
    assert_eq!(output.work.finalizations, 6);
    assert_eq!(output.work.activation_cells, 3072);
    assert_eq!(output.work.dendritic.branches_evaluated, 88);
    assert_eq!(output.work.dendritic.inputs_evaluated, 88);
    assert_eq!(output.work.dendritic.work_units, 176);
    (output, error)
}

#[test]
fn fourteen_exact_store_to_native_evaluations() {
    // Synthetic sealed contract fixtures, not world intervals or policy selections.
    let rows = [
        ("measured-relief", 0.15, -0.007_499_993, true, 0.15, true),
        ("measured-zero-at-zero", 0.0, 0.0, true, 0.0, true),
        ("bounded-near-zero", 0.001, -0.001, true, 0.001, true),
        ("emergent-harm", 0.0, 0.007_499_993, true, 0.0, true),
        ("measured-zero-at-baseline", 0.15, 0.0, true, 0.15, true),
        ("unmeasured-unknown", 0.15, 0.0, false, 0.15, false),
        ("exact-context-miss", 0.15, -0.007_499_993, true, 0.0, false),
    ];
    let mut count = 0;
    let mut maximum_error: f32 = 0.0;
    for (label, source_loneliness, delta, measured, query_loneliness, expected_known) in rows {
        let mut deltas = [0.0; N];
        deltas[4] = delta;
        let source = fixture(Fixture {
            loneliness: source_loneliness,
            deltas,
            measured,
            ..Fixture::default()
        });
        source.validate_contract().unwrap();
        assert!(source.selected_bundle().is_none());
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&source).unwrap();
        // Later frame, identical context except the deliberate exact-miss row.
        // It is NOT an evolved next-world frame or approximate retrieval.
        let current = fixture(Fixture {
            sequence: 2,
            tick: 12,
            loneliness: query_loneliness,
            measured: false,
            ..Fixture::default()
        });
        let frame = current.pre_action().perception();
        frame.validate_contract().unwrap();
        assert_eq!(frame.candidates().len(), 1);
        assert_eq!(frame.candidates()[0].family, CandidateActionFamily::Inspect);
        let query = NeedConsequenceQueryV1::from_frame(frame, &frame.candidates()[0]).unwrap();
        let recalled = store.recall(&query, frame.tick(), 1, 4).unwrap();
        let evidence = match recalled.evidence {
            NeedConsequenceEvidenceV1::Unknown => SyntheticEvidence::Unknown,
            NeedConsequenceEvidenceV1::Measured(records) => {
                assert_eq!(records.len(), 1);
                let record = &records[0];
                assert_eq!(record.source_sequence_id, source.header().sequence_id);
                assert_eq!(
                    record.source_frame_digest,
                    source.pre_action().perception().frame_digest().0
                );
                assert_eq!(record.joint_command_count, 1);
                assert!(record.source_bundle_digest.is_none());
                assert_eq!(
                    record.attribution,
                    NeedConsequenceAttributionV1::JointIntervalAssociation
                );
                assert_eq!(record.before_tick, Tick(10));
                assert_eq!(record.after_tick, Tick(11));
                let before = source
                    .outcome()
                    .measured_physiology
                    .as_ref()
                    .unwrap()
                    .before
                    .homeostasis
                    .drives
                    .to_array();
                assert_eq!(before, frame.homeostasis().drives.to_array());
                assert_eq!(record.confidence.raw(), 0.8);
                evidence(record.drive_deltas, record.confidence.raw())
            }
        };
        let mut matched = None;
        for gain in [0.0, 0.35] {
            let mut gene = Gene::reference();
            gene.recipe.gain = gain;
            let circuit = Circuit::compile(gene).unwrap();
            let (output, error) =
                compare(&circuit, frame.homeostasis().drives.to_array(), evidence);
            maximum_error = maximum_error.max(error);
            assert_eq!(output.known, expected_known);
            if let Some((previous_association, previous_work)) = &matched {
                assert_eq!(&output.association, previous_association);
                assert_eq!(&output.work, previous_work);
            }
            println!("row={label} gain={gain} known={} supported={} confidence={} current={} delta={} successor={} p={} q={} Bp={} Bq={} u={} native_readout={} diagnostic_confidence_gated={} error={} work={:?}",
                output.known, output.supported, output.confidence, output.current[4], output.delta[4],
                output.successor[4], output.pressure[4], output.pressure[N+4], output.burden[4],
                output.burden[N+4], output.association[4], output.native_readout,
                output.diagnostic_confidence_gated, error, output.work);
            matched = Some((output.association, output.work));
            count += 1;
        }
    }
    assert_eq!(count, 14);
    println!(
        "fourteen_evaluations={count} maximum_absolute_error={maximum_error} tolerance={TOLERANCE}"
    );
}

#[test]
fn all_eleven_roles_both_signs_and_pressure_boundaries() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    let mut maximum: f32 = 0.0;
    let mut count = 0;
    for lane in 0..N {
        for raw in [0.0, 0.001, 0.15, 0.5, 0.75, 1.0] {
            for delta in [-1.0, -0.05, 0.0, 0.05, 1.0] {
                let mut current = state();
                current[lane] = raw;
                let mut deltas = [0.0; N];
                deltas[lane] = delta;
                let (out, error) = compare(&circuit, current, evidence(deltas, 1.0));
                maximum = maximum.max(error);
                count += 1;
                match circuit.recipe.roles[lane] {
                    Role::Burden => {
                        if delta < 0.0 {
                            assert!(out.association[lane] >= 0.0);
                        }
                        if delta > 0.0 {
                            assert!(out.association[lane] <= 0.0);
                        }
                        if raw == 0.0 && delta < 0.0 {
                            assert_eq!(out.association[lane], 0.0);
                        }
                        assert!(out.association[lane] <= circuit.recipe.burden(raw) + TOLERANCE);
                    }
                    Role::Availability { .. } => {
                        if delta > 0.0 {
                            assert!(out.association[lane] >= 0.0);
                        }
                        if delta < 0.0 {
                            assert!(out.association[lane] <= 0.0);
                        }
                        if raw >= 0.75 && delta >= 0.0 {
                            assert_eq!(out.association[lane], 0.0);
                        }
                        assert!(
                            out.association[lane]
                                <= circuit.recipe.burden((0.75 - raw).max(0.0)) + TOLERANCE
                        );
                    }
                    Role::Informational => assert_eq!(out.association[lane], 0.0),
                    Role::TypedBoredomFixture => unreachable!(),
                }
                if delta == 0.0 {
                    assert_eq!(out.association, [0.0; N]);
                }
                assert!(out.native_readout.abs() <= 0.35);
                assert_eq!(
                    out.clipped_successor[lane],
                    !(0.0..=1.0).contains(&(raw + delta))
                );
            }
        }
    }
    // Above-target ATP loss is neutral until it crosses the actual target.
    for (available, change, adverse) in [(0.9, -0.05, false), (0.76, -0.05, true)] {
        let mut current = state();
        current[6] = available;
        let mut delta = [0.0; N];
        delta[6] = change;
        let out = compare(&circuit, current, evidence(delta, 1.0)).0;
        if adverse {
            assert!(out.association[6] < 0.0);
        } else {
            assert_eq!(out.association[6], 0.0);
        }
    }
    assert_eq!(count, 330);
    println!("eleven_lane_boundary_evaluations={count} maximum_absolute_error={maximum}");
}

#[test]
fn evidence_status_confidence_and_similarity_are_separate() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    let (unknown, _) = compare(&circuit, state(), SyntheticEvidence::Unknown);
    let (zero, _) = compare(&circuit, state(), evidence([0.0; N], 1.0));
    assert!(!unknown.known);
    assert!(zero.known);
    assert_eq!(unknown.association, zero.association);
    let mut delta = [0.0; N];
    delta[4] = -0.05;
    let (full, _) = compare(&circuit, state(), evidence(delta, 1.0));
    let (half, _) = compare(&circuit, state(), evidence(delta, 0.5));
    let (no_confidence, _) = compare(&circuit, state(), evidence(delta, 0.0));
    assert!(no_confidence.known && no_confidence.supported);
    assert_eq!(half.native_readout, full.native_readout);
    assert_eq!(half.diagnostic_confidence_gated, full.native_readout * 0.5);
    assert_eq!(no_confidence.diagnostic_confidence_gated, 0.0);
    for similarity in [None, Some(0.0), Some(0.975), Some(1.0)] {
        let (out, _) = compare(
            &circuit,
            state(),
            SyntheticEvidence::MeasuredFixture {
                delta,
                confidence: 0.5,
                age_ticks: 1,
                similarity,
                arithmetic_supported: true,
            },
        );
        assert_eq!(
            out.diagnostic_confidence_gated,
            half.diagnostic_confidence_gated
        );
        assert_eq!(out.delta, delta);
    }
    let (unsupported, _) = compare(
        &circuit,
        state(),
        SyntheticEvidence::MeasuredFixture {
            delta,
            confidence: 1.0,
            age_ticks: 1,
            similarity: Some(1.0),
            arithmetic_supported: false,
        },
    );
    assert!(unsupported.known && !unsupported.supported);
    assert_eq!(unsupported.delta, delta);
    assert_eq!(unsupported.diagnostic_supported, 0.0);
    assert_eq!(unsupported.diagnostic_confidence_gated, 0.0);
    assert_eq!(unsupported.work, full.work);
    // Unknown executes the same graph; gated branch count is a data diagnostic.
    assert_eq!(unknown.work.active_synapses, full.work.active_synapses);
    assert_eq!(
        unknown.work.dendritic.work_units,
        full.work.dendritic.work_units
    );
}

#[test]
fn shared_gene_identity_count_and_recipe_binding() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    assert_eq!(circuit.genes.len(), 22); // 4 knots+4 basis+11 magnitudes+1 target+gain+ceiling.
    assert_eq!(circuit.branch_ids.len(), 88);
    for lane in 0..N {
        for k in 0..K {
            let current = &circuit.branch_ids[lane * K + k];
            let successor = &circuit.branch_ids[N * K + lane * K + k];
            assert_eq!(current.side, Side::Current);
            assert_eq!(successor.side, Side::Successor);
            assert_eq!(current.lane, successor.lane);
            assert_eq!(current.role, successor.role);
            assert_eq!(current.threshold_gene, successor.threshold_gene);
            assert_eq!(current.weight_gene, successor.weight_gene);
            assert_eq!(current.weight_gene, GeneId::Basis(k as u8));
            let a = &circuit.branches.branches()[lane * K + k];
            let b = &circuit.branches.branches()[N * K + lane * K + k];
            assert_eq!(a.threshold.to_bits(), b.threshold.to_bits());
            assert_eq!(a.output_gain.to_bits(), b.output_gain.to_bits());
            assert_eq!(
                a.output_gain.to_bits(),
                circuit.genes[&current.weight_gene].to_bits()
            );
            assert_eq!(a.inputs[0].weight, 1.0);
            assert_eq!(b.inputs[0].weight, 1.0);
        }
    }
    assert_eq!(
        circuit.identity,
        Circuit::compile(Gene::reference()).unwrap().identity
    );
    let mut altered = Gene::reference();
    altered.recipe.gain = 0.0;
    assert_ne!(
        circuit.identity,
        Circuit::compile(altered).unwrap().identity
    );
    let mut altered = Gene::reference();
    altered.recipe.roles[6] = Role::Availability { target: 1.0 };
    assert_ne!(
        circuit.identity,
        Circuit::compile(altered).unwrap().identity
    );
    let typed = Circuit::compile(Gene::typed_boredom()).unwrap();
    assert_ne!(circuit.identity, typed.identity);
    let mut current = state();
    current[9] = 0.2;
    let mut delta = [0.0; N];
    delta[9] = -0.05;
    assert!(
        compare(&typed, current, evidence(delta, 1.0))
            .0
            .native_readout
            > 0.0
    );
    let mut unbound = Gene::typed_boredom();
    unbound.typed_boredom_binding = None;
    assert!(Circuit::compile(unbound).is_err());
    let mut wrong = Gene::typed_boredom();
    wrong.typed_boredom_binding = Some("unverified extension");
    assert!(Circuit::compile(wrong).is_err());
    println!("recipe_identity={:?} unique_numeric_genes={} dendritic_occurrences={} fixed_sparse_edges=112", circuit.identity, circuit.genes.len(), circuit.branch_ids.len());
}

#[test]
fn opposing_drives_cancel_and_current_pressure_changes_preference() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    let mut current = state();
    current[4] = 0.75;
    current[1] = 0.5;
    let mut delta = [0.0; N];
    delta[4] = -0.25;
    delta[1] = 0.25;
    let (cancel, _) = compare(&circuit, current, evidence(delta, 1.0));
    assert!(cancel.association[4] > 0.0 && cancel.association[1] < 0.0);
    assert_eq!(cancel.association[4], -cancel.association[1]);
    assert_eq!(cancel.native_readout, 0.0);
    let mut work = [0.0; N];
    work[4] = -0.05;
    work[1] = 0.03;
    work[6] = -0.03;
    let mut recovery = [0.0; N];
    recovery[1] = -0.04;
    recovery[6] = 0.04;
    let mut rested = state();
    rested[4] = 0.8;
    rested[1] = 0.01;
    rested[6] = 0.8;
    let mut exhausted = state();
    exhausted[4] = 0.01;
    exhausted[1] = 0.8;
    exhausted[6] = 0.2;
    let score = |x, d| compare(&circuit, x, evidence(d, 1.0)).0.native_readout;
    assert!(score(rested, work) > score(rested, recovery));
    assert!(score(exhausted, recovery) > score(exhausted, work));
    assert_eq!(score(rested, [0.0; N]), 0.0);
    // Neither row has an action label. Recovery is hypothetical, not a V1 Rest memory.
}

#[test]
fn conditioning_permuted_need_labels_and_candidate_rows_do_not_collapse() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    let mut delta = [0.0; N];
    delta[4] = -0.05;
    let mut low = state();
    low[4] = 0.0;
    let mut high = state();
    high[4] = 0.8;
    let (low_out, _) = compare(&circuit, low, evidence(delta, 1.0));
    let (high_out, _) = compare(&circuit, high, evidence(delta, 1.0));
    assert_eq!(low_out.native_readout, 0.0);
    assert!(high_out.native_readout > 0.0);
    // Fixed-reference control improperly erases current pressure; label it explicitly.
    let fixed_reference = state();
    let fixed_low = compare(&circuit, fixed_reference, evidence(delta, 1.0)).0;
    let fixed_high = compare(&circuit, fixed_reference, evidence(delta, 1.0)).0;
    assert_eq!(fixed_low.native_readout, fixed_high.native_readout);
    assert!(fixed_low.native_readout > low_out.native_readout);
    let mut wrong_need = delta;
    wrong_need.swap(4, 1);
    high[1] = 0.0;
    assert_eq!(
        compare(&circuit, high, evidence(wrong_need, 1.0))
            .0
            .native_readout,
        0.0
    );
    let mut second_target = delta;
    second_target[4] = 0.05;
    let target_one_alone = compare(&circuit, high, evidence(delta, 1.0)).0;
    let target_two = compare(&circuit, high, evidence(second_target, 1.0)).0;
    let target_one_again = compare(&circuit, high, evidence(delta, 1.0)).0;
    assert_eq!(
        target_one_alone.native_readout,
        target_one_again.native_readout
    );
    assert!(target_one_alone.native_readout > 0.0 && target_two.native_readout < 0.0);
    // No candidate count, family pooling, selector or target semantic label in Circuit.
}

#[test]
fn invalid_inputs_and_nonmonotone_genes_fail_closed() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.001, 1.001] {
        for lane in 0..N {
            let mut current = state();
            current[lane] = value;
            assert!(circuit.run(current, SyntheticEvidence::Unknown).is_err());
        }
        assert!(circuit.run(state(), evidence([0.0; N], value)).is_err());
        assert!(circuit
            .run(
                state(),
                SyntheticEvidence::MeasuredFixture {
                    delta: [0.0; N],
                    confidence: 1.0,
                    age_ticks: 1,
                    similarity: Some(value),
                    arithmetic_supported: true
                }
            )
            .is_err());
    }
    for value in [f32::NAN, f32::INFINITY, -1.001, 1.001] {
        for lane in 0..N {
            let mut delta = [0.0; N];
            delta[lane] = value;
            assert!(circuit.run(state(), evidence(delta, 1.0)).is_err());
        }
    }
    let invalid: [fn(&mut Recipe); 15] = [
        |r| {
            r.roles[9] = Role::Burden;
            r.magnitudes[9] = 0.08;
        },
        |r| r.roles[6] = Role::Burden,
        |r| {
            r.roles[8] = Role::Burden;
            r.magnitudes[8] = 0.08;
        },
        |r| r.basis[0] = -0.1,
        |r| r.basis = [0.3; K],
        |r| r.knots[1] = 0.0,
        |r| r.knots[0] = f32::NAN,
        |r| r.gain = 0.351,
        |r| r.gain = -0.1,
        |r| r.ceiling = 0.351,
        |r| r.roles[6] = Role::Availability { target: 0.0 },
        |r| r.roles[6] = Role::Availability { target: 1.001 },
        |r| r.magnitudes[5] = 0.1,
        |r| r.magnitudes[0] = -0.1,
        |r| {
            r.magnitudes[0] = 1.0;
            r.magnitudes[1] = 1.0;
        },
    ];
    for edit in invalid {
        let mut gene = Gene::reference();
        edit(&mut gene.recipe);
        assert!(Circuit::compile(gene).is_err());
    }
}

#[test]
fn fresh_scratch_never_accepts_or_resets_acquired_offsets() {
    let circuit = Circuit::compile(Gene::reference()).unwrap();
    let before = serde_json::to_vec(&(&circuit.projections, &circuit.branches)).unwrap();
    for schema in &circuit.projections {
        for tile in &schema.projections[0].tiles {
            let SparseTilePayload::Coo(coo) = &tile.payload else {
                panic!("COO fixture");
            };
            for entry in &coo.entries {
                assert_eq!(entry.weights.lifetime_consolidated, 0.0);
                assert_eq!(entry.weights.alpha, 0.0);
                assert_eq!(entry.weights.h_operational, 0.0);
                assert_eq!(entry.weights.h_shadow, 0.0);
            }
        }
    }
    // Synthetic acquired sentinel is never supplied to Circuit; check no alias/mutation.
    let mut sentinel =
        CpuNeuralState::for_brain_class(&BrainClassSpec::for_tier(BrainScaleTier::Nano512))
            .unwrap();
    sentinel.activations[7] = 0.314;
    sentinel.previous_activations[19] = -0.271;
    sentinel.accumulators[1] = 0.618;
    let sentinel_before = serde_json::to_vec(&sentinel).unwrap();
    let mut delta = [0.0; N];
    delta[4] = -0.05;
    let first = compare(&circuit, state(), evidence(delta, 1.0)).0;
    compare(&circuit, [1.0; N], evidence([1.0; N], 1.0));
    let again = compare(&circuit, state(), evidence(delta, 1.0)).0;
    assert_eq!(first.association, again.association);
    assert_eq!(first.native_readout, again.native_readout);
    assert_eq!(
        before,
        serde_json::to_vec(&(&circuit.projections, &circuit.branches)).unwrap()
    );
    assert_eq!(sentinel_before, serde_json::to_vec(&sentinel).unwrap());
}

#[test]
fn native_readout_ceiling_and_shared_basis_variants_match_oracle() {
    let mut gene = Gene::reference();
    gene.recipe.ceiling = 0.001;
    let circuit = Circuit::compile(gene).unwrap();
    let mut current = [1.0; N];
    current[6] = 0.0;
    let mut delta = [-1.0; N];
    delta[6] = 1.0;
    let (out, _) = compare(&circuit, current, evidence(delta, 1.0));
    assert!(out.readout_clipped);
    assert_eq!(out.native_readout, 0.001);
    for basis in [[0.0; K], [0.25; K], [0.0, 0.0, 0.0, 1.0]] {
        let mut gene = Gene::reference();
        gene.recipe.basis = basis;
        let circuit = Circuit::compile(gene).unwrap();
        for raw in [0.0, 0.001, 0.15, 0.5, 1.0] {
            let mut x = state();
            x[4] = raw;
            let mut d = [0.0; N];
            d[4] = -0.05;
            let (out, _) = compare(&circuit, x, evidence(d, 1.0));
            assert!(out.association[4] >= 0.0);
            if raw == 0.0 {
                assert_eq!(out.native_readout, 0.0);
            }
        }
    }
}
