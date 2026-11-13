// Local full-host checkpoint validation probe; no World stepping or state edits.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{AdamWConfig, StageTrainableMask, cpu::*};
use alife_world::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(2026111701, 1)?;
    c.brain_class = BrainScaleTier::Standard2048;
    c.sensor_profile = asset.manifest().sensor_profile();
    Ok(c)
}
// Load the exported construction genome directly. No new chromosome mutation,
// weight rebinding or reconstruction of a different founder is performed.
fn prepare_parent(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
) -> Result<(BrainPhenotype, CanonicalNewGame)> {
    let embedded = genome
        .n2048_foundation_candidate
        .as_ref()
        .ok_or("parent genome lacks foundation")?;
    if embedded.encode_canonical()? != asset.encode_canonical()? {
        return Err("genome/asset pair mismatch".into());
    }
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express()?;
    let dev = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        dev,
        asset.clone(),
    )?;
    if !compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits())
    {
        return Err("compiled parent weights changed".into());
    }
    game.world
        .replace_organism_record_exact(WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            genome.clone(),
            expressed,
            *record.biochemistry(),
            record.birth_tick(),
        )?)?;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(genome)?).0;
    Ok((compiled, game))
}

#[path = "support/terminal_evidence2135.rs"]
mod evidence;
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(&args[1])?)?;
    let genome: CreatureGenome = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let (p, _) = prepare_parent(&asset, &genome)?;
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default())?;
    let source_bytes = std::fs::read(&args[3])?;
    let low_atp = args.get(5).is_some_and(|a| a == "--low-atp");
    let variants = if low_atp { vec![("low-atp-v2", None)] } else { vec![("none",None),("praise",Some(PlayerInteractionV1::Praise)),
        ("scold",Some(PlayerInteractionV1::Scold)),("pet",Some(PlayerInteractionV1::Pet)),
        ("slap",Some(PlayerInteractionV1::Slap))] };
    for (label, kind) in variants {
        let mut ev = evidence::LocalEvidence::begin(std::path::Path::new(&args[4]), &format!("feedback-v6-{label}"))?;
        ev.save("SOURCE.cp.json", &source_bytes)?;
        ev.save("FOUNDER-OPTIMIZER.json", &serde_json::to_vec(&trainer.checkpoint())?)?;
        ev.save("PROVENANCE.json", &serde_json::to_vec(&serde_json::json!({"evidence_directory":ev.directory(),"source":args[3],"label":label,"steps":2,"experiment":"matched delivery only, no learned-skill claim","source_code":include_str!("terminal_feedback_probe2135.rs")}))?)?;
        let result = (|| -> Result<serde_json::Value> {
            let cp: CpuCanonicalCheckpoint = serde_json::from_slice(&source_bytes)?;
            let mut host = CpuCanonicalHost::from_checkpoint(cp, &trainer)?;
            let (own, entity) = host.world().organism_entity_ids()[0];
            if low_atp {
                // Declared physiology fixture, not a learned-state migration or measured natural run.
                let mut registry = host.world().organism_registry().clone();
                registry.with_biology_mut(own, |biology| {
                    biology.homeostasis.drives.brain_atp = 0.1;
                    Ok(())
                })?;
                host.world_mut().replace_organism_registry_exact(registry.iter().cloned())?;
                ev.save("LOW-ATP-FIXTURE.cp.json", &serde_json::to_vec(&host.checkpoint(&trainer)?)?)?;
            }
            let before = *host.world().organism_registry().get(own).unwrap().biochemistry();
            let position = host.world().entity(entity).unwrap().position;
            let queued = kind.map(|k|host.world_mut().queue_player_interaction_v1(own,position,k)).transpose()?;
            let mut all_rows = Vec::new();
            for n in 0..2 {
                ev.remember_valid(serde_json::to_vec(&host.checkpoint(&trainer)?)?);
                let step = host.step(&trainer,1.)?;
                let bytes = serde_json::to_vec(&host.checkpoint(&trainer)?)?;
                ev.remember_valid(bytes.clone());
                ev.save(&format!("STEP-{n}.cp.json"),&bytes)?;
                ev.save(&format!("STEP-{n}.rows.json"),&serde_json::to_vec(&step.decisions)?)?;
                all_rows.extend(step.decisions);
            }
            let after = *host.world().organism_registry().get(own).unwrap().biochemistry();
            let bytes = serde_json::to_vec(&host.checkpoint(&trainer)?)?;
            let restored = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?,&trainer)?;
            assert_eq!(bytes,serde_json::to_vec(&restored.checkpoint(&trainer)?)?);
            let summary = serde_json::json!({"label":label,"queued":queued,"decisions":all_rows.len(),
                "before_drives":before.homeostasis.drives,"after_drives":after.homeostasis.drives,
                "before_hormones":before.homeostasis.hormones,"after_hormones":after.homeostasis.hormones,
                "before_health":before.body.health,"after_health":after.body.health,
                "biological_rewards":all_rows.iter().map(|r|r.biological_reward).collect::<Vec<_>>(),
                "terminal_admissions":all_rows.iter().map(|r|r.terminal_context2135.as_ref().map(|t|&t.admission)).collect::<Vec<_>>(),"cold_exact":true});
            ev.save("COMPLETED.json", &serde_json::to_vec(&summary)?)?;
            Ok(summary)
        })();
        match result {
            Ok(v) => println!("{v}"),
            Err(e) => { ev.retain_error_checkpoint()?; let v=serde_json::json!({"label":label,"error":e.to_string(),"automatic_retry":false});ev.save("ERROR.json",&serde_json::to_vec(&v)?)?;println!("{v}"); }
        }
    }
    Ok(())
}
