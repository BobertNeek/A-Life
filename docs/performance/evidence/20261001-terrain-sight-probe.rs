use alife_core::{HomeostaticSnapshot, OrganismId, SensorProfile, Tick, Vec3f};
use alife_world::{
    HeadlessScenarioBuilder, HeadlessWorld, LocomotionLimits, TerrainData, WorldTerrain,
};
use std::hint::black_box;
use std::time::Instant;

fn world(far_count: usize) -> HeadlessWorld {
    let mut builder = HeadlessScenarioBuilder::new(60012);
    for row in 0..8 {
        builder = builder.agent(
            &format!("observer-{row}"),
            OrganismId(row + 1),
            Vec3f::new(row as f32 * 0.05, 0.0, 0.0),
        );
    }
    for (label, x, z) in [
        ("near-visible", 3.0, 2.0),
        ("near-occluded", 4.0, 0.0),
        ("at-radius", 0.0, 8.0),
        ("outside-radius", 0.0, 8.0001),
    ] {
        builder = builder.food(label, Vec3f::new(x, z, 0.0), 0.6);
    }
    for row in 0..far_count {
        let x = 9.0 + (row % 16) as f32 * 0.4;
        let z = (row / 16) as f32 * 0.4;
        builder = builder.food(&format!("far-{row}"), Vec3f::new(x, z, 0.0), 0.6);
    }
    let mut world = builder.build().unwrap();
    let mut obstacles = vec![[2.0, -0.15, 2.25, 0.15, 0.0, 2.0]];
    for i in 0..1715 {
        let x = 25.0 + (i % 40) as f32 * 0.4;
        let z = 25.0 + (i / 40) as f32 * 0.4;
        obstacles.push([x, z, x + 0.1, z + 0.1, 0.0, 2.0]);
    }
    let terrain = WorldTerrain::new(
        TerrainData {
            width: 81,
            depth: 81,
            origin_x: -24.0,
            origin_z: -24.0,
            spacing: 1.0,
            heights: vec![0.0; 81 * 81],
            obstacles,
            water_level: None,
        },
        LocomotionLimits::default(),
    )
    .unwrap();
    world
        .enable_terrain_for_new_game(terrain, Vec3f::ZERO)
        .unwrap();
    world
}

fn main() {
    for profile in [
        SensorProfile::GroundedObjectSlotsV1,
        SensorProfile::GroundedTerrainVisionV1,
    ] {
        for far_count in [0, 32, 128] {
            let mut world = world(far_count);
            let index = world.build_perception_batch_index().unwrap();
            for _ in 0..20 {
                for row in 1..=8 {
                    black_box(
                        world
                            .perception_frame_draft_indexed(
                                OrganismId(row),
                                Tick::ZERO,
                                profile,
                                HomeostaticSnapshot::baseline(Tick::ZERO),
                                &index,
                            )
                            .unwrap(),
                    );
                }
            }
            let mut samples_ns = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                for _ in 0..20 {
                    for row in 1..=8 {
                        black_box(
                            world
                                .perception_frame_draft_indexed(
                                    OrganismId(row),
                                    Tick::ZERO,
                                    profile,
                                    HomeostaticSnapshot::baseline(Tick::ZERO),
                                    &index,
                                )
                                .unwrap(),
                        );
                    }
                }
                samples_ns.push(start.elapsed().as_nanos() as u64 / 20);
            }
            let drafts = (1..=8)
                .map(|row| {
                    world
                        .perception_frame_draft_indexed(
                            OrganismId(row),
                            Tick::ZERO,
                            profile,
                            HomeostaticSnapshot::baseline(Tick::ZERO),
                            &index,
                        )
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let words = serde_json::to_vec(&drafts).unwrap();
            println!("profile={profile:?} far_count={far_count} obstacles=1716 samples_ns_per_eight_rows={samples_ns:?} draft_bytes={} draft_blake3={}", words.len(), blake3::hash(&words));
        }
    }
}
