//! Bounded ordinary-game food-choice probe for the opt-in inherited pain floor.

use super::*;
use alife_core::{
    CandidateActionFamily, CompiledSynapseKind, DecoderHeadKind, OutcomeCreditPacket,
};

#[derive(Debug, serde::Serialize)]
pub(super) struct ChoiceResult {
    cyan_nutritious: bool,
    world_ticks: u64,
    awake_opportunities: [u32; 2],
    late_both_available: u32,
    late_nutritious_meals: u32,
    late_harmful_meals: u32,
    all_nutritious_meals: u32,
    all_harmful_meals: u32,
    first_harmful_third_factor: Option<f32>,
    first_nutritious_third_factor: Option<f32>,
    harmful_positive_factors: u32,
    harmful_negative_factors: u32,
    harmful_zero_factors: u32,
    no_progress_polls: u32,
}

#[test]
#[ignore = "bounded manual preference probe; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn inherited_nociception_food_choice_probe() {
    let candidate = FoundationWeightAsset::decode_canonical(
        &std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        candidate.digest().bytes(),
        &[
            14, 148, 152, 50, 97, 32, 184, 207, 71, 123, 153, 74, 88, 1, 122, 247, 18, 136, 157,
            226, 80, 112, 127, 203, 223, 222, 92, 5, 5, 204, 232, 246,
        ]
    );
    let (phenotype, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&candidate).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/nociception-food-{}",
        std::process::id(),
    ));
    assert!(!root.exists(), "preserve earlier evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("neural-phenotype.json"),
        serde_json::to_vec_pretty(&phenotype).unwrap(),
    )
    .unwrap();
    let harmful_amber = run_food_life(
        &candidate,
        &phenotype,
        true,
        &root.join("cyan-nutritious"),
        None,
    );
    let nutritious_amber = run_food_life(
        &candidate,
        &phenotype,
        false,
        &root.join("amber-nutritious"),
        None,
    );
    std::fs::write(
        root.join("outcomes.json"),
        serde_json::to_vec_pretty(&[&harmful_amber, &nutritious_amber]).unwrap(),
    )
    .unwrap();
    println!(
        "nociception_choices: {harmful_amber:?}; {nutritious_amber:?}; evidence={}",
        root.display()
    );
    assert_choices(&harmful_amber, &nutritious_amber);
}

pub(super) fn assert_choices(harmful_amber: &ChoiceResult, nutritious_amber: &ChoiceResult) {
    // Sign is a physiological gate, separate from useful later choice.
    assert!(
        harmful_amber
            .first_harmful_third_factor
            .is_some_and(|v| v < 0.0),
        "actual first harmful meal did not produce negative decoder modulation"
    );
    assert!(
        nutritious_amber
            .first_nutritious_third_factor
            .is_some_and(|v| v > 0.0),
        "actual first nutritious meal did not produce positive decoder modulation"
    );
    for result in [harmful_amber, nutritious_amber] {
        assert!(
            result.late_both_available >= 8,
            "insufficient late awake choice opportunities: {result:?}"
        );
        assert!(
            result.late_nutritious_meals >= 3
                && result.late_nutritious_meals > result.late_harmful_meals,
            "no useful preference demonstrated in the bounded late choices: {result:?}"
        );
    }
}

pub(super) fn run_food_life(
    candidate: &FoundationWeightAsset,
    phenotype: &alife_core::BrainPhenotype,
    cyan_nutritious: bool,
    root: &Path,
    action_credit: Option<&alife_core::Nano512ActionCreditCandidateV2>,
) -> ChoiceResult {
    let (mut runtime, _) = super::founder_consequence_tests::paired_food_runtime_with_action_credit(
        candidate,
        cyan_nutritious,
        root,
        true,
        action_credit,
    );
    let organism = OrganismId(1);
    let foods = [WorldEntityId(2), WorldEntityId(3)];
    let nutritious = if cyan_nutritious { foods[0] } else { foods[1] };
    let mut result = ChoiceResult {
        cyan_nutritious,
        world_ticks: 0,
        awake_opportunities: [0; 2],
        late_both_available: 0,
        late_nutritious_meals: 0,
        late_harmful_meals: 0,
        all_nutritious_meals: 0,
        all_harmful_meals: 0,
        first_harmful_third_factor: None,
        first_nutritious_third_factor: None,
        harmful_positive_factors: 0,
        harmful_negative_factors: 0,
        harmful_zero_factors: 0,
        no_progress_polls: 0,
    };
    let started = Instant::now();
    while runtime.world_snapshot().tick().raw() < 32 && started.elapsed().as_secs() < 120 {
        let before = runtime.world_snapshot();
        let record = before.organism_registry().get(organism).unwrap();
        if !record.lifecycle().is_alive() {
            break;
        }
        let receptors = record
            .biochemistry()
            .neural_receptor_frame(record.phenotype())
            .unwrap();
        let sensed = before.sensory_report(organism, before.tick()).unwrap();
        let available = foods.map(|food| {
            !before.entity(food).unwrap().consumed
                && sensed
                    .visible_entities
                    .iter()
                    .any(|object| object.id == food)
        });
        let fast_before = runtime.active_fast_weights_for_test(organism).unwrap();
        let summaries = match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::NoProgress(reason) => {
                result.no_progress_polls += 1;
                std::fs::write(root.join("no-progress.json"), serde_json::to_vec_pretty(&serde_json::json!({
                    "count":result.no_progress_polls,"tick":before.tick(),"reason":format!("{reason:?}")
                })).unwrap()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            GpuLiveTickOutcome::Progressed(summaries) => summaries,
        };
        let after = runtime.world_snapshot();
        result.world_ticks = after.tick().raw();
        let alive = after
            .organism_registry()
            .get(organism)
            .unwrap()
            .lifecycle()
            .is_alive();
        let path = root.join(format!("tick-{:02}.json", result.world_ticks));
        if !summaries[0].patch_sealed {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "tick":after.tick(),"sealed":false,"alive":alive,"availability":available,
                    "biochemistry":after.organism_registry().get(organism).unwrap().biochemistry(),
                }))
                .unwrap(),
            )
            .unwrap();
            continue;
        }
        for (count, available) in result.awake_opportunities.iter_mut().zip(&available) {
            *count += u32::from(*available);
        }
        let late_choice_opportunity = result.world_ticks > 16 && available.iter().all(|v| *v);
        if late_choice_opportunity {
            result.late_both_available += 1;
        }
        let patch = runtime.sealed_patches().last().unwrap().clone();
        let evidence = patch.decision().neural_evidence().unwrap();
        assert_eq!(evidence.phenotype_hash, phenotype.phenotype_hash());
        let credit = OutcomeCreditPacket::from_sealed_patch(&patch)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        let lanes = *credit.modulator().frame().lanes();
        let consumed = patch.outcome().physical.contact == PhysicalContactKind::Consumed;
        let receptor_family = if consumed {
            CandidateActionFamily::Ingest
        } else {
            evidence.action_family
        };
        let receptor = phenotype.synapses().iter().find_map(|s| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == receptor_family =>
            {
                Some(&phenotype.plasticity_receptors()[usize::from(s.receptor_index())])
            }
            _ => None,
        });
        let third_factor = receptor.map(|r| {
            let mut sum = 0.0_f32;
            let mut scale = 0.0_f32;
            for (weight, lane) in r.receptor_profile().weights().iter().zip(lanes) {
                sum += weight * lane;
                scale += weight.abs();
            }
            if scale == 0.0 {
                0.0
            } else {
                (sum / scale).clamp(-1.0, 1.0)
            }
        });
        let physiology = patch.outcome().measured_physiology.as_ref().unwrap();
        let target = patch.outcome().physical.target_entity;
        if consumed {
            let value = third_factor.expect("consumption must use a real candidate receptor");
            if target == Some(nutritious) {
                result.all_nutritious_meals += 1;
                result.first_nutritious_third_factor.get_or_insert(value);
                if late_choice_opportunity {
                    result.late_nutritious_meals += 1;
                }
            } else {
                result.all_harmful_meals += 1;
                assert!(target.is_some_and(|t| foods.contains(&t)));
                result.first_harmful_third_factor.get_or_insert(value);
                if value > 0.0 {
                    result.harmful_positive_factors += 1;
                } else if value < 0.0 {
                    result.harmful_negative_factors += 1;
                } else {
                    result.harmful_zero_factors += 1;
                }
                if late_choice_opportunity {
                    result.late_harmful_meals += 1;
                }
            }
        }
        let activity = if alive {
            Some(runtime.evidence_activity_snapshot(organism).unwrap())
        } else {
            None
        };
        let fast_after = if alive {
            Some(runtime.active_fast_weights_for_test(organism).unwrap())
        } else {
            None
        };
        let learning = runtime.last_learning_receipts().first().unwrap();
        let row = serde_json::json!({
            "tick":after.tick(),"sealed":true,"alive":alive,"availability":available,
            "regrowth_count":after.ecology_metrics().resources_regrown,"receptors":receptors,"patch":patch,
            "pain_before":physiology.before.homeostasis.drives.pain,"pain_after":physiology.after.homeostasis.drives.pain,
            "pain_delta":physiology.pain_delta,"energy_delta":physiology.energy_delta,
            "world_energy_before":record.biochemistry().body.energy,
            "world_energy_after":after.organism_registry().get(organism).unwrap().biochemistry().body.energy,
            "sealed_energy_after":physiology.after.body.energy,
            "credit_lanes":lanes,"decoder_receptor":receptor,"receptor_family":receptor_family,"third_factor":third_factor,
            "late_choice_opportunity":late_choice_opportunity,
            "pressure":activity.as_ref().and_then(|a| a.pressure),"throttle":activity.as_ref().and_then(|a| a.throttle.as_ref()),
            "work":activity.as_ref().and_then(|a| a.work.as_ref()),"actual_gpu_time_ns":activity.as_ref().map(|a| a.next_completed_gpu_time_ns),
            "fast_before":fast_before.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "fast_after":fast_after.as_ref().map(|v| v.iter().map(|w|w.to_bits()).collect::<Vec<_>>()),
            "learning":{"sequence":learning.sequence_id,"dispatch":learning.dispatch_generation,
                "input_fast":learning.input_fast_generation,"output_fast":learning.output_fast_generation,
                "eligibility":learning.output_eligibility_generation,"changed":learning.fast_weights_changed,"max_abs_delta":learning.max_abs_delta},
        });
        std::fs::write(path, serde_json::to_vec_pretty(&row).unwrap()).unwrap();
        println!("nociception_case={cyan_nutritious}; tick={}; contact={:?}; target={target:?}; pain={}→{}; TF={third_factor:?}",
            result.world_ticks,patch.outcome().physical.contact,physiology.before.homeostasis.drives.pain,physiology.after.homeostasis.drives.pain);
        assert_eq!(learning.dispatch_generation, evidence.dispatch_generation);
    }
    std::fs::write(
        root.join("outcome.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    result
}
