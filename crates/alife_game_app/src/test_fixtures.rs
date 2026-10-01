//! Current organism authority for app tests, with historical scene/asset metadata as input.
//! This constructs a new test world; it does not migrate the saved source fixture.

use std::path::Path;

use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, OrganismId, Tick, Vec3f,
};
use alife_world::{
    persistence::{
        AssetManifest, CreatureSaveState, PortableAssetDigest, PortableSaveFile, RuntimeConfig,
    },
    HeadlessScenarioBuilder, WorldObjectKind, WorldOrganismRecord,
};

pub(crate) fn current_scene_save(root: &Path, population: usize) -> PortableSaveFile {
    let scene: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("tiny_save.json")).unwrap()).unwrap();
    let config = RuntimeConfig::from_json_file(root.join("tiny_config.json")).unwrap();
    let assets: AssetManifest = serde_json::from_value(scene["assets"].clone()).unwrap();
    let mut summaries = scene["creatures"].clone();
    // CPU scene tests do not restore neural tensors. Historical GPU checkpoints
    // have their own schema and must not be relabeled as current checkpoints.
    for summary in summaries.as_array_mut().unwrap() {
        summary["gpu_brain"] = serde_json::Value::Null;
    }
    let templates: Vec<CreatureSaveState> = serde_json::from_value(summaries).unwrap();
    let mut objects = scene["world"]["objects"].as_array().unwrap().clone();
    objects.sort_by_key(|object| object["id"].as_u64().unwrap());
    let mut builder = HeadlessScenarioBuilder::new(config.deterministic_seed);
    let mut existing = 0;
    for object in objects {
        let label = object["label"].as_str().unwrap();
        let position: Vec3f = serde_json::from_value(object["position"].clone()).unwrap();
        builder = match object["kind"].as_str().unwrap() {
            "Agent" if existing < population => {
                existing += 1;
                builder.agent(
                    label,
                    OrganismId(object["organism_id"].as_u64().unwrap()),
                    position,
                )
            }
            "Agent" => continue,
            "Food" => builder.food(
                label,
                position,
                object["nutrition"].as_f64().unwrap() as f32,
            ),
            "Hazard" => builder.hazard(
                label,
                position,
                object["hazard_pain"].as_f64().unwrap() as f32,
            ),
            "Obstacle" => {
                builder.obstacle(label, position, object["radius"].as_f64().unwrap() as f32)
            }
            kind => panic!("unsupported test scene object kind {kind}"),
        };
    }
    for index in existing..population {
        builder = builder.agent(
            &format!("fixture-creature-{}", index + 1),
            OrganismId(index as u64 + 1),
            Vec3f::new((index % 20) as f32, 0.0, (index / 20) as f32),
        );
    }
    let mut world = builder.build().unwrap();
    let mut creatures = Vec::new();
    for object in world.object_snapshots() {
        if object.kind != WorldObjectKind::Agent {
            continue;
        }
        let organism_id = object.organism_id.unwrap();
        let genome = CreatureGenome::early_mammal_founder(
            config.deterministic_seed + organism_id.raw(),
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let record =
            WorldOrganismRecord::newborn(organism_id, object.id, genome, phenotype, Tick::ZERO)
                .unwrap();
        let template = templates
            .iter()
            .find(|creature| creature.organism_id == organism_id);
        let mut creature = template.unwrap_or(&templates[0]).clone();
        creature.organism_id = organism_id;
        creature.genome_id = record.genome().id;
        creature.development_tick = record.biochemistry().development.last_update_tick;
        creature.mind.tick = record.biochemistry().tick;
        creature.mind.homeostasis = record.biochemistry().homeostasis;
        creature.weights.genetic_fixed_digest =
            PortableAssetDigest::for_bytes(&serde_json::to_vec(record.genome()).unwrap()).0;
        if template.is_none() {
            creature.gpu_brain = None;
        }
        world.register_organism_record(record).unwrap();
        creatures.push(creature);
    }
    let save = PortableSaveFile::from_headless_world(
        "current-app-test-scene",
        &world,
        config,
        assets,
        creatures,
    )
    .unwrap();
    save.validate_with_asset_root(root).unwrap();
    save
}
