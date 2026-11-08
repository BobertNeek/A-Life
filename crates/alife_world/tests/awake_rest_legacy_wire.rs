//! Run unchanged against accepted main and the candidate; compare LEGACY_WIRE
//! records exactly. Only fixed disposable ordinary founders, never saved lives.
use alife_core::*;
use alife_world::*;

fn digest(bytes: &[u8]) -> [u64; 4] {
    let mut hash = CanonicalDigestBuilder::new(b"AWAKE-REST-LEGACY-WIRE-CONTROL-V1");
    hash.write_bytes(bytes);
    hash.finish256()
}

#[test]
fn awake_rest_legacy_wire_baseline() {
    let config = CanonicalNewGameConfig::phase3(961_060, 1).unwrap();
    let foundation = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
    let game = create_canonical_new_game(&config, &foundation).unwrap();
    let subject = OrganismId(1);
    let mut prepared = game.world;
    for age in [0, 60, 240] {
        while prepared.tick().raw() < age {
            prepared.try_advance_tick().unwrap();
        }
        for (label, kind, id, registered) in [
            (
                "rest-direct",
                ActionKind::Rest,
                ActionKind::Rest.canonical_id(),
                false,
            ),
            (
                "rest-registered",
                ActionKind::Rest,
                ActionKind::Rest.canonical_id(),
                true,
            ),
            (
                "unknown-rest-direct",
                ActionKind::Rest,
                ActionId(u32::MAX),
                false,
            ),
            (
                "unknown-rest-registered",
                ActionKind::Rest,
                ActionId(u32::MAX),
                true,
            ),
            (
                "idle",
                ActionKind::Idle,
                ActionKind::Idle.canonical_id(),
                true,
            ),
            (
                "no-posture",
                ActionKind::Hold,
                HeadlessActionIds::NO_POSTURE,
                true,
            ),
        ] {
            let mut world = prepared.clone();
            let record = world.organism_registry().get(subject).unwrap();
            let own = record.world_entity_id();
            let command = ActionCommand::structured(
                subject,
                id,
                kind,
                ActionTarget::NONE,
                Intensity::new(1.0).unwrap(),
                DurationTicks::new(1),
                Confidence::new(1.0).unwrap(),
                0,
                None,
                None,
                None,
            )
            .unwrap();
            let receipt = if registered {
                format!(
                    "{:?}",
                    world
                        .apply_registered_command(&command, own, Tick(age + 1))
                        .unwrap()
                )
            } else {
                format!("{:?}", world.apply_command(&command).unwrap())
            };
            world.try_advance_tick().unwrap();
            let biology = world
                .organism_registry()
                .get(subject)
                .unwrap()
                .biochemistry();
            let mut creatures = game.creatures.clone();
            creatures[0].mind.tick = biology.tick;
            creatures[0].mind.homeostasis = biology.homeostasis;
            creatures[0].development_tick = biology.development.last_update_tick;
            let save = PortableSaveFile::from_headless_world(
                "legacy-awake-rest-control",
                &world,
                RuntimeConfig::deterministic_default(961_060, BrainScaleTier::Nano512),
                AssetManifest::empty(),
                creatures,
            )
            .unwrap();
            let wire = serde_json::to_vec(&save).unwrap();
            // Optional local-only byte comparison of these freshly generated
            // disposable fixtures. Never point this at a user's saved life.
            if let Some(directory) = std::env::var_os("ALIFE_AWAKE_REST_WIRE_OUTPUT") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(directory.join(format!("age-{age}-{label}.json")), &wire).unwrap();
                std::fs::write(
                    directory.join(format!("age-{age}-{label}.receipt.txt")),
                    &receipt,
                )
                .unwrap();
            }
            let restored = PortableSaveFile::from_json_str(std::str::from_utf8(&wire).unwrap())
                .unwrap()
                .restore_headless_world()
                .unwrap();
            assert_eq!(
                world.canonical_signature_digest().unwrap(),
                restored.canonical_signature_digest().unwrap()
            );
            assert_eq!(
                world.organism_registry().get(subject),
                restored.organism_registry().get(subject)
            );
            println!(
                "LEGACY_WIRE age={age} action={label} bytes={} save={:?} receipt={:?} world={:?}",
                wire.len(),
                digest(&wire),
                digest(receipt.as_bytes()),
                world.canonical_signature_digest().unwrap()
            );
        }
    }
}
