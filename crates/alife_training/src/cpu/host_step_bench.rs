//! Bounded, explicitly invoked host-span experiment; no retained-life input.
use super::*;
use alife_world::{
    create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig, WorldOrganismRecord,
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    time::Instant,
};
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ROWS: RefCell<Vec<Span>> = const { RefCell::new(Vec::new()) };
}
#[derive(serde::Serialize)]
struct Span {
    tick: u64,
    residents: usize,
    decisions: usize,
    validation_ns: u64,
    clone_ns: u64,
    inner_ns: u64,
    commit_drop_ns: u64,
}
#[allow(dead_code)]
pub(super) fn clock() -> Option<Instant> {
    ENABLED.with(|x| x.get()).then(Instant::now)
}
#[allow(dead_code)]
pub(super) fn record(result: &CpuCanonicalStep, residents: usize, points: [Option<Instant>; 5]) {
    let [Some(a), Some(b), Some(c), Some(d), Some(e)] = points else {
        return;
    };
    ROWS.with(|r| {
        r.borrow_mut().push(Span {
            tick: result.tick_before.raw(),
            residents,
            decisions: result.decisions.len(),
            validation_ns: b.duration_since(a).as_nanos() as u64,
            clone_ns: c.duration_since(b).as_nanos() as u64,
            inner_ns: d.duration_since(c).as_nanos() as u64,
            commit_drop_ns: e.duration_since(d).as_nanos() as u64,
        })
    });
}
fn new_game_with_profile_fixture(
    atp: Option<f32>,
    profile: SensorProfile,
) -> (
    CanonicalNewGame,
    CpuFoundationTrainer,
    PhenotypeCompilerInputs,
) {
    let builtin = if profile == SensorProfile::GroundedTerrainVisionV1 {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x7A1123, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let phenotype = PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            profile,
        )
        .unwrap();
        FoundationWeightAsset::from_phenotype_for_genetic_birth(&phenotype).unwrap()
    } else {
        FoundationWeightAsset::builtin_n2048_v1(profile).unwrap()
    };
    let mut config = CanonicalNewGameConfig::phase3(0x7A1123, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = profile;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &builtin).unwrap();
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = &record.phenotype().brain_genome;
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let base =
        PhenotypeCompiler::compile(genome, &capacity, &development, config.sensor_profile).unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (p, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    let mut game = create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
    if let Some(atp) = atp {
        let record = game.world.organism_registry().iter().next().unwrap();
        let mut chemistry = *record.biochemistry();
        chemistry.homeostasis.drives.brain_atp = atp;
        let replacement = WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            record.genome().clone(),
            record.phenotype().clone(),
            chemistry,
            record.birth_tick(),
        )
        .unwrap();
        game.world
            .replace_organism_record_exact(replacement)
            .unwrap();
    }
    let mask = StageTrainableMask::from_synapse_indices(
        &p,
        &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
    )
    .unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
    (game, trainer, inputs)
}

#[test]
#[ignore = "bounded CPU host span experiment; explicitly run in cloud"]
fn bounded_native_host_spans() {
    let control = std::env::var_os("ALIFE_HOST_CONTROL_ONLY").is_some();
    let (game, trainer, _) =
        new_game_with_profile_fixture(None, SensorProfile::GroundedTerrainVisionV1);
    let initial = CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap();
    let mut trials = Vec::new();
    let mut expected = None;
    for trial in 0..8 {
        let modes: Vec<_> = if control {
            vec![false]
        } else if trial % 2 == 0 {
            vec![false, true]
        } else {
            vec![true, false]
        };
        for enabled in modes {
            ROWS.with(|r| {
                let mut r = r.borrow_mut();
                r.clear();
                r.reserve(65);
            });
            ENABLED.with(|x| x.set(enabled));
            let mut host = initial.snapshot();
            let mut outer = Vec::with_capacity(65);
            let mut decisions = Vec::with_capacity(65);
            let started = Instant::now();
            for _ in 0..65 {
                let at = Instant::now();
                let step = host.step(&trainer, 0.9).unwrap();
                outer.push(at.elapsed().as_nanos() as u64);
                decisions.extend(step.decisions);
            }
            let loop_ns = started.elapsed().as_nanos() as u64;
            ENABLED.with(|x| x.set(false));
            assert_eq!(decisions.len(), 65);
            assert!(host
                .world()
                .organism_registry()
                .iter()
                .all(|r| r.lifecycle().is_alive()));
            let decision_hash = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&decisions).unwrap())
            );
            let private_hash = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap())
            );
            let identity = (decision_hash.clone(), private_hash.clone());
            if let Some(ref expected) = expected {
                assert_eq!(expected, &identity);
            } else {
                expected = Some(identity);
            }
            let spans = ROWS.with(|r| std::mem::take(&mut *r.borrow_mut()));
            if enabled {
                assert_eq!(spans.len(), 65);
            }
            for (r, wall) in spans.iter().zip(&outer) {
                assert_eq!(r.residents, 1);
                assert_eq!(r.decisions, 1);
                assert!(r.validation_ns + r.clone_ns + r.inner_ns + r.commit_drop_ns <= *wall);
            }
            trials.push(serde_json::json!({"trial":trial,"warm_up":trial==0,"mode":if control {"uninstrumented"} else if enabled {"enabled"} else {"disabled"},
                "world_steps":65,"native_decisions":decisions.len(),"loop_ns":loop_ns,"outer_step_ns":outer,"spans":spans,
                "decisions_sha256":decision_hash,"private_sha256":private_hash}));
        }
    }
    let tag = std::env::var("ALIFE_HOST_BENCH_TAG").unwrap();
    let dir = std::path::PathBuf::from(std::env::var_os("ALIFE_BENCH_OUTPUT_DIR").unwrap());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{tag}.json")),
        serde_json::to_vec_pretty(&trials).unwrap(),
    )
    .unwrap();
    eprintln!("HOST_SPANS {} trials; native CPU only", trials.len());
}
