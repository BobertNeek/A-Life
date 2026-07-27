//! Player speech and cross-save lineage selection for the production frontend.

use std::path::{Path, PathBuf};

use alife_archive::{LineageLibrary, LineageLibraryConfig};
use alife_core::{
    ArchiveCheckpointDisposition, ArchiveCheckpointRetention, Blake3Digest, FounderMode,
    FounderSelection, LanguageCodebookV1, MetricReading, OrganismId, PassiveLifeStatistics,
    PassiveMetricKind, SpeechTranslationInput, SpeechTranslationReceipt, SpeechTranslationRequest,
    SurfaceTokenBinding, UtteranceId, UtteranceSourceKind, Validate, Vec3f,
};
use alife_semantic::{BoundedSpeechTranslator, TranslationAssistance};
use alife_world::{persistence::PortableSaveFile, StableVoxelRefKind, WorldObjectKind};
use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::{
        App, BackgroundColor, ButtonInput, Color, Component, GlobalZIndex, KeyCode, MessageReader,
        Name, Node, NonSend, NonSendMut, ParamSet, PositionType, Res, ResMut, Resource, Text,
        Text2d, TextColor, TextFont, Transform, Update, Val, Visibility, With,
    },
};

use crate::bevy_shell::ProductionGpuBrainRuntimeResource;
use crate::{
    materialize_founder_gpu_states, Fvr03ProductionVoxelSceneResource,
    Fvr03ProductionVoxelSelectionResource, Fvr04ProductionCreatureSceneResource,
    Fvr05ProductionUxStateResource, GameAppShellError, ProductionVoxelLaunchSummary,
};

const MAX_TYPED_CHARS: usize = 512;
const MAX_COHORT_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NarrationDisplayFrequency {
    Off,
    Sparse,
    Normal,
    Frequent,
}

impl NarrationDisplayFrequency {
    const fn next(self) -> Self {
        match self {
            Self::Off => Self::Sparse,
            Self::Sparse => Self::Normal,
            Self::Normal => Self::Frequent,
            Self::Frequent => Self::Off,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Sparse => "sparse",
            Self::Normal => "normal",
            Self::Frequent => "frequent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineageFilter {
    All,
    GeneticArchives,
    LearnedCheckpoints,
}

impl LineageFilter {
    const fn next(self) -> Self {
        match self {
            Self::All => Self::GeneticArchives,
            Self::GeneticArchives => Self::LearnedCheckpoints,
            Self::LearnedCheckpoints => Self::All,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::All => "all runs",
            Self::GeneticArchives => "genetic archives",
            Self::LearnedCheckpoints => "learned checkpoints",
        }
    }
}

#[derive(Debug, Clone)]
struct LineageUiRow {
    digest: Blake3Digest,
    source_run_id: String,
    organism_id: OrganismId,
    deceased: bool,
    checkpoint: Option<(Blake3Digest, ArchiveCheckpointRetention)>,
    survival: String,
    problem_solving: String,
    language_unaided: String,
    language_assisted: String,
    overall_q16: Option<u32>,
}

#[derive(Debug, Clone, Resource)]
pub struct ProductionConversationLineageUiState {
    input_open: bool,
    input: String,
    address_selected: bool,
    muted: bool,
    narration: NarrationDisplayFrequency,
    translation_enabled: bool,
    raw_tokens_visible: bool,
    slm_off: bool,
    developer_overlay: bool,
    bindings: Vec<SurfaceTokenBinding>,
    last_player_receipt: Option<SpeechTranslationReceipt>,
    last_creature_receipt: Option<SpeechTranslationReceipt>,
    last_creature_utterance_id: Option<UtteranceId>,
    last_creature_speaker: Option<OrganismId>,
    creature_utterance_active: bool,
    status: String,
    lineage_root: PathBuf,
    lineage_open: bool,
    lineage_filter: LineageFilter,
    lineage_rows: Vec<LineageUiRow>,
    lineage_cursor: usize,
    pending_founder_mode: FounderMode,
    cohort: Vec<FounderSelection>,
}

impl ProductionConversationLineageUiState {
    fn new(summary: &ProductionVoxelLaunchSummary) -> Self {
        let lineage_root = default_lineage_root();
        let (lineage_rows, status) = load_lineage_rows(&lineage_root)
            .map(|rows| {
                let status = format!("Lineage Library: {} current archives", rows.len());
                (rows, status)
            })
            .unwrap_or_else(|error| (Vec::new(), format!("Lineage Library unavailable: {error}")));
        Self {
            input_open: false,
            input: String::new(),
            address_selected: false,
            muted: false,
            narration: NarrationDisplayFrequency::Normal,
            translation_enabled: true,
            raw_tokens_visible: summary.developer_overlay,
            slm_off: true,
            developer_overlay: summary.developer_overlay,
            bindings: Vec::new(),
            last_player_receipt: None,
            last_creature_receipt: None,
            last_creature_utterance_id: None,
            last_creature_speaker: None,
            creature_utterance_active: false,
            status,
            lineage_root,
            lineage_open: false,
            lineage_filter: LineageFilter::All,
            lineage_rows,
            lineage_cursor: 0,
            pending_founder_mode: FounderMode::GeneticFounder,
            cohort: Vec::new(),
        }
    }

    pub const fn blocks_world_shortcuts(&self) -> bool {
        self.input_open || self.lineage_open
    }

    pub(crate) fn prepare_recorded_speech_capture(&mut self) {
        self.lineage_open = false;
        self.input_open = true;
        self.address_selected = true;
        self.input = "what are you doing?".to_string();
        self.status = "Player speech is spatial perception at the Hand".to_string();
    }

    pub(crate) fn prepare_recorded_lineage_capture(&mut self) {
        self.input_open = false;
        self.lineage_open = true;
        self.status = "Choose archived founders from this run or earlier simulations".to_string();
    }

    pub(crate) fn clear_recorded_capture(&mut self) {
        self.input_open = false;
        self.lineage_open = false;
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.lineage_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| match self.lineage_filter {
                LineageFilter::All | LineageFilter::GeneticArchives => Some(index),
                LineageFilter::LearnedCheckpoints if row.checkpoint.is_some() => Some(index),
                LineageFilter::LearnedCheckpoints => None,
            })
            .collect()
    }

    fn current_row(&self) -> Option<&LineageUiRow> {
        let filtered = self.filtered_indices();
        filtered
            .get(self.lineage_cursor.min(filtered.len().saturating_sub(1)))
            .and_then(|index| self.lineage_rows.get(*index))
    }
}

#[derive(Component)]
struct ProductionSpeechEntryPanel;
#[derive(Component)]
struct ProductionSpeechControlsPanel;
#[derive(Component)]
struct ProductionSpeechDeveloperPanel;
#[derive(Component)]
struct ProductionCreatureSpeechBubble;
#[derive(Component)]
struct ProductionLineageLibraryPanel;

pub fn install_production_conversation_lineage_ui(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
) {
    app.insert_resource(ProductionConversationLineageUiState::new(summary));
    spawn_ui(app);
    app.add_systems(
        Update,
        (
            handle_production_conversation_lineage_input,
            refresh_creature_speech_receipt,
            sync_production_conversation_lineage_ui,
        ),
    );
}

fn spawn_ui(app: &mut App) {
    app.world_mut().spawn((
        Name::new("A-Life player Hand speech entry"),
        Text::new("Speak near the Hand"),
        TextFont {
            font_size: 18.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.96, 0.93, 0.76)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(27.0),
            right: Val::Percent(27.0),
            bottom: Val::Px(74.0),
            min_height: Val::Px(58.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.025, 0.040, 0.022, 0.92)),
        GlobalZIndex(80),
        Visibility::Hidden,
        ProductionSpeechEntryPanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life conversation controls"),
        Text::new("Enter speak | Y Lineage Library"),
        TextFont {
            font_size: 12.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.86, 0.93, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            bottom: Val::Px(12.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.015, 0.026, 0.018, 0.76)),
        GlobalZIndex(70),
        ProductionSpeechControlsPanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life developer speech inspector"),
        Text::new("Speech Inspector [DEV ONLY]"),
        TextFont {
            font_size: 13.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.88, 0.96, 0.80)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(54.0),
            right: Val::Px(18.0),
            width: Val::Px(340.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.010, 0.025, 0.014, 0.92)),
        GlobalZIndex(90),
        Visibility::Hidden,
        ProductionSpeechDeveloperPanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life literal creature speech bubble"),
        Text2d::new(""),
        TextFont {
            font_size: 18.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.18, 0.15, 0.09)),
        Transform::from_xyz(0.0, 3.0, 0.0),
        Visibility::Hidden,
        ProductionCreatureSpeechBubble,
    ));
    app.world_mut().spawn((
        Name::new("A-Life Lineage Library"),
        Text::new("Lineage Library"),
        TextFont {
            font_size: 15.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.94, 0.91, 0.74)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(5.0),
            left: Val::Percent(7.0),
            right: Val::Percent(7.0),
            bottom: Val::Percent(7.0),
            padding: bevy::ui::UiRect::all(Val::Px(18.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.014, 0.028, 0.020, 0.97)),
        GlobalZIndex(100),
        Visibility::Hidden,
        ProductionLineageLibraryPanel,
    ));
}

fn handle_production_conversation_lineage_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut key_messages: MessageReader<KeyboardInput>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    ux: Res<Fvr05ProductionUxStateResource>,
    mut runtime: NonSendMut<ProductionGpuBrainRuntimeResource>,
    mut state: ResMut<ProductionConversationLineageUiState>,
) {
    if state.lineage_open {
        handle_lineage_input(&keyboard, &ux, &mut state);
        return;
    }
    if !state.input_open {
        if keyboard.just_pressed(KeyCode::Enter) {
            state.input_open = true;
            state.input.clear();
            state.status = "Player speech input opened at the Hand".to_string();
        } else if keyboard.just_pressed(KeyCode::KeyY) {
            state.lineage_open = true;
            state.input_open = false;
            state.status = "Lineage Library opened".to_string();
        } else {
            handle_speech_setting_keys(&keyboard, &mut state);
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        state.input_open = false;
        state.input.clear();
        state.status = "Player speech cancelled".to_string();
        return;
    }
    if keyboard.just_pressed(KeyCode::Tab) {
        state.address_selected = !state.address_selected;
    }
    if keyboard.just_pressed(KeyCode::Backspace) {
        state.input.pop();
    }
    for message in key_messages.read() {
        if message.state != ButtonState::Pressed {
            continue;
        }
        if let Some(text) = &message.text {
            for character in text.chars().filter(|character| !character.is_control()) {
                if state.input.chars().count() < MAX_TYPED_CHARS {
                    state.input.push(character);
                }
            }
        }
    }
    if keyboard.just_pressed(KeyCode::Enter) && !state.input.trim().is_empty() {
        if let Err(error) =
            send_player_speech(&selection, &creatures, &mut runtime.runtime, &mut state)
        {
            state.status = format!("Player speech failed: {error}");
        }
    }
}

fn handle_speech_setting_keys(
    keyboard: &ButtonInput<KeyCode>,
    state: &mut ProductionConversationLineageUiState,
) {
    if keyboard.just_pressed(KeyCode::F6) {
        state.muted = !state.muted;
    }
    if keyboard.just_pressed(KeyCode::F7) {
        state.narration = state.narration.next();
    }
    if keyboard.just_pressed(KeyCode::F8) {
        state.translation_enabled = !state.translation_enabled;
    }
    if keyboard.just_pressed(KeyCode::F10) {
        state.raw_tokens_visible = !state.raw_tokens_visible;
    }
    if keyboard.just_pressed(KeyCode::F11) {
        state.slm_off = !state.slm_off;
    }
}

fn send_player_speech(
    selection: &Fvr03ProductionVoxelSelectionResource,
    creatures: &Fvr04ProductionCreatureSceneResource,
    runtime: &mut crate::GpuLiveBrainRuntime,
    state: &mut ProductionConversationLineageUiState,
) -> Result<(), GameAppShellError> {
    let text = state.input.trim().to_string();
    let snapshot = runtime.world_snapshot();
    let explicit_addressee = named_addressee(&text, &snapshot);
    let selected_addressee = state
        .address_selected
        .then(|| selected_organism(selection, creatures))
        .flatten();
    let addressee = explicit_addressee.or(selected_addressee);
    let source_position = selection
        .selected
        .and_then(|selected| selected.tile)
        .map_or(Vec3f::ZERO, |tile| {
            Vec3f::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5)
        });
    let translator = BoundedSpeechTranslator::new(
        "alife-bounded-speech-ui-v1",
        if state.slm_off {
            TranslationAssistance::Disabled
        } else {
            TranslationAssistance::SlmAssisted
        },
    )?;
    let request = SpeechTranslationRequest::try_new(
        UtteranceId::new(1)?,
        addressee,
        SpeechTranslationInput::PlayerText { text: text.clone() },
        state.bindings.clone(),
    )?;
    let mut receipt = translator.translate(&request)?;
    let audible =
        runtime.emit_player_tokens(addressee, source_position, receipt.literal_tokens.clone())?;
    receipt.utterance_id = audible.utterance_id;
    receipt.validate_contract()?;
    for novel in &receipt.novel_tokens {
        if state.bindings.len() >= alife_core::SPEECH_TRANSLATION_MAX_BINDINGS {
            break;
        }
        if !state
            .bindings
            .iter()
            .any(|binding| binding.token == novel.token)
        {
            state
                .bindings
                .push(SurfaceTokenBinding::try_new(&novel.surface, novel.token)?);
        }
    }
    state.last_player_receipt = Some(receipt);
    state.input.clear();
    state.input_open = false;
    state.status = match addressee {
        Some(organism) => format!("Spoke spatially to creature {}", organism.raw()),
        None => "Spoke spatially to every creature in hearing range".to_string(),
    };
    Ok(())
}

fn named_addressee(text: &str, world: &alife_world::HeadlessWorld) -> Option<OrganismId> {
    let normalized = text.trim().to_ascii_lowercase();
    world
        .object_snapshots()
        .into_iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .filter_map(|object| {
            let organism = object.organism_id?;
            let labels = [
                object.label.to_ascii_lowercase(),
                format!("creature {}", organism.raw()),
            ];
            labels
                .into_iter()
                .any(|label| {
                    normalized == label
                        || normalized.strip_prefix(&label).is_some_and(|rest| {
                            rest.chars()
                                .next()
                                .is_some_and(|character| matches!(character, ',' | ':' | ' '))
                        })
                })
                .then_some(organism)
        })
        .next()
}

fn selected_organism(
    selection: &Fvr03ProductionVoxelSelectionResource,
    creatures: &Fvr04ProductionCreatureSceneResource,
) -> Option<OrganismId> {
    let selected = selection.selected?;
    if selected.kind != StableVoxelRefKind::Creature {
        return None;
    }
    creatures
        .sample_for_stable_id(selected.stable_id?)
        .map(|sample| sample.organism_id)
}

fn refresh_creature_speech_receipt(
    runtime: NonSend<ProductionGpuBrainRuntimeResource>,
    mut state: ResMut<ProductionConversationLineageUiState>,
) {
    let Some(utterance) = runtime
        .runtime
        .active_utterances()
        .into_iter()
        .filter(|utterance| utterance.source_kind == UtteranceSourceKind::Creature)
        .max_by_key(|utterance| utterance.utterance_id.raw())
    else {
        state.creature_utterance_active = false;
        return;
    };
    state.creature_utterance_active = true;
    if state.last_creature_utterance_id == Some(utterance.utterance_id) {
        return;
    }
    let assistance = if state.slm_off {
        TranslationAssistance::Disabled
    } else {
        TranslationAssistance::SlmAssisted
    };
    let receipt = BoundedSpeechTranslator::new("alife-bounded-speech-ui-v1", assistance).and_then(
        |translator| {
            SpeechTranslationRequest::try_new(
                utterance.utterance_id,
                utterance.addressee,
                SpeechTranslationInput::CreatureTokens {
                    tokens: utterance.tokens.clone(),
                },
                state.bindings.clone(),
            )
            .and_then(|request| translator.translate(&request))
        },
    );
    match receipt {
        Ok(receipt) => {
            state.last_creature_utterance_id = Some(utterance.utterance_id);
            state.last_creature_speaker = utterance.speaker_id;
            state.last_creature_receipt = Some(receipt);
        }
        Err(error) => state.status = format!("Creature speech translation failed: {error}"),
    }
}

fn handle_lineage_input(
    keyboard: &ButtonInput<KeyCode>,
    ux: &Fvr05ProductionUxStateResource,
    state: &mut ProductionConversationLineageUiState,
) {
    if keyboard.just_pressed(KeyCode::Escape) || keyboard.just_pressed(KeyCode::KeyY) {
        state.lineage_open = false;
        state.status = "Lineage Library closed".to_string();
        return;
    }
    if keyboard.just_pressed(KeyCode::Tab) {
        state.lineage_filter = state.lineage_filter.next();
        state.lineage_cursor = 0;
    }
    let count = state.filtered_indices().len();
    if keyboard.just_pressed(KeyCode::ArrowDown) && count != 0 {
        state.lineage_cursor = (state.lineage_cursor + 1) % count;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) && count != 0 {
        state.lineage_cursor = state.lineage_cursor.checked_sub(1).unwrap_or(count - 1);
    }
    if keyboard.just_pressed(KeyCode::KeyF) {
        let checkpoint = state.current_row().and_then(|row| row.checkpoint);
        state.pending_founder_mode = match state.pending_founder_mode {
            FounderMode::GeneticFounder => checkpoint.map_or(
                FounderMode::GeneticOffspring { mutation_seed: 1 },
                |(checkpoint_digest, _)| FounderMode::MindStateClone { checkpoint_digest },
            ),
            FounderMode::MindStateClone { .. } => FounderMode::GeneticOffspring {
                mutation_seed: checkpoint
                    .map(|(digest, _)| mutation_seed(digest))
                    .unwrap_or(1),
            },
            FounderMode::GeneticOffspring { .. } => FounderMode::GeneticFounder,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyA) {
        if let Some(row) = state.current_row() {
            let digest = row.digest;
            if state.cohort.len() < MAX_COHORT_SIZE
                && !state
                    .cohort
                    .iter()
                    .any(|selection| selection.source_manifest_digest == digest)
            {
                let mode = state.pending_founder_mode;
                state.cohort.push(FounderSelection {
                    source_manifest_digest: digest,
                    mode,
                });
                state.status = format!(
                    "Added creature to founder cohort ({}/{MAX_COHORT_SIZE})",
                    state.cohort.len()
                );
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyX) {
        state.cohort.pop();
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        match load_lineage_rows(&state.lineage_root) {
            Ok(rows) => {
                state.lineage_rows = rows;
                state.lineage_cursor = 0;
                state.status = "Lineage Library refreshed".to_string();
            }
            Err(error) => state.status = format!("Lineage refresh failed: {error}"),
        }
    }
    if keyboard.just_pressed(KeyCode::Enter) && !state.cohort.is_empty() {
        match create_founder_world(state, ux) {
            Ok(path) => state.status = format!("Created new founder world: {}", path.display()),
            Err(error) => state.status = format!("Founder world creation failed: {error}"),
        }
    }
}

fn create_founder_world(
    state: &ProductionConversationLineageUiState,
    ux: &Fvr05ProductionUxStateResource,
) -> Result<PathBuf, GameAppShellError> {
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&state.lineage_root))?;
    let source_save_path = PathBuf::from(&ux.settings.runtime_save_path);
    let mut base = PortableSaveFile::from_json_file(&source_save_path)?;
    let organism_ids = base
        .creatures
        .iter()
        .map(|creature| creature.organism_id)
        .collect::<Vec<_>>();
    let mut world = base.restore_headless_world()?;
    for organism in organism_ids {
        world.remove_organism(organism)?;
    }
    base.replace_headless_world_snapshot(&world)?;
    base.creatures.clear();
    let seed = base.deterministic_seed;
    let save_id = format!("founder-world-{seed:016x}");
    base.save_id = save_id.clone();
    let cohort = library.resolve_founder_cohort(&save_id, seed, &state.cohort)?;
    let asset_root = &ux.asset_root;
    let skeleton = library.create_new_save_from_founders(base, asset_root, &cohort)?;
    let backend = alife_gpu_backend::GpuClosedLoopBackend::new_required(
        alife_gpu_backend::GpuRuntimeProfile::production_v1(),
    )?;
    let completed = materialize_founder_gpu_states(backend, skeleton, asset_root, &cohort)?;
    let output = source_save_path
        .parent()
        .unwrap_or(asset_root)
        .join(format!("{save_id}.json"));
    completed.validate_with_asset_root(asset_root)?;
    completed.to_json_file(&output)?;
    Ok(output)
}

#[allow(clippy::type_complexity)]
fn sync_production_conversation_lineage_ui(
    state: Res<ProductionConversationLineageUiState>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    mut panels: ParamSet<(
        bevy::prelude::Query<(&mut Text, &mut Visibility), With<ProductionSpeechEntryPanel>>,
        bevy::prelude::Query<&mut Text, With<ProductionSpeechControlsPanel>>,
        bevy::prelude::Query<(&mut Text, &mut Visibility), With<ProductionSpeechDeveloperPanel>>,
        bevy::prelude::Query<
            (&mut Text2d, &mut Transform, &mut Visibility),
            With<ProductionCreatureSpeechBubble>,
        >,
        bevy::prelude::Query<(&mut Text, &mut Visibility), With<ProductionLineageLibraryPanel>>,
    )>,
) {
    for (mut text, mut visibility) in &mut panels.p0() {
        text.0 = format!(
            "Speak near the Hand  |  To: {}\n{}▌\nEnter send  Tab address selected  Esc cancel",
            if state.address_selected {
                "selected creature"
            } else {
                "broadcast / named addressee"
            },
            state.input
        );
        *visibility = if state.input_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut text in &mut panels.p1() {
        text.0 = format!(
            "Enter speak | Y Lineage Library | F6 mute:{} | F7 narration:{} | F8 translation:{} | F10 raw:{} | F11 SLM:{}\n{}",
            state.muted,
            state.narration.label(),
            state.translation_enabled,
            state.raw_tokens_visible,
            if state.slm_off { "off" } else { "assisted" },
            state.status
        );
    }
    let codebook = LanguageCodebookV1::canonical();
    for (mut text, mut visibility) in &mut panels.p2() {
        let receipt = state
            .last_creature_receipt
            .as_ref()
            .or(state.last_player_receipt.as_ref());
        text.0 = receipt.map_or_else(
            || "Speech Inspector [DEV ONLY]\nNo utterance receipt".to_string(),
            |receipt| {
                format!(
                    "Speech Inspector [DEV ONLY]\nutterance {} | addressee {}\nraw tokens: {}\nliteral: {}\nrendered: {}\nconfidence {:.2} | SLM-assisted {} | uncertain {}",
                    receipt.utterance_id.raw(),
                    receipt
                        .addressee
                        .map(|id| id.raw().to_string())
                        .unwrap_or_else(|| "broadcast".to_string()),
                    receipt
                        .literal_tokens
                        .iter()
                        .map(|token| token.raw().to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    receipt.literal_text,
                    receipt.rendered_text,
                    receipt.confidence.raw(),
                    receipt.assisted,
                    receipt.uncertain,
                )
            },
        );
        *visibility = if state.developer_overlay && receipt.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, mut transform, mut visibility) in &mut panels.p3() {
        let receipt = state.last_creature_receipt.as_ref();
        let stable_id = state.last_creature_speaker.and_then(|speaker| {
            creatures
                .expression_buffer
                .iter()
                .find(|sample| sample.organism_id == speaker)
                .map(|sample| sample.stable_id)
        });
        let show_for_frequency =
            state
                .last_creature_utterance_id
                .is_some_and(|id| match state.narration {
                    NarrationDisplayFrequency::Off => false,
                    NarrationDisplayFrequency::Sparse => id.raw() % 4 == 0,
                    NarrationDisplayFrequency::Normal => id.raw() % 2 == 0,
                    NarrationDisplayFrequency::Frequent => true,
                });
        if !state.creature_utterance_active
            || !show_for_frequency
            || receipt.is_none()
            || stable_id.is_none()
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        let receipt = receipt.expect("checked above");
        text.0 = if state.translation_enabled {
            format!("{}\n{}", receipt.literal_text, receipt.rendered_text)
        } else {
            receipt.literal_text.clone()
        };
        if state.developer_overlay && state.raw_tokens_visible {
            text.0.push_str(&format!(
                "\n[{}]",
                receipt
                    .literal_tokens
                    .iter()
                    .map(|token| codebook.pronounceable_symbol(*token))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if let Some(position) = scene.selection_position(stable_id.expect("checked above")) {
            transform.translation = position + bevy::prelude::Vec3::Y * 1.55;
        }
        *visibility = Visibility::Visible;
    }
    for (mut text, mut visibility) in &mut panels.p4() {
        text.0 = lineage_panel_text(&state);
        *visibility = if state.lineage_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn lineage_panel_text(state: &ProductionConversationLineageUiState) -> String {
    let filtered = state.filtered_indices();
    let cursor = state.lineage_cursor.min(filtered.len().saturating_sub(1));
    let mut lines = vec![
        "Lineage Library".to_string(),
        format!(
            "Filter: {}  |  Tab filter  ↑/↓ select  F founder mode  A add  X remove  R refresh  Enter Create New World  Y/Esc close",
            state.lineage_filter.label()
        ),
        "Creature             Run              State       Survival    Problem     Language unaided/SLM  Checkpoint".to_string(),
    ];
    for (visible_index, row_index) in filtered.iter().take(14).enumerate() {
        let row = &state.lineage_rows[*row_index];
        lines.push(format!(
            "{} Creature {:<8} {:<16} {:<10} {:<11} {:<11} {:>7}/{:<7} {}",
            if visible_index == cursor { ">" } else { " " },
            row.organism_id.raw(),
            row.source_run_id,
            if row.deceased { "Deceased" } else { "Archived" },
            row.survival,
            row.problem_solving,
            row.language_unaided,
            row.language_assisted,
            row.checkpoint
                .map_or("genetic", |(_, retention)| match retention {
                    ArchiveCheckpointRetention::Pinned => "pinned checkpoint",
                    ArchiveCheckpointRetention::AutomaticPermanent => "learned checkpoint",
                    ArchiveCheckpointRetention::TemporaryPeak => "temporary checkpoint",
                }),
        ));
    }
    let selected = state.current_row();
    lines.push(String::new());
    lines.push(format!(
        "Selected: {} | Provenance: {} | Overall: {} | Founder mode: {}",
        selected
            .map(|row| format!("Creature {}", row.organism_id.raw()))
            .unwrap_or_else(|| "none".to_string()),
        selected
            .map(|row| short_digest(row.digest))
            .unwrap_or_else(|| "none".to_string()),
        selected
            .and_then(|row| row.overall_q16)
            .map(q16_text)
            .unwrap_or_else(|| "Unknown".to_string()),
        founder_mode_label(state.pending_founder_mode),
    ));
    lines.push(format!(
        "Founder Cohort {}/{} (genome distance and ancestry remain visible in provenance; no default kinship penalty)",
        state.cohort.len(),
        MAX_COHORT_SIZE
    ));
    lines.push(
        state
            .cohort
            .iter()
            .enumerate()
            .map(|(index, selection)| {
                format!(
                    "{}. {} [{}]",
                    index + 1,
                    short_digest(selection.source_manifest_digest),
                    founder_mode_label(selection.mode)
                )
            })
            .collect::<Vec<_>>()
            .join("   "),
    );
    lines.push(format!("Status: {}", state.status));
    lines.join("\n")
}

fn load_lineage_rows(root: &Path) -> Result<Vec<LineageUiRow>, alife_archive::ArchiveError> {
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(root))?;
    let mut rows = Vec::new();
    for digest in library.latest_manifest_digests()? {
        let manifest = library.load_manifest(digest)?;
        let statistics = manifest
            .life
            .as_ref()
            .map(|_| library.load_life_statistics(&manifest))
            .transpose()?;
        let checkpoint = manifest
            .life
            .as_ref()
            .and_then(|life| match &life.checkpoint {
                ArchiveCheckpointDisposition::Stored(reference) => {
                    Some((reference.digest, reference.retention))
                }
                _ => None,
            });
        rows.push(LineageUiRow {
            digest,
            source_run_id: manifest.genetic.source_run_id,
            organism_id: manifest.genetic.organism_id,
            deceased: manifest.life.is_some(),
            checkpoint,
            survival: statistics
                .as_ref()
                .map(|stats| stats.survival_ticks().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            problem_solving: combined_metric(
                statistics.as_ref(),
                &[
                    PassiveMetricKind::LearningSlope,
                    PassiveMetricKind::ReversalRecovery,
                ],
            ),
            language_unaided: metric_text(
                statistics.as_ref(),
                PassiveMetricKind::UnaidedComprehension,
            ),
            language_assisted: metric_text(
                statistics.as_ref(),
                PassiveMetricKind::SlmAssistedComprehension,
            ),
            overall_q16: overall_score(statistics.as_ref()),
        });
    }
    rows.sort_by(|left, right| {
        right
            .overall_q16
            .cmp(&left.overall_q16)
            .then_with(|| left.source_run_id.cmp(&right.source_run_id))
            .then_with(|| left.organism_id.raw().cmp(&right.organism_id.raw()))
    });
    Ok(rows)
}

fn overall_score(statistics: Option<&PassiveLifeStatistics>) -> Option<u32> {
    let statistics = statistics?;
    let values = PassiveMetricKind::ALL
        .iter()
        .filter_map(|kind| statistics.metric(*kind).value_q16())
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| {
        let sum = values.iter().map(|value| u64::from(*value)).sum::<u64>();
        (sum / values.len() as u64) as u32
    })
}

fn combined_metric(
    statistics: Option<&PassiveLifeStatistics>,
    kinds: &[PassiveMetricKind],
) -> String {
    let values = statistics
        .into_iter()
        .flat_map(|statistics| kinds.iter().map(|kind| statistics.metric(*kind)))
        .filter_map(MetricReading::value_q16)
        .collect::<Vec<_>>();
    if values.is_empty() {
        "Unknown".to_string()
    } else {
        q16_text(values.iter().sum::<u32>() / values.len() as u32)
    }
}

fn metric_text(statistics: Option<&PassiveLifeStatistics>, kind: PassiveMetricKind) -> String {
    statistics
        .and_then(|statistics| statistics.metric(kind).value_q16())
        .map(q16_text)
        .unwrap_or_else(|| "Unknown".to_string())
}

fn q16_text(value: u32) -> String {
    format!("{:.2}", value as f32 / 65_535.0)
}

fn founder_mode_label(mode: FounderMode) -> &'static str {
    match mode {
        FounderMode::GeneticFounder => "Genetic Founder",
        FounderMode::MindStateClone { .. } => "Mind Clone",
        FounderMode::GeneticOffspring { .. } => "Mutation Seed",
    }
}

fn mutation_seed(digest: Blake3Digest) -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest.bytes()[..8]);
    u64::from_le_bytes(bytes).max(1)
}

fn short_digest(digest: Blake3Digest) -> String {
    digest.bytes()[..5]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn default_lineage_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("A-Life")
        .join("lineage-library")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_metrics_remain_unknown() {
        assert_eq!(
            metric_text(None, PassiveMetricKind::UnaidedComprehension),
            "Unknown"
        );
        assert_eq!(
            combined_metric(None, &[PassiveMetricKind::LearningSlope]),
            "Unknown"
        );
    }

    #[test]
    fn founder_mode_defaults_to_genetic() {
        assert_eq!(
            founder_mode_label(FounderMode::default()),
            "Genetic Founder"
        );
    }

    #[test]
    fn digest_derived_mutation_seed_is_stable_and_nonzero() {
        let digest = Blake3Digest::from_bytes([7; 32]);
        assert_eq!(mutation_seed(digest), mutation_seed(digest));
        assert_ne!(mutation_seed(digest), 0);
    }
}
