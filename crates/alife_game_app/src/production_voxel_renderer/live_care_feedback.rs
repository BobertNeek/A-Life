//! The creature's own word for the care it just felt, shown above it.
//!
//! Visual style is the language: the response reads in the creature, not in
//! a label. A fresh player-care receipt makes the creature answer twice —
//! once in its posture (a nod for praise/pet, a flinch for scold/slap; see
//! `hearthling::OutcomeFeedback`) and once in its own word, in a bubble
//! above it. The word is the creature's reading of the moment — what it
//! thinks just happened, in its own 18-word vocabulary — and the posture
//! carries whether it was good or bad. Together they are the "what", not
//! just the "that something happened".
//!
//! Timing is the strictest rule: the word fires only for a receipt newer
//! than the last one shown, lives about three seconds, and then clears. It
//! never floats free of the moment the dose was consumed, and it never
//! re-appears for an old receipt. No receipt means no word at all —
//! unknown is never rendered as a "nothing felt" bubble, a nod, or a
//! flinch. An old save with no recorded receipt reads exactly like a
//! creature never yet cared for, and both show nothing.
use super::*;
use bevy::prelude::{Text2d, TextColor, TextFont};

// One word per individual, bounded independently of population scale.
const MAX_CUES: usize = 8;
// Long enough to read, short enough to stay tied to the moment.
const CARE_WORD_SECONDS: f32 = 3.0;
// Above the sleep rings and above the creature's own speech bubble.
const CARE_WORD_HEIGHT: f32 = 2.10;

#[derive(Component)]
pub(super) struct CareCue {
    root: Entity,
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    shown_tick: u64,
    seconds: f32,
}

#[derive(Component)]
pub(super) struct CareWord;

/// The creature's own word for the receipt kind, drawn from the 18-word
/// grounded codebook (`alife_core::BASIC_VOCABULARY_V1`). The word names
/// what the creature thinks just happened, not a label about the action:
///
/// - pet (positive physical) -> "play": the creature reads the touch as
///   social play, and plays back.
/// - praise (positive social) -> "get": the creature reads the praise as
///   having received a reward. There is no "reward" word in the codebook;
///   "get" is the closest verb for receiving one.
/// - scold (negative social) -> "look": the creature turns its attention
///   to the one correcting it — the Black & White judgement moment, where
///   the creature looks at you. The weakest mapping; revisit if the
///   codebook gains an approval/disapproval word.
/// - slap (negative physical) -> "retreat": the creature reads the hit as
///   a reason to move away.
/// A receipt may carry several kinds at once (a pet and a praise queued
/// before one tick, a scold with a slap). The word comes from the receipt's
/// display kind: negative dominates, and among positives a pet outranks
/// praise, so pet-and-praise still says "play". That rule is interpretive,
/// not a measurement — the debug readout (`status_hud::debug_care_line`)
/// keeps the exact set.
///
/// Pet and a pet-and-praise set collide on "play"; the nod is the same for
/// both. That is honest — the creature's vocabulary cannot say more yet.
pub(super) fn care_word(kind: alife_world::PlayerCareKind) -> &'static str {
    match kind {
        alife_world::PlayerCareKind::Pet => "play",
        alife_world::PlayerCareKind::Praise => "get",
        alife_world::PlayerCareKind::Scold => "look",
        alife_world::PlayerCareKind::Slap => "retreat",
    }
}

/// The word's colour is the valence, read before the word is: warm green
/// for the positive kinds, muted red for the negative ones. Distinct from
/// the dark text of the creature's own speech bubble.
pub(super) fn care_color(kind: alife_world::PlayerCareKind) -> Color {
    if kind.is_negative() {
        Color::srgb(0.85, 0.42, 0.35)
    } else {
        Color::srgb(0.45, 0.78, 0.38)
    }
}

fn cared_row(
    root: &ProductionCreatureAssemblyRoot,
    visual: &Fvr04ProductionCreatureVisualMarker,
    object: Option<&alife_world::WorldObject>,
    row: Option<&WorldOrganismPresentationRow>,
) -> Option<alife_world::PlayerCareReceipt> {
    let object = object?;
    let row = row?;
    if visual.stable_id == root.stable_id
        && visual.organism_id == root.organism_id
        && object.id == root.stable_id
        && object.kind == WorldObjectKind::Agent
        && object.organism_id == Some(root.organism_id)
        && row.world_entity_id == root.stable_id
        && row.organism_id == root.organism_id
        && row.object.id == root.stable_id
        && row.object.kind == WorldObjectKind::Agent
        && row.object.organism_id == Some(root.organism_id)
        && row.lifecycle.is_alive()
    {
        match &row.last_player_care {
            alife_world::PresentationEvidence::Observed { value, .. } => Some(*value),
            // Unknown is not a receipt: no word, no nod, no "nothing felt".
            alife_world::PresentationEvidence::Unknown => None,
        }
    } else {
        None
    }
}

fn cared_receipt(
    frame: &LiveBrainPresentationFrame,
    root: &ProductionCreatureAssemblyRoot,
    visual: &Fvr04ProductionCreatureVisualMarker,
) -> Option<alife_world::PlayerCareReceipt> {
    cared_row(
        root,
        visual,
        frame.object(root.stable_id),
        frame.organism(root.stable_id),
    )
}

fn cue_transform(root: &Transform, camera_rotation: Quat, seconds: f32) -> Transform {
    let breath = (seconds * 1.3).sin();
    // Follow the displayed pose and terrain height, including motion between ticks.
    // Keep the word above the silhouette, facing the player at either camera angle.
    let offset = camera_rotation * Vec3::new(0.32, 0.06 + breath * 0.025, 0.0);
    Transform {
        translation: Vec3::new(0.0, CARE_WORD_HEIGHT, 0.0)
            + root.rotation.inverse() * offset / root.scale,
        // Cancel nonuniform root scale before the child turns toward the camera.
        // Combining the inverse scale and billboard rotation here would shear it.
        rotation: Quat::IDENTITY,
        scale: root.scale.recip(),
    }
}

fn word_transform(root_rotation: Quat, camera_rotation: Quat) -> Transform {
    Transform {
        translation: Vec3::ZERO,
        rotation: root_rotation.inverse() * camera_rotation,
        scale: Vec3::ONE,
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Bevy injects independent ECS parameters."
)]
pub(super) fn sync(
    mut commands: Commands,
    time: Res<Time>,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    roots: Query<
        (
            Entity,
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        ),
        Without<CareCue>,
    >,
    cameras: Query<&Transform, (With<Fvr03ProductionVoxelCamera>, Without<CareCue>)>,
    mut cues: Query<
        (Entity, &mut CareCue, &mut Transform, &Children),
        Without<ProductionCreatureAssemblyRoot>,
    >,
    mut words: Query<
        (&CareWord, &mut Transform),
        (
            Without<CareCue>,
            Without<ProductionCreatureAssemblyRoot>,
            Without<Fvr03ProductionVoxelCamera>,
        ),
    >,
    mut shown: Local<BTreeMap<u64, u64>>,
) {
    let paused = ux.as_ref().is_some_and(|ux| ux.settings.paused);
    let speed = ux.as_ref().map_or(1.0, |ux| ux.animation_speed);
    let camera_rotation = cameras.iter().next().map_or(Quat::IDENTITY, |t| t.rotation);
    // Reconcile even without a new world tick: a superseded receipt must
    // clear its word, and a held/interpolated root or moving camera still
    // changes its displayed pose.
    let mut eligible = BTreeMap::new();
    if let Some(frame) = frame.as_ref() {
        for (entity, root, visual, transform) in &roots {
            if !transform.scale.is_finite() || transform.scale.abs().min_element() <= f32::EPSILON {
                continue;
            }
            if let Some(receipt) = cared_receipt(&frame.current, root, visual) {
                eligible.insert(
                    (root.stable_id.raw(), root.organism_id.raw()),
                    (entity, *transform, receipt),
                );
            }
        }
    }
    // Persistent historical receipts must not consume the visible budget.
    // Retain an active word or a strictly newer receipt before applying the
    // cap, otherwise eight previously cared-for low IDs starve every later ID.
    let active: BTreeSet<_> = cues
        .iter()
        .map(|(_, cue, _, _)| {
            (
                cue.root,
                cue.stable_id.raw(),
                cue.organism_id.raw(),
                cue.shown_tick,
            )
        })
        .collect();
    eligible.retain(|&(stable_id, organism_id), (root, _, receipt)| {
        let tick = receipt.applied_tick.raw();
        active.contains(&(*root, stable_id, organism_id, tick))
            || shown.get(&organism_id).is_none_or(|last| tick > *last)
    });
    // Stable IDs, rather than ECS iteration order, decide the bounded visible set.
    let mut eligible = eligible
        .into_iter()
        .take(MAX_CUES)
        .collect::<BTreeMap<_, _>>();
    for (entity, mut cue, mut transform, children) in &mut cues {
        let key = (cue.stable_id.raw(), cue.organism_id.raw());
        // Copy the receipt out of the map: the reconcile below mutates the
        // map (despawn + remove), so no borrow of it may survive the lookup.
        let showing = eligible
            .get(&key)
            .filter(|(root, _, _)| *root == cue.root)
            .map(|(_, pose, receipt)| (*pose, *receipt));
        if let Some((pose, receipt)) = showing {
            if cue.shown_tick == receipt.applied_tick.raw() {
                cue.seconds =
                    advance_fvr04_animation_phase(cue.seconds, time.delta_secs(), speed, paused);
                if cue.seconds >= CARE_WORD_SECONDS {
                    commands.entity(entity).despawn();
                    eligible.remove(&key);
                    continue;
                }
                let next = cue_transform(&pose, camera_rotation, cue.seconds);
                if *transform != next {
                    *transform = next;
                }
                for child in children {
                    if let Ok((_, mut transform)) = words.get_mut(*child) {
                        let next = word_transform(pose.rotation, camera_rotation);
                        if *transform != next {
                            *transform = next;
                        }
                    }
                }
                eligible.remove(&key);
            } else {
                // A newer receipt superseded this one: clear the old word so
                // the fresh one can take its place below.
                commands.entity(entity).despawn();
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    if eligible.is_empty() {
        return;
    }
    for ((stable_id, organism_id), (root, pose, receipt)) in eligible {
        // The word fires once per receipt tick. An old receipt never
        // re-appears: the freshness map remembers what was already shown.
        if shown
            .get(&organism_id)
            .is_some_and(|tick| *tick >= receipt.applied_tick.raw())
        {
            continue;
        }
        shown.insert(organism_id, receipt.applied_tick.raw());
        // The word and colour come from the display kind: one kind per
        // moment, chosen by the receipt's rule (negative dominates, pet
        // over praise). An empty set has no display kind and shows nothing.
        let Some(display) = receipt.display_kind() else {
            continue;
        };
        commands
            .spawn((
                Name::new("Live care response: creature word"),
                CareCue {
                    root,
                    stable_id: WorldEntityId(stable_id),
                    organism_id: OrganismId(organism_id),
                    shown_tick: receipt.applied_tick.raw(),
                    seconds: 0.0,
                },
                cue_transform(&pose, camera_rotation, 0.0),
                Visibility::Inherited,
                bevy::picking::Pickable::IGNORE,
                ChildOf(root),
            ))
            .with_children(|parent| {
                parent.spawn((
                    CareWord,
                    Text2d::new(care_word(display)),
                    TextFont {
                        font_size: 18.0,
                        ..Default::default()
                    },
                    TextColor(care_color(display)),
                    word_transform(pose.rotation, camera_rotation),
                    bevy::picking::Pickable::IGNORE,
                ));
            });
    }
}

#[cfg(test)]
mod tests;
