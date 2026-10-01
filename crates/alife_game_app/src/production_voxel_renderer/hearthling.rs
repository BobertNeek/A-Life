//! Approved Blender character, projected onto existing organism identities.
use super::*;
use bevy::{camera::primitives::Aabb, gltf::Gltf, prelude::*, scene::SceneInstanceReady};

const PATH: &str = "creatures/hearthling/hearthling.glb";

// export_hearthling.py: each foot travels from -0.36 to +0.36 model units.
// Two steps cover 1.44 units per 24-frame cycle. The GLB contains three cycles,
// sampled at 24 fps starting at frame 1, rather than at time zero.
const WALK_CYCLE_DISTANCE: f32 = 1.44;
const WALK_CLIP_START: f32 = 1.0 / 24.0;
const WALK_CLIP_SECONDS: f32 = 3.0;
const INGESTION_FEEDBACK_SECONDS: f32 = 0.60;
const SPEECH_FEEDBACK_SECONDS: f32 = 0.45;

#[derive(Default)]
struct OutcomeFeedback {
    last_ingestion_tick: Option<u64>,
    last_utterance_id: Option<u64>,
    ingestion_remaining: f32,
    speech_remaining: f32,
}

impl OutcomeFeedback {
    fn advance(&mut self, seconds: f32, paused: bool) {
        if !paused {
            self.ingestion_remaining = (self.ingestion_remaining - seconds).max(0.0);
            self.speech_remaining = (self.speech_remaining - seconds).max(0.0);
        }
    }

    fn observe(&mut self, ingestion_tick: Option<u64>, utterance_id: Option<u64>) {
        if let Some(tick) =
            ingestion_tick.filter(|tick| self.last_ingestion_tick.is_none_or(|last| *tick > last))
        {
            self.last_ingestion_tick = Some(tick);
            self.ingestion_remaining = INGESTION_FEEDBACK_SECONDS;
        }
        if let Some(id) =
            utterance_id.filter(|id| self.last_utterance_id.is_none_or(|last| *id > last))
        {
            self.last_utterance_id = Some(id);
            self.speech_remaining = SPEECH_FEEDBACK_SECONDS;
        }
    }
}

fn confirmed_consumption(receipt: &alife_world::HeadlessMotorChannelReceipt) -> bool {
    receipt.command.channel == alife_core::MotorChannel::Manipulation
        && receipt.command.primitive == alife_world::HeadlessActionIds::EAT
        && receipt.physical.contact == alife_core::PhysicalContactKind::Consumed
}

fn confirmed_ingestion_tick(
    summary: &LiveBrainTickSummary,
    organism: OrganismId,
    current_tick: u64,
) -> Option<u64> {
    (summary.organism_id == organism
        && summary.patch_sealed
        && summary.world_tick_after.raw() <= current_tick
        && summary
            .motor_execution
            .as_ref()
            .is_some_and(|trace| trace.channel_receipts.iter().any(confirmed_consumption)))
    .then_some(summary.world_tick_after.raw())
}

fn emitted_utterance_id(
    utterances: &[alife_world::AudibleUtterance],
    organism: OrganismId,
    current_tick: u64,
) -> Option<u64> {
    utterances
        .iter()
        .filter(|utterance| {
            utterance.source_kind == alife_core::UtteranceSourceKind::Creature
                && utterance.speaker_id == Some(organism)
                && utterance.emitted_tick.raw() <= current_tick
                && utterance.expires_after_tick.raw() >= current_tick
                && !utterance.tokens.is_empty()
        })
        .map(|utterance| utterance.utterance_id.raw())
        .max()
}

fn advance_walk(distance: f32, forward_scale: f32, seconds: f32) -> f32 {
    (seconds + distance / (WALK_CYCLE_DISTANCE * forward_scale)).rem_euclid(WALK_CLIP_SECONDS)
}

#[derive(Resource, Default)]
struct SharedHearthlingAssets {
    graph: Option<(Handle<AnimationGraph>, Vec<AnimationNodeIndex>)>,
    coats: BTreeMap<(bevy::asset::AssetId<StandardMaterial>, usize), Handle<StandardMaterial>>,
}

#[derive(Component)]
pub(super) struct HearthlingVisual {
    appearance: CreatureAppearanceGenome,
    previous_position: Vec3,
    walk_seconds: f32,
    moved: bool,
    feedback: OutcomeFeedback,
}

impl HearthlingVisual {
    pub(super) fn capture_feedback(&self) -> serde_json::Value {
        serde_json::json!({
            "last_confirmed_ingestion_tick": self.feedback.last_ingestion_tick,
            "last_emitted_utterance_id": self.feedback.last_utterance_id,
            "ingestion_remaining_seconds": self.feedback.ingestion_remaining,
            "speech_remaining_seconds": self.feedback.speech_remaining,
            "ingestion_duration_seconds": INGESTION_FEEDBACK_SECONDS,
            "speech_duration_seconds": SPEECH_FEEDBACK_SECONDS,
            "ingestion_source": "sealed_matching_organism_manipulation_eat_consumed_receipt",
            "speech_source": "active_nonempty_creature_audible_utterance_matching_speaker",
        })
    }
}

#[derive(Component)]
pub(super) struct HearthlingPlayer {
    root: Entity,
    clips: Vec<AnimationNodeIndex>,
    state: usize,
}

pub(super) fn spawn(world: &mut World, root: Entity, appearance: CreatureAppearanceGenome) {
    world.init_resource::<SharedHearthlingAssets>();
    // Headless preflight/continuity apps retain organism roots without loading scenes.
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(PATH));
    let gltf: Handle<Gltf> = server.load(PATH);
    let previous_position = world.get::<Transform>(root).unwrap().translation;
    world.entity_mut(root).insert((
        HearthlingVisual {
            appearance,
            previous_position,
            walk_seconds: 0.0,
            moved: false,
            feedback: OutcomeFeedback::default(),
        },
        HearthlingSource(gltf),
    ));
    world
        .spawn((
            Name::new("Approved Hearthling skinned scene"),
            SceneRoot(scene),
            Transform::default(),
            ChildOf(root),
        ))
        .observe(ready);
}

#[derive(Component)]
struct HearthlingSource(Handle<Gltf>);

#[derive(Component)]
pub(super) struct InheritedBoneScale(Vec3);

#[derive(Component)]
pub(super) struct HearthlingPoseBone {
    root: Entity,
    part: ExpressionBone,
    bind_rotation: Quat,
}

impl HearthlingPoseBone {
    pub(super) fn capture_root(&self) -> Entity {
        self.root
    }

    pub(super) fn capture_bind_rotation(&self) -> [f32; 4] {
        self.bind_rotation.to_array()
    }
}

#[derive(Clone, Copy)]
enum ExpressionBone {
    Head,
    Ear(f32),
    Lid(f32),
}

fn expression_rotation(
    sampled: Quat,
    bind: Quat,
    part: ExpressionBone,
    expression: CreatureExpressionState,
    animation: CreatureAnimationState,
) -> Quat {
    // The seated sleep clip already closes the eyes and lowers the head/ears.
    if animation == CreatureAnimationState::Sleeping {
        return sampled;
    }
    match (part, expression) {
        (ExpressionBone::Lid(closed_angle), CreatureExpressionState::Tired) => {
            // Leave authored blinks intact. Set a minimum partial closure rather
            // than adding rotation, which would drive a blink through the globe.
            let relative = bind.inverse() * sampled;
            let angle = 2.0 * relative.x.atan2(relative.w);
            let minimum = closed_angle * 0.28;
            if angle * closed_angle.signum() >= minimum.abs() {
                sampled
            } else {
                bind * Quat::from_rotation_x(minimum)
            }
        }
        (ExpressionBone::Head, CreatureExpressionState::Tired) => {
            sampled * Quat::from_rotation_x(0.10)
        }
        (ExpressionBone::Head, CreatureExpressionState::Pained) => {
            sampled * Quat::from_rotation_x(0.18)
        }
        (ExpressionBone::Head, CreatureExpressionState::Afraid) => {
            sampled * Quat::from_rotation_x(-0.10)
        }
        (ExpressionBone::Ear(_), CreatureExpressionState::Afraid) => {
            sampled * Quat::from_rotation_x(-0.35)
        }
        (ExpressionBone::Ear(side), CreatureExpressionState::Pained) => {
            sampled * Quat::from_rotation_z(side * 0.22)
        }
        _ => sampled,
    }
}

fn outcome_rotation(
    sampled: Quat,
    part: ExpressionBone,
    feedback: &OutcomeFeedback,
    animation: CreatureAnimationState,
) -> Quat {
    if animation == CreatureAnimationState::Sleeping {
        return sampled;
    }
    // One smooth, bounded acknowledgment of a measured outcome, not chewing
    // or lip sync. The rig has no jaw or authored feeding/speech clip.
    let pulse = |remaining: f32, duration: f32| {
        if remaining <= 0.0 || remaining >= duration {
            0.0
        } else {
            (std::f32::consts::PI * remaining / duration).sin()
        }
    };
    let ate = pulse(feedback.ingestion_remaining, INGESTION_FEEDBACK_SECONDS);
    let spoke = pulse(feedback.speech_remaining, SPEECH_FEEDBACK_SECONDS);
    if ate == 0.0 && spoke == 0.0 {
        return sampled;
    }
    match part {
        ExpressionBone::Head => sampled * Quat::from_rotation_x(0.20 * ate - 0.08 * spoke),
        ExpressionBone::Ear(_) => sampled * Quat::from_rotation_x(0.10 * spoke),
        ExpressionBone::Lid(_) => sampled,
    }
}

fn body_rotation(world_yaw: f32) -> Quat {
    // World yaw zero faces +X; the exported character faces +Z.
    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 - world_yaw)
}

fn head_rotation(sampled: Quat, world_head_yaw: f32) -> Quat {
    // Keep the authored nod/roll, replacing its idle yaw with the chosen gaze.
    let twist = Quat::from_xyzw(0.0, sampled.y, 0.0, sampled.w).normalize();
    (sampled * twist.inverse()) * Quat::from_rotation_y(-world_head_yaw)
}

pub(super) fn apply_head_direction(
    roots: Query<(&Fvr04ProductionCreatureVisualMarker, &HearthlingVisual)>,
    mut bones: Query<(&mut Transform, &HearthlingPoseBone)>,
) {
    // Every authored clip samples these rotations, even while paused. Start
    // from that frame's pose so inherited gaze/expression never accumulates.
    for (mut transform, bone) in &mut bones {
        if let Ok((marker, visual)) = roots.get(bone.root) {
            let sampled = match bone.part {
                ExpressionBone::Head => head_rotation(transform.rotation, marker.head_yaw),
                _ => transform.rotation,
            };
            let expression = expression_rotation(
                sampled,
                bone.bind_rotation,
                bone.part,
                marker.expression,
                marker.animation,
            );
            transform.rotation =
                outcome_rotation(expression, bone.part, &visual.feedback, marker.animation);
        }
    }
}

pub(super) fn apply_inherited_proportions(mut bones: Query<(&mut Transform, &InheritedBoneScale)>) {
    // All three authored clips sample scale on these bones, including at speed zero.
    // Apply the inherited multiplier after animation has restored the sampled scale.
    for (mut transform, scale) in &mut bones {
        transform.scale *= scale.0;
    }
}

pub(super) fn scale(appearance: CreatureAppearanceGenome) -> Vec3 {
    let mass = f32::from(appearance.body_mass_trait)
        / f32::from(alife_world::CREATURE_APPEARANCE_GENE_BUCKETS - 1);
    Vec3::new(0.60 + mass * 0.14, 0.67 + mass * 0.08, 0.62 + mass * 0.10)
}

fn clip(animation: CreatureAnimationState) -> usize {
    match animation {
        CreatureAnimationState::Sleeping => 2,
        CreatureAnimationState::Moving => 1,
        _ => 0,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
fn ready(
    event: On<SceneInstanceReady>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    roots: Query<(
        &HearthlingSource,
        &HearthlingVisual,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut shared: ResMut<SharedHearthlingAssets>,
    mut players: Query<&mut AnimationPlayer>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(parent) = parents.get(event.entity) else {
        return;
    };
    let root = parent.parent();
    let Ok((source, visual, marker)) = roots.get(root) else {
        return;
    };
    let Some(gltf) = gltfs.get(&source.0) else {
        return;
    };
    let (graph, clips) = shared
        .graph
        .get_or_insert_with(|| {
            let (graph, clips) = AnimationGraph::from_clips(
                [
                    "Hearthling_CuriousIdle",
                    "Hearthling_Walk",
                    "Hearthling_Sleep",
                ]
                .map(|name| gltf.named_animations[name].clone()),
            );
            (graphs.add(graph), clips)
        })
        .clone();
    let state = clip(marker.animation);
    // Tint only the coat. Eyes, nose, tongue and cream muzzle keep authored colors.
    let tints = [
        [1.0, 1.0, 1.0],
        [0.76, 1.06, 1.28],
        [1.18, 0.86, 0.76],
        [0.87, 0.86, 1.24],
        [0.86, 1.13, 0.83],
        [1.16, 1.03, 0.78],
    ];
    let palette = usize::from(visual.appearance.palette_family) % tints.len();
    let tint = tints[palette];
    let coat_materials = [
        "Hearthling | vertex-colored coat",
        "Hearthling | warm ochre",
        "Hearthling | brows and tuft shadows",
    ]
    .map(|name| gltf.named_materials[name].id());
    let mut mesh_count = 0;
    for entity in children.iter_descendants(event.entity) {
        if let Ok(name) = names.get(entity) {
            let part = match name.as_str() {
                "head" => Some(ExpressionBone::Head),
                "ear.L" => Some(ExpressionBone::Ear(1.0)),
                "ear.R" => Some(ExpressionBone::Ear(-1.0)),
                // Closure angles are authored in scripts/hearthling_eyes.py.
                "lid_upper.L" | "lid_upper.R" => Some(ExpressionBone::Lid(1.07)),
                "lid_lower.L" | "lid_lower.R" => Some(ExpressionBone::Lid(-1.02)),
                _ => None,
            };
            if let (Some(part), Ok(transform)) = (part, transforms.get(entity)) {
                commands.entity(entity).insert(HearthlingPoseBone {
                    root,
                    part,
                    bind_rotation: transform.rotation,
                });
            }
            let ear = f32::from(visual.appearance.ear_muzzle_trait) / 15.0;
            let tail = f32::from(visual.appearance.tail_trait) / 15.0;
            let scale = match name.as_str() {
                "ear.L" | "ear.R" => Some(Vec3::new(1.0, 0.85 + ear * 0.30, 1.0)),
                "head" => Some(Vec3::new(0.94 + ear * 0.12, 1.0, 1.0)),
                "tail.00" => Some(Vec3::splat(0.90 + tail * 0.20)),
                _ => None,
            };
            if let Some(scale) = scale {
                commands.entity(entity).insert(InheritedBoneScale(scale));
            }
        }
        if let Ok(mut player) = players.get_mut(entity) {
            player
                .play(clips[state])
                .repeat()
                .set_seek_time(marker.phase % 3.0)
                .set_speed(0.0);
            commands.entity(entity).insert((
                AnimationGraphHandle(graph.clone()),
                HearthlingPlayer {
                    root,
                    clips: clips.clone(),
                    state,
                },
            ));
        }
        if let Ok(handle) = mesh_materials.get(entity) {
            mesh_count += 1;
            // Covers every authored pose and inherited ear/tail scale. Static bind-pose
            // bounds can clip animated extremities; disabling culling costs every draw.
            commands
                .entity(entity)
                .insert(Aabb::from_min_max(Vec3::splat(-6.0), Vec3::splat(6.0)));
            if coat_materials.contains(&handle.0.id()) {
                let replacement =
                    shared
                        .coats
                        .entry((handle.0.id(), palette))
                        .or_insert_with(|| {
                            let mut material = materials
                                .get(&handle.0)
                                .expect("loaded glTF material")
                                .clone();
                            let color = material.base_color.to_linear();
                            material.base_color = Color::linear_rgba(
                                color.red * tint[0],
                                color.green * tint[1],
                                color.blue * tint[2],
                                color.alpha,
                            );
                            materials.add(material)
                        });
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(replacement.clone()));
            }
        }
    }
    // Bevy creates a Mesh handle for each material primitive inside a glTF mesh.
    let shared_meshes = mesh_count;
    commands.queue(move |world: &mut World| {
        if let Some(mut receipt) = world.get_resource_mut::<Fvr04ProductionCreatureSceneResource>()
        {
            receipt.creature_part_entity_count += mesh_count;
            receipt.mesh_pool_count = shared_meshes;
            receipt.creature_shared_mesh_handle_count = shared_meshes;
        }
    });
    info!(
        "Hearthling ready: stable {}, palette {}, state {}",
        marker.stable_id.raw(),
        visual.appearance.palette_family,
        state
    );
}

pub(super) fn animate(
    hand: Option<Res<god_hand::HandInteraction>>,
    time: Res<Time>,
    ux: Res<Fvr05ProductionUxStateResource>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    #[cfg(feature = "gpu-runtime")] runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
    mut players: Query<(&mut AnimationPlayer, &mut HearthlingPlayer)>,
    mut transforms: Query<(
        &mut Transform,
        &mut HearthlingVisual,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
) {
    let new_frame = frame.as_ref().is_some_and(|frame| frame.is_changed());
    // World speech is already CPU-side authoritative state. Read once per
    // changed frame, never from requested vocal commands or translated text.
    #[cfg(feature = "gpu-runtime")]
    let utterances = if new_frame {
        runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.runtime.active_utterances())
    } else {
        Vec::new()
    };
    #[cfg(not(feature = "gpu-runtime"))]
    let utterances = Vec::new();
    for (mut transform, mut visual, marker) in &mut transforms {
        visual
            .feedback
            .advance(time.delta_secs(), ux.settings.paused);
        if new_frame {
            if let Some(frame) = frame.as_ref().filter(|frame| {
                frame.current.organism(marker.stable_id).is_some_and(|row| {
                    row.organism_id == marker.organism_id && row.lifecycle.is_alive()
                })
            }) {
                let tick = frame.current.authoritative_world_tick.raw();
                let ingestion = frame
                    .current
                    .tick_summaries
                    .iter()
                    .filter_map(|summary| {
                        confirmed_ingestion_tick(summary, marker.organism_id, tick)
                    })
                    .max();
                let utterance = emitted_utterance_id(&utterances, marker.organism_id, tick);
                visual.feedback.observe(ingestion, utterance);
            }
        }
        // World ticks supply targets; render frames supply the visible stride.
        // Only the display transform is smoothed, never the organism position.
        let target = marker.base_translation;
        let blend = 1.0 - (-30.0 * time.delta_secs().min(0.1)).exp();
        if !ux.settings.paused {
            transform.translation.x =
                visual.previous_position.x + (target.x - visual.previous_position.x) * blend;
            transform.translation.z =
                visual.previous_position.z + (target.z - visual.previous_position.z) * blend;
            if Vec2::new(
                target.x - transform.translation.x,
                target.z - transform.translation.z,
            )
            .length_squared()
                < 0.000001
            {
                transform.translation.x = target.x;
                transform.translation.z = target.z;
            }
        }
        let delta = transform.translation - visual.previous_position;
        visual.previous_position = transform.translation;
        let distance = Vec2::new(delta.x, delta.z).length();
        visual.moved = !ux.settings.paused
            && distance > f32::EPSILON
            && !hand
                .as_ref()
                .is_some_and(|h| h.held == Some(marker.stable_id));
        if visual.moved {
            visual.walk_seconds = advance_walk(
                distance,
                transform.scale.z.abs().max(f32::EPSILON),
                visual.walk_seconds,
            );
        }
        // Turning in place is a real action too. Heading must never be
        // inferred from displacement, which can be blocked or sideways.
        transform.rotation = body_rotation(marker.body_yaw);
    }
    for (mut player, mut model) in &mut players {
        let Ok((_, visual, marker)) = transforms.get(model.root) else {
            continue;
        };
        // Actual displacement can outlast a selected Move action (or be blocked
        // despite it). Pose cadence follows the presented world displacement.
        let next = if hand
            .as_ref()
            .is_some_and(|h| h.held == Some(marker.stable_id))
        {
            0
        } else if ux.settings.paused {
            model.state
        } else if visual.moved {
            1
        } else if marker.animation == CreatureAnimationState::Moving {
            // A blocked Move is not a walking pose, and inter-tick frames must
            // not restart idle while the displayed creature is still moving.
            0
        } else {
            clip(marker.animation)
        };
        if next != model.state {
            player.stop_all();
            player
                .play(model.clips[next])
                .repeat()
                .set_seek_time(marker.phase % 3.0);
            model.state = next;
        }
        let speed = if ux.settings.paused {
            0.0
        } else {
            ux.animation_speed
        };
        for (_, active) in player.playing_animations_mut() {
            if next == 1 {
                // Seeking prevents clock time, frame rate, and simulation speed
                // from advancing the feet a second time after world movement.
                active
                    .set_speed(0.0)
                    .set_seek_time(WALK_CLIP_START + visual.walk_seconds);
            } else {
                active.set_speed(speed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_feedback_deduplicates_events_freezes_on_pause_and_returns_to_sampled_pose() {
        let mut feedback = OutcomeFeedback::default();
        let sampled = Quat::from_rotation_y(0.3);
        assert!(outcome_rotation(
            sampled,
            ExpressionBone::Head,
            &feedback,
            CreatureAnimationState::Idle
        )
        .abs_diff_eq(sampled, 1e-6));
        feedback.observe(Some(10), Some(20));
        feedback.advance(0.15, false);
        let ingestion = feedback.ingestion_remaining;
        let speech = feedback.speech_remaining;
        feedback.observe(Some(10), Some(20));
        feedback.observe(Some(9), Some(19));
        assert_eq!(feedback.ingestion_remaining, ingestion);
        assert_eq!(feedback.speech_remaining, speech);
        feedback.advance(10.0, true);
        assert_eq!(feedback.ingestion_remaining, ingestion);
        assert_eq!(feedback.speech_remaining, speech);
        assert!(!outcome_rotation(
            sampled,
            ExpressionBone::Head,
            &feedback,
            CreatureAnimationState::Idle
        )
        .abs_diff_eq(sampled, 1e-6));
        assert_eq!(
            outcome_rotation(
                sampled,
                ExpressionBone::Head,
                &feedback,
                CreatureAnimationState::Sleeping
            ),
            sampled
        );
        feedback.advance(10.0, false);
        assert!(outcome_rotation(
            sampled,
            ExpressionBone::Head,
            &feedback,
            CreatureAnimationState::Idle
        )
        .abs_diff_eq(sampled, 1e-6));
        feedback.observe(Some(11), Some(21));
        assert_eq!(feedback.ingestion_remaining, INGESTION_FEEDBACK_SECONDS);
        assert_eq!(feedback.speech_remaining, SPEECH_FEEDBACK_SECONDS);
    }

    #[test]
    fn ingestion_feedback_requires_a_matching_sealed_consumption_receipt() {
        let organism = OrganismId(1);
        let eat = alife_world::HeadlessWorldCommand::eat(organism, WorldEntityId(2)).unwrap();
        let command =
            alife_core::channel_command_for_action(alife_core::MotorChannel::Manipulation, &eat)
                .unwrap();
        let receipt = alife_world::HeadlessMotorChannelReceipt {
            command: command.clone(),
            observation: None,
            physical: alife_core::PhysicalActionOutcome {
                contact: alife_core::PhysicalContactKind::Consumed,
                target_entity: Some(WorldEntityId(2)),
                displacement: alife_core::Vec3f::ZERO,
                collision_normal: None,
                energy_cost: alife_core::NormalizedScalar::new(0.02).unwrap(),
            },
        };
        let mut summary = LiveBrainTickSummary {
            schema: crate::G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: crate::G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id: organism,
            tick_before: Tick::new(9),
            tick_after: Tick::new(10),
            world_tick_before: Tick::new(9),
            world_tick_after: Tick::new(10),
            status: alife_core::BrainTickStatus::SafeIdle,
            selected_action_kind: Some(alife_core::ActionKind::Move),
            selected_action_id: None,
            target_entity: None,
            patch_sealed: true,
            patch_sequence_id: Some(10),
            patch_success: Some(false),
            physical_contact: Some(alife_core::PhysicalContactKind::Blocked),
            action_failure: None,
            motor_execution: Some(crate::LiveMotorExecutionTrace {
                requested_channels: vec![command],
                channel_receipts: vec![receipt.clone()],
            }),
            sealed_patch_count: 1,
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: 0,
            last_diagnostic: None,
            causal_stages: Vec::new(),
        };
        // Another channel may fail: actual ingestion still happened.
        assert_eq!(confirmed_ingestion_tick(&summary, organism, 10), Some(10));
        assert_eq!(confirmed_ingestion_tick(&summary, OrganismId(3), 10), None);
        assert_eq!(confirmed_ingestion_tick(&summary, organism, 9), None);
        summary.patch_sealed = false;
        assert_eq!(confirmed_ingestion_tick(&summary, organism, 10), None);
        summary.patch_sealed = true;
        summary
            .motor_execution
            .as_mut()
            .unwrap()
            .channel_receipts
            .clear();
        assert_eq!(confirmed_ingestion_tick(&summary, organism, 10), None);
        let mut blocked = receipt.clone();
        blocked.physical.contact = alife_core::PhysicalContactKind::Blocked;
        assert!(!confirmed_consumption(&blocked));
        let mut grab = receipt;
        grab.command.primitive = alife_world::HeadlessActionIds::GRAB;
        assert!(!confirmed_consumption(&grab));
    }

    #[test]
    fn speech_feedback_requires_an_active_emitted_creature_utterance_from_the_same_individual() {
        let mut utterance = alife_world::AudibleUtterance {
            utterance_id: alife_core::UtteranceId::new(5).unwrap(),
            source_kind: alife_core::UtteranceSourceKind::Creature,
            speaker_id: Some(OrganismId(1)),
            addressee: None,
            source_position: alife_core::Vec3f::ZERO,
            tokens: vec![alife_core::LanguageTokenId::new(7).unwrap()],
            confidence: alife_core::Confidence::new(1.0).unwrap(),
            teacher_channel: None,
            emitted_tick: Tick::new(10),
            expires_after_tick: Tick::new(11),
        };
        let event = |value: &alife_world::AudibleUtterance, organism, tick| {
            emitted_utterance_id(std::slice::from_ref(value), organism, tick)
        };
        assert_eq!(event(&utterance, OrganismId(1), 10), Some(5));
        assert_eq!(event(&utterance, OrganismId(2), 10), None);
        assert_eq!(event(&utterance, OrganismId(1), 9), None);
        assert_eq!(event(&utterance, OrganismId(1), 12), None);
        utterance.source_kind = alife_core::UtteranceSourceKind::Player;
        assert_eq!(event(&utterance, OrganismId(1), 10), None);
        utterance.source_kind = alife_core::UtteranceSourceKind::Teacher;
        assert_eq!(event(&utterance, OrganismId(1), 10), None);
    }

    #[test]
    fn expression_preserves_neutral_and_authored_sleep_poses() {
        let sampled = Quat::from_rotation_y(0.3) * Quat::from_rotation_x(0.2);
        for part in [
            ExpressionBone::Head,
            ExpressionBone::Ear(1.0),
            ExpressionBone::Lid(1.07),
        ] {
            assert_eq!(
                expression_rotation(
                    sampled,
                    Quat::IDENTITY,
                    part,
                    CreatureExpressionState::Neutral,
                    CreatureAnimationState::Idle
                ),
                sampled
            );
            for expression in [
                CreatureExpressionState::Tired,
                CreatureExpressionState::Pained,
                CreatureExpressionState::Afraid,
            ] {
                assert_eq!(
                    expression_rotation(
                        sampled,
                        Quat::IDENTITY,
                        part,
                        expression,
                        CreatureAnimationState::Sleeping
                    ),
                    sampled
                );
            }
        }
    }

    #[test]
    fn tired_expression_closes_open_lids_without_overclosing_authored_blinks() {
        let bind = Quat::from_rotation_x(-0.02839);
        for closed in [1.07, -1.02] {
            let part = ExpressionBone::Lid(closed);
            let tired = expression_rotation(
                bind,
                bind,
                part,
                CreatureExpressionState::Tired,
                CreatureAnimationState::Idle,
            );
            assert!(tired.abs_diff_eq(bind * Quat::from_rotation_x(closed * 0.28), 1e-6));
            for amount in [0.5, 1.0] {
                let blink = bind * Quat::from_rotation_x(closed * amount);
                assert_eq!(
                    expression_rotation(
                        blink,
                        bind,
                        part,
                        CreatureExpressionState::Tired,
                        CreatureAnimationState::Idle
                    ),
                    blink
                );
            }
        }
    }

    #[test]
    fn expression_posture_uses_each_fresh_animation_sample_and_recovers_to_neutral() {
        let frames = [Quat::from_rotation_z(0.04), Quat::from_rotation_z(-0.03)];
        for sampled in frames {
            let projected = expression_rotation(
                sampled,
                Quat::IDENTITY,
                ExpressionBone::Head,
                CreatureExpressionState::Pained,
                CreatureAnimationState::Idle,
            );
            let nod = sampled.inverse() * projected;
            assert!(nod.abs_diff_eq(Quat::from_rotation_x(0.18), 1e-6));
            assert_eq!(
                expression_rotation(
                    sampled,
                    Quat::IDENTITY,
                    ExpressionBone::Head,
                    CreatureExpressionState::Neutral,
                    CreatureAnimationState::Idle
                ),
                sampled
            );
        }
        let left = expression_rotation(
            Quat::IDENTITY,
            Quat::IDENTITY,
            ExpressionBone::Ear(1.0),
            CreatureExpressionState::Pained,
            CreatureAnimationState::Idle,
        );
        let right = expression_rotation(
            Quat::IDENTITY,
            Quat::IDENTITY,
            ExpressionBone::Ear(-1.0),
            CreatureExpressionState::Pained,
            CreatureAnimationState::Idle,
        );
        assert!(left.abs_diff_eq(right.inverse(), 1e-6));
        let afraid = expression_rotation(
            Quat::IDENTITY,
            Quat::IDENTITY,
            ExpressionBone::Ear(1.0),
            CreatureExpressionState::Afraid,
            CreatureAnimationState::Idle,
        );
        assert!(afraid.abs_diff_eq(Quat::from_rotation_x(-0.35), 1e-6));
    }

    #[test]
    fn world_heading_and_chosen_gaze_replace_animation_yaw_without_drift() {
        for yaw in [0.0, 0.7, -1.2, std::f32::consts::PI] {
            let forward = body_rotation(yaw) * Vec3::Z;
            assert!(forward.abs_diff_eq(Vec3::new(yaw.cos(), 0.0, yaw.sin()), 1e-6));
            let chosen = head_rotation(Quat::from_rotation_y(0.15), yaw);
            let gaze = body_rotation(0.0) * chosen * Vec3::Z;
            assert!(gaze.abs_diff_eq(Vec3::new(yaw.cos(), 0.0, yaw.sin()), 1e-6));
            let nod = Quat::from_rotation_x(0.35) * Quat::from_rotation_y(0.15);
            let once = head_rotation(nod, yaw);
            let twice = head_rotation(once, yaw);
            assert!(once.abs_diff_eq(twice, 1e-6));
        }
    }

    #[test]
    fn one_step_tracks_authored_foot_travel_at_each_inherited_size() {
        for mass in 0..alife_world::CREATURE_APPEARANCE_GENE_BUCKETS {
            let appearance = CreatureAppearanceGenome {
                body_mass_trait: mass,
                ..Default::default()
            };
            let forward_scale = scale(appearance).z;
            let seconds = advance_walk(0.72 * forward_scale, forward_scale, 0.0);
            assert!((seconds - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn walk_distance_is_frame_partition_independent_and_stops_without_motion() {
        let scale = 0.7;
        let distance = 1.3;
        let once = advance_walk(distance, scale, 0.0);
        let mut partitioned = 0.0;
        for _ in 0..120 {
            partitioned = advance_walk(distance / 120.0, scale, partitioned);
        }
        assert!((once - partitioned).abs() < 1e-5);
        assert_eq!(advance_walk(0.0, scale, partitioned), partitioned);
        assert!(advance_walk(100.0, scale, partitioned) < WALK_CLIP_SECONDS);
    }
    #[test]
    fn renderless_preflight_retains_the_organism_root_without_an_asset_server() {
        let mut world = World::new();
        let root = world.spawn(Transform::default()).id();
        spawn(&mut world, root, CreatureAppearanceGenome::default());
        assert!(world.get::<Transform>(root).is_some());
        assert!(world.get::<HearthlingVisual>(root).is_none());
    }
    #[test]
    fn sleep_and_locomotion_use_distinct_authored_clips() {
        assert_ne!(
            clip(CreatureAnimationState::Sleeping),
            clip(CreatureAnimationState::Idle)
        );
        assert_ne!(
            clip(CreatureAnimationState::Moving),
            clip(CreatureAnimationState::Idle)
        );
    }
}
