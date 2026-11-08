//! Fresh deterministic wire comparison. No saved life or checkpoint input.
use alife_core::*;
use alife_world::*;

fn digest<T: serde::Serialize>(value: &T) -> String {
    format!(
        "{:?}",
        PortableAssetDigest::for_bytes(&serde_json::to_vec(value).unwrap())
    )
}
fn main() {
    let output = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    let config = CanonicalNewGameConfig::phase3(0x0343_4D31_4658_0001, 1).unwrap();
    let foundation = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
    for awake_rest in [false, true] {
        let mut game = if awake_rest {
            create_canonical_new_game_with_awake_rest_research_v1(&config).unwrap()
        } else {
            create_canonical_new_game(&config, &foundation).unwrap()
        };
        for age in [0, 240, 1200] {
            while game.world.tick().raw() < age {
                game.world.try_advance_tick().unwrap();
            }
            let record = game.world.organism_registry().get(OrganismId(1)).unwrap();
            let biology = record.biochemistry();
            game.creatures[0].mind.tick = biology.tick;
            game.creatures[0].mind.homeostasis = biology.homeostasis;
            game.creatures[0].development_tick = biology.development.last_update_tick;
            let save = PortableSaveFile::from_headless_world(
                "fresh-recovery-legacy-wire",
                &game.world,
                RuntimeConfig::deterministic_default(config.world_seed, BrainScaleTier::Nano512),
                AssetManifest::empty(),
                game.creatures.clone(),
            )
            .unwrap();
            let bytes = serde_json::to_vec(&save).unwrap();
            let restored =
                PortableSaveFile::from_json_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
            assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
            assert_eq!(
                record,
                restored
                    .restore_headless_world()
                    .unwrap()
                    .organism_registry()
                    .get(OrganismId(1))
                    .unwrap()
            );
            if let Some(path) = &output {
                std::fs::write(
                    path.join(format!("awake-rest-{awake_rest}-age-{age}.json")),
                    &bytes,
                )
                .unwrap();
            }
            println!("LEGACY_WIRE awake_rest={awake_rest} age={age} portable={} genome={} phenotype={} biology={} brain={} memory={} canonical_foundation={:?}",
                digest(&save),digest(record.genome()),digest(record.phenotype()),digest(record.biochemistry()),
                digest(&record.state_graph().brain),digest(&record.state_graph().memory),
                PortableAssetDigest::for_bytes(&foundation.encode_canonical().unwrap()));
        }
    }
}
