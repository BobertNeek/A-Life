//! Player speech and cross-save lineage selection for the production frontend.

use std::path::{Path, PathBuf};

use alife_archive::{LineageLibrary, LineageLibraryConfig};
use alife_core::{
    ArchiveCheckpointDisposition, ArchiveCheckpointRetention, Blake3Digest, BrainClassId,
    FounderMode, FounderSelection, GenomeId, LanguageCodebookV1, LineageId, MetricReading,
    OrganismId, PassiveLifeStatistics, PassiveMetricKind, SpeechTranslationInput,
    SpeechTranslationReceipt, SpeechTranslationRequest, SurfaceTokenBinding, Tick, UtteranceId,
    UtteranceSourceKind, Validate, Vec3f,
};
use alife_semantic::{
    BoundedSpeechTranslator, LlamaCppSpeechTranslationConfig, LlamaCppSpeechTranslator,
    TranslationAssistance,
};
use alife_world::{persistence::PortableSaveFile, StableVoxelRefKind, WorldObjectKind};
use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::{
        App, BackgroundColor, ButtonInput, ChildOf, Color, Component, FlexDirection, FlexWrap,
        GlobalZIndex, KeyCode, MessageReader, Name, Node, NonSend, NonSendMut, ParamSet,
        PositionType, Res, ResMut, Resource, Text, Text2d, TextColor, TextFont, Transform, UiRect,
        Update, Val, Visibility, With,
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
const MIN_COHORT_SIZE: usize = 4;
const MAX_LIST_ROW_NODES: usize = 12;

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
enum LineageDataFilter {
    All,
    GeneticArchives,
    LearnedCheckpoints,
}

impl LineageDataFilter {
    const fn next(self) -> Self {
        match self {
            Self::All => Self::GeneticArchives,
            Self::GeneticArchives => Self::LearnedCheckpoints,
            Self::LearnedCheckpoints => Self::All,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::All => "all evidence",
            Self::GeneticArchives => "genetic archives",
            Self::LearnedCheckpoints => "learned checkpoints",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LineageSourceFilter {
    All,
    Run(String),
}

impl LineageSourceFilter {
    fn label(&self) -> &str {
        match self {
            Self::All => "All runs",
            Self::Run(run) => run,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineageSort {
    Overall,
    Survival,
    ProblemSolving,
    Language,
    Creature,
}

impl LineageSort {
    const fn next(self) -> Self {
        match self {
            Self::Overall => Self::Survival,
            Self::Survival => Self::ProblemSolving,
            Self::ProblemSolving => Self::Language,
            Self::Language => Self::Creature,
            Self::Creature => Self::Overall,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Overall => "Overall evidence",
            Self::Survival => "Survival",
            Self::ProblemSolving => "Problem solving",
            Self::Language => "Language (unaided)",
            Self::Creature => "Creature ID",
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
    survival_ticks: Option<u64>,
    survival: String,
    problem_q16: Option<u32>,
    problem_solving: String,
    language_unaided_q16: Option<u32>,
    language_unaided: String,
    language_assisted: String,
    overall_q16: Option<u32>,
    genome_id: Option<GenomeId>,
    lineage_id: Option<LineageId>,
    brain_class_id: Option<BrainClassId>,
    birth_tick: Option<Tick>,
    death_tick: Option<Tick>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineageLabRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

impl LineageLabRect {
    const fn right(self) -> f32 {
        self.left + self.width
    }

    const fn bottom(self) -> f32 {
        self.top + self.height
    }

    fn overlaps(self, other: Self) -> bool {
        self.left < other.right()
            && self.right() > other.left
            && self.top < other.bottom()
            && self.bottom() > other.top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineageLabSectionKind {
    Filters,
    List,
    Details,
    Founder,
    Cohort,
}

#[derive(Debug, Clone, Copy, Resource)]
struct LineageLabLayout {
    critical_font_size: f32,
    visible_rows: usize,
    filters: LineageLabRect,
    list: LineageLabRect,
    details: LineageLabRect,
    founder: LineageLabRect,
    cohort: LineageLabRect,
}

impl LineageLabLayout {
    fn for_resolution(width: u32, height: u32) -> Self {
        let compact = width < 1_600 || height < 900;
        Self {
            critical_font_size: if compact { 12.0 } else { 14.0 },
            visible_rows: if compact { 8 } else { MAX_LIST_ROW_NODES },
            filters: LineageLabRect {
                left: 1.0,
                top: 9.0,
                width: 20.0,
                height: 63.0,
            },
            list: LineageLabRect {
                left: 22.0,
                top: 9.0,
                width: 48.0,
                height: 63.0,
            },
            details: LineageLabRect {
                left: 71.0,
                top: 9.0,
                width: 28.0,
                height: 41.0,
            },
            founder: LineageLabRect {
                left: 71.0,
                top: 51.0,
                width: 28.0,
                height: 21.0,
            },
            cohort: LineageLabRect {
                left: 1.0,
                top: 73.0,
                width: 98.0,
                height: 22.0,
            },
        }
    }

    const fn section(self, kind: LineageLabSectionKind) -> LineageLabRect {
        match kind {
            LineageLabSectionKind::Filters => self.filters,
            LineageLabSectionKind::List => self.list,
            LineageLabSectionKind::Details => self.details,
            LineageLabSectionKind::Founder => self.founder,
            LineageLabSectionKind::Cohort => self.cohort,
        }
    }

    const fn primary_sections(self) -> [LineageLabRect; 5] {
        [
            self.filters,
            self.list,
            self.details,
            self.founder,
            self.cohort,
        ]
    }

    fn primary_sections_overlap(self) -> bool {
        let sections = self.primary_sections();
        sections.iter().enumerate().any(|(index, section)| {
            sections[index + 1..]
                .iter()
                .any(|other| section.overlaps(*other))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CohortEditError {
    Duplicate,
    Full,
}

fn add_founder_selection(
    cohort: &mut Vec<FounderSelection>,
    selection: FounderSelection,
) -> Result<(), CohortEditError> {
    if cohort
        .iter()
        .any(|existing| existing.source_manifest_digest == selection.source_manifest_digest)
    {
        return Err(CohortEditError::Duplicate);
    }
    if cohort.len() >= MAX_COHORT_SIZE {
        return Err(CohortEditError::Full);
    }
    cohort.push(selection);
    Ok(())
}

fn founder_cohort_ready(cohort: &[FounderSelection]) -> bool {
    (MIN_COHORT_SIZE..=MAX_COHORT_SIZE).contains(&cohort.len())
}

fn lineage_panel_visibility(open: bool) -> Visibility {
    if open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
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
    lineage_source_filter: LineageSourceFilter,
    lineage_data_filter: LineageDataFilter,
    lineage_sort: LineageSort,
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
            lineage_source_filter: LineageSourceFilter::All,
            lineage_data_filter: LineageDataFilter::All,
            lineage_sort: LineageSort::Overall,
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
        filtered_lineage_indices(
            &self.lineage_rows,
            &self.lineage_source_filter,
            self.lineage_data_filter,
            self.lineage_sort,
        )
    }

    fn current_row(&self) -> Option<&LineageUiRow> {
        let filtered = self.filtered_indices();
        filtered
            .get(self.lineage_cursor.min(filtered.len().saturating_sub(1)))
            .and_then(|index| self.lineage_rows.get(*index))
    }

    fn cycle_source_filter(&mut self) {
        let mut runs = self
            .lineage_rows
            .iter()
            .map(|row| row.source_run_id.clone())
            .collect::<Vec<_>>();
        runs.sort();
        runs.dedup();
        self.lineage_source_filter = match &self.lineage_source_filter {
            LineageSourceFilter::All => runs
                .first()
                .cloned()
                .map(LineageSourceFilter::Run)
                .unwrap_or(LineageSourceFilter::All),
            LineageSourceFilter::Run(current) => runs
                .iter()
                .position(|run| run == current)
                .and_then(|index| runs.get(index + 1))
                .cloned()
                .map(LineageSourceFilter::Run)
                .unwrap_or(LineageSourceFilter::All),
        };
        self.lineage_cursor = 0;
    }
}

fn compare_optional_desc<T: Ord>(left: Option<T>, right: Option<T>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn filtered_lineage_indices(
    rows: &[LineageUiRow],
    source_filter: &LineageSourceFilter,
    data_filter: LineageDataFilter,
    sort: LineageSort,
) -> Vec<usize> {
    let mut indices = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let source_matches = match source_filter {
                LineageSourceFilter::All => true,
                LineageSourceFilter::Run(run) => row.source_run_id == *run,
            };
            let data_matches = match data_filter {
                LineageDataFilter::All | LineageDataFilter::GeneticArchives => true,
                LineageDataFilter::LearnedCheckpoints => row.checkpoint.is_some(),
            };
            (source_matches && data_matches).then_some(index)
        })
        .collect::<Vec<_>>();
    indices.sort_by(|left_index, right_index| {
        let left = &rows[*left_index];
        let right = &rows[*right_index];
        let ordering = match sort {
            LineageSort::Overall => compare_optional_desc(left.overall_q16, right.overall_q16),
            LineageSort::Survival => {
                compare_optional_desc(left.survival_ticks, right.survival_ticks)
            }
            LineageSort::ProblemSolving => {
                compare_optional_desc(left.problem_q16, right.problem_q16)
            }
            LineageSort::Language => {
                compare_optional_desc(left.language_unaided_q16, right.language_unaided_q16)
            }
            LineageSort::Creature => left.organism_id.raw().cmp(&right.organism_id.raw()),
        };
        ordering
            .then_with(|| left.source_run_id.cmp(&right.source_run_id))
            .then_with(|| left.organism_id.raw().cmp(&right.organism_id.raw()))
    });
    indices
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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct LineageLabSectionMarker(LineageLabSectionKind);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum LineageLabTextRole {
    Header,
    FilterTitle,
    FilterSource,
    FilterData,
    FilterSort,
    ListTitle,
    ListHeader,
    ListRow(usize),
    DetailTitle,
    DetailIdentity,
    DetailProvenance,
    DetailMetrics,
    FounderTitle,
    FounderGenetic,
    FounderMind,
    FounderMutation,
    CohortHeader,
    CohortSlot(usize),
    Footer,
}

pub fn install_production_conversation_lineage_ui(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
) {
    app.insert_resource(ProductionConversationLineageUiState::new(summary));
    let layout = LineageLabLayout::for_resolution(summary.resolution.0, summary.resolution.1);
    app.insert_resource(layout);
    spawn_ui(app, layout);
    app.add_systems(
        Update,
        (
            handle_production_conversation_lineage_input,
            refresh_creature_speech_receipt,
            sync_production_conversation_lineage_ui,
            sync_production_lineage_laboratory_ui,
        ),
    );
}

fn spawn_ui(app: &mut App, layout: LineageLabLayout) {
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
    let root = app
        .world_mut()
        .spawn((
            Name::new("A-Life Lineage Library"),
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
        ))
        .id();

    spawn_lab_text(
        app,
        root,
        LineageLabTextRole::Header,
        "LINEAGE LIBRARY  /  ERA 0 SELECTION LABORATORY",
        layout.critical_font_size + 8.0,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(1.0),
            top: Val::Percent(1.0),
            width: Val::Percent(98.0),
            height: Val::Percent(6.0),
            ..Default::default()
        },
    );

    let filters = spawn_lab_section(app, root, layout, LineageLabSectionKind::Filters);
    for (role, text) in [
        (LineageLabTextRole::FilterTitle, "SOURCE / DATA FILTERS"),
        (LineageLabTextRole::FilterSource, "Source: All runs"),
        (LineageLabTextRole::FilterData, "Data: all evidence"),
        (LineageLabTextRole::FilterSort, "Sort: Overall evidence"),
    ] {
        spawn_lab_text(
            app,
            filters,
            role,
            text,
            layout.critical_font_size,
            lab_flow_text_node(),
        );
    }

    let list = spawn_lab_section(app, root, layout, LineageLabSectionKind::List);
    spawn_lab_text(
        app,
        list,
        LineageLabTextRole::ListTitle,
        "CREATURE ARCHIVES",
        layout.critical_font_size + 2.0,
        lab_flow_text_node(),
    );
    spawn_lab_text(
        app,
        list,
        LineageLabTextRole::ListHeader,
        "CREATURE      RUN          SURVIVAL   PROBLEM   LANGUAGE   CHECKPOINT",
        layout.critical_font_size - 1.0,
        lab_flow_text_node(),
    );
    for index in 0..MAX_LIST_ROW_NODES {
        spawn_lab_text(
            app,
            list,
            LineageLabTextRole::ListRow(index),
            "",
            layout.critical_font_size,
            lab_flow_text_node(),
        );
    }

    let details = spawn_lab_section(app, root, layout, LineageLabSectionKind::Details);
    for (role, text) in [
        (LineageLabTextRole::DetailTitle, "SELECTED CREATURE"),
        (LineageLabTextRole::DetailIdentity, "No archive selected"),
        (LineageLabTextRole::DetailProvenance, "Provenance: Unknown"),
        (LineageLabTextRole::DetailMetrics, "Evidence: Unknown"),
    ] {
        spawn_lab_text(
            app,
            details,
            role,
            text,
            layout.critical_font_size,
            lab_flow_text_node(),
        );
    }

    let founder = spawn_lab_section(app, root, layout, LineageLabSectionKind::Founder);
    for (role, text) in [
        (LineageLabTextRole::FounderTitle, "FOUNDER MODE  [F cycle]"),
        (LineageLabTextRole::FounderGenetic, "Genetic Founder"),
        (LineageLabTextRole::FounderMind, "Mind Clone"),
        (LineageLabTextRole::FounderMutation, "Mutation Seed"),
    ] {
        spawn_lab_text(
            app,
            founder,
            role,
            text,
            layout.critical_font_size,
            lab_flow_text_node(),
        );
    }

    let cohort = spawn_lab_section(app, root, layout, LineageLabSectionKind::Cohort);
    spawn_lab_text(
        app,
        cohort,
        LineageLabTextRole::CohortHeader,
        "FOUNDER COHORT 0/16  /  4 required  [A add] [X remove] [Enter create]",
        layout.critical_font_size,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(20.0),
            ..Default::default()
        },
    );
    for index in 0..MAX_COHORT_SIZE {
        spawn_lab_text(
            app,
            cohort,
            LineageLabTextRole::CohortSlot(index),
            &format!("{}  Empty", index + 1),
            layout.critical_font_size - 1.0,
            Node {
                width: Val::Percent(12.5),
                height: Val::Percent(38.0),
                padding: UiRect::all(Val::Px(3.0)),
                ..Default::default()
            },
        );
    }

    spawn_lab_text(
        app,
        root,
        LineageLabTextRole::Footer,
        "S source  D/Tab data  O sort  Up/Down select  F founder mode  A add  X remove  R refresh  Y/Esc close",
        layout.critical_font_size,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(1.0),
            top: Val::Percent(95.0),
            width: Val::Percent(98.0),
            height: Val::Percent(4.0),
            ..Default::default()
        },
    );
}

fn spawn_lab_section(
    app: &mut App,
    root: bevy::prelude::Entity,
    layout: LineageLabLayout,
    kind: LineageLabSectionKind,
) -> bevy::prelude::Entity {
    let rect = layout.section(kind);
    let is_cohort = kind == LineageLabSectionKind::Cohort;
    app.world_mut()
        .spawn((
            Name::new(format!("Lineage laboratory {kind:?} section")),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(rect.left),
                top: Val::Percent(rect.top),
                width: Val::Percent(rect.width),
                height: Val::Percent(rect.height),
                flex_direction: if is_cohort {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                flex_wrap: if is_cohort {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.028, 0.050, 0.035, 0.92)),
            ChildOf(root),
            LineageLabSectionMarker(kind),
        ))
        .id()
}

fn spawn_lab_text(
    app: &mut App,
    parent: bevy::prelude::Entity,
    role: LineageLabTextRole,
    value: &str,
    font_size: f32,
    node: Node,
) {
    app.world_mut().spawn((
        Name::new(format!("Lineage laboratory {role:?}")),
        Text::new(value),
        TextFont {
            font_size,
            ..Default::default()
        },
        TextColor(Color::srgb(0.92, 0.90, 0.74)),
        node,
        Visibility::Inherited,
        ChildOf(parent),
        role,
    ));
}

fn lab_flow_text_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        min_height: Val::Px(18.0),
        padding: UiRect::axes(Val::Px(3.0), Val::Px(2.0)),
        ..Default::default()
    }
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
    let request = SpeechTranslationRequest::try_new(
        UtteranceId::new(1)?,
        addressee,
        SpeechTranslationInput::PlayerText { text: text.clone() },
        state.bindings.clone(),
    )?;
    let (mut receipt, translation_warning) = translate_speech(&request, state.slm_off)?;
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
    state.status = translation_warning.unwrap_or_else(|| match addressee {
        Some(organism) => format!("Spoke spatially to creature {}", organism.raw()),
        None => "Spoke spatially to every creature in hearing range".to_string(),
    });
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
    let receipt = SpeechTranslationRequest::try_new(
        utterance.utterance_id,
        utterance.addressee,
        SpeechTranslationInput::CreatureTokens {
            tokens: utterance.tokens.clone(),
        },
        state.bindings.clone(),
    )
    .and_then(|request| translate_speech(&request, state.slm_off).map(|value| value.0));
    match receipt {
        Ok(receipt) => {
            state.last_creature_utterance_id = Some(utterance.utterance_id);
            state.last_creature_speaker = utterance.speaker_id;
            state.last_creature_receipt = Some(receipt);
        }
        Err(error) => state.status = format!("Creature speech translation failed: {error}"),
    }
}

fn translate_speech(
    request: &SpeechTranslationRequest,
    slm_off: bool,
) -> Result<(SpeechTranslationReceipt, Option<String>), alife_core::ScaffoldContractError> {
    if !slm_off {
        let assisted = LlamaCppSpeechTranslator::new(LlamaCppSpeechTranslationConfig::default())
            .map_err(|error| error.to_string())
            .and_then(|translator| translator.translate(request));
        if let Ok(receipt) = assisted {
            return Ok((receipt, None));
        }
    }
    let translator =
        BoundedSpeechTranslator::new("alife-bounded-unaided-v1", TranslationAssistance::Disabled)?;
    let receipt = translator.translate(request)?;
    let warning = (!slm_off).then(|| {
        "Local SLM unavailable or rejected its bounded output; used literal translation".to_string()
    });
    Ok((receipt, warning))
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
    if keyboard.just_pressed(KeyCode::KeyS) {
        state.cycle_source_filter();
    }
    if keyboard.just_pressed(KeyCode::Tab) || keyboard.just_pressed(KeyCode::KeyD) {
        state.lineage_data_filter = state.lineage_data_filter.next();
        state.lineage_cursor = 0;
    }
    if keyboard.just_pressed(KeyCode::KeyO) {
        state.lineage_sort = state.lineage_sort.next();
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
            let mode = state.pending_founder_mode;
            match add_founder_selection(
                &mut state.cohort,
                FounderSelection {
                    source_manifest_digest: digest,
                    mode,
                },
            ) {
                Ok(()) => {
                    state.status = format!(
                        "Added creature to founder cohort ({}/{MAX_COHORT_SIZE})",
                        state.cohort.len()
                    )
                }
                Err(CohortEditError::Duplicate) => {
                    state.status = "Founder cohort rejected duplicate creature".to_string()
                }
                Err(CohortEditError::Full) => {
                    state.status = "Founder cohort is already at 16 creatures".to_string()
                }
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
    if keyboard.just_pressed(KeyCode::Enter) {
        if founder_cohort_ready(&state.cohort) {
            match create_founder_world(state, ux) {
                Ok(path) => state.status = format!("Created new founder world: {}", path.display()),
                Err(error) => state.status = format!("Founder world creation failed: {error}"),
            }
        } else {
            state.status = format!(
                "Founder cohort needs {MIN_COHORT_SIZE}-{MAX_COHORT_SIZE} distinct creatures"
            );
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
}

fn sync_production_lineage_laboratory_ui(
    state: Res<ProductionConversationLineageUiState>,
    layout: Res<LineageLabLayout>,
    mut roots: bevy::prelude::Query<
        &mut Visibility,
        (
            With<ProductionLineageLibraryPanel>,
            bevy::prelude::Without<LineageLabTextRole>,
        ),
    >,
    mut texts: bevy::prelude::Query<
        (&LineageLabTextRole, &mut Text, &mut Visibility),
        With<LineageLabTextRole>,
    >,
) {
    for mut visibility in &mut roots {
        *visibility = lineage_panel_visibility(state.lineage_open);
    }

    let filtered = state.filtered_indices();
    let cursor = state.lineage_cursor.min(filtered.len().saturating_sub(1));
    let visible_count = layout.visible_rows.min(MAX_LIST_ROW_NODES);
    let window_start = if cursor >= visible_count {
        cursor + 1 - visible_count
    } else {
        0
    };
    let selected = state.current_row();

    for (role, mut text, mut visibility) in &mut texts {
        *visibility = Visibility::Inherited;
        text.0 = match *role {
            LineageLabTextRole::Header => {
                "LINEAGE LIBRARY  /  ERA 0 SELECTION LABORATORY".to_string()
            }
            LineageLabTextRole::FilterTitle => "SOURCE / DATA FILTERS".to_string(),
            LineageLabTextRole::FilterSource => format!(
                "Source run [S]\n{}",
                state.lineage_source_filter.label()
            ),
            LineageLabTextRole::FilterData => {
                format!("Data type [D/Tab]\n{}", state.lineage_data_filter.label())
            }
            LineageLabTextRole::FilterSort => {
                format!("Sort by [O]\n{}", state.lineage_sort.label())
            }
            LineageLabTextRole::ListTitle => {
                format!("CREATURE ARCHIVES  /  {} shown", filtered.len())
            }
            LineageLabTextRole::ListHeader => {
                "CREATURE      RUN          SURVIVAL   PROBLEM   LANGUAGE   CHECKPOINT".to_string()
            }
            LineageLabTextRole::ListRow(slot) => {
                let visible_index = window_start + slot;
                let Some(row_index) = filtered.get(visible_index) else {
                    *visibility = Visibility::Hidden;
                    text.0.clear();
                    continue;
                };
                let row = &state.lineage_rows[*row_index];
                format_lineage_row(row, visible_index == cursor)
            }
            LineageLabTextRole::DetailTitle => "SELECTED CREATURE".to_string(),
            LineageLabTextRole::DetailIdentity => selected.map_or_else(
                || "No archive selected".to_string(),
                |row| {
                    format!(
                        "Creature {}  /  {}\nRun {}  /  {}",
                        row.organism_id.raw(),
                        if row.deceased { "Deceased" } else { "Archived" },
                        row.source_run_id,
                        checkpoint_label(row.checkpoint),
                    )
                },
            ),
            LineageLabTextRole::DetailProvenance => selected.map_or_else(
                || "Manifest: Unknown\nGenome: Unknown\nLineage: Unknown".to_string(),
                |row| {
                    format!(
                        "Manifest {}\nGenome {}  /  Lineage {}\nBrain {}  /  Born {}  /  Died {}",
                        short_digest(row.digest),
                        debug_option(row.genome_id),
                        debug_option(row.lineage_id),
                        debug_option(row.brain_class_id),
                        tick_option(row.birth_tick),
                        tick_option(row.death_tick),
                    )
                },
            ),
            LineageLabTextRole::DetailMetrics => selected.map_or_else(
                || "Overall Unknown\nSurvival Unknown\nProblem Unknown\nLanguage Unknown / Unknown".to_string(),
                |row| {
                    format!(
                        "Overall {}\nSurvival {}\nProblem {}\nLanguage unaided {} / SLM {}",
                        option_q16(row.overall_q16),
                        row.survival,
                        row.problem_solving,
                        row.language_unaided,
                        row.language_assisted,
                    )
                },
            ),
            LineageLabTextRole::FounderTitle => "FOUNDER MODE  [F cycle]".to_string(),
            LineageLabTextRole::FounderGenetic => founder_mode_card(
                "Genetic Founder",
                "Genome + foundation only",
                matches!(state.pending_founder_mode, FounderMode::GeneticFounder),
            ),
            LineageLabTextRole::FounderMind => founder_mode_card(
                "Mind Clone",
                "Requires stored checkpoint",
                matches!(state.pending_founder_mode, FounderMode::MindStateClone { .. }),
            ),
            LineageLabTextRole::FounderMutation => founder_mode_card(
                "Mutation Seed",
                "Deterministic genetic offspring",
                matches!(state.pending_founder_mode, FounderMode::GeneticOffspring { .. }),
            ),
            LineageLabTextRole::CohortHeader => format!(
                "FOUNDER COHORT {}/{}  /  {}  [A add] [X remove] [Enter create]",
                state.cohort.len(),
                MAX_COHORT_SIZE,
                if founder_cohort_ready(&state.cohort) {
                    "Ready"
                } else {
                    "4 required"
                }
            ),
            LineageLabTextRole::CohortSlot(slot) => state.cohort.get(slot).map_or_else(
                || format!("{}  Empty", slot + 1),
                |selection| {
                    let row = state
                        .lineage_rows
                        .iter()
                        .find(|row| row.digest == selection.source_manifest_digest);
                    format!(
                        "{}  {}\n{}",
                        slot + 1,
                        row.map(|row| format!("Creature {}", row.organism_id.raw()))
                            .unwrap_or_else(|| short_digest(selection.source_manifest_digest)),
                        founder_mode_label(selection.mode),
                    )
                },
            ),
            LineageLabTextRole::Footer => format!(
                "S source  D/Tab data  O sort  Up/Down select  F mode  A add  X remove  R refresh  Y/Esc close\n{}",
                state.status
            ),
        };
    }
}

fn format_lineage_row(row: &LineageUiRow, selected: bool) -> String {
    format!(
        "{} Creature {:<5} {:<12} {:>8} {:>9} {:>10}   {}",
        if selected { ">" } else { " " },
        row.organism_id.raw(),
        bounded_text(&row.source_run_id, 12),
        row.survival,
        row.problem_solving,
        row.language_unaided,
        checkpoint_label(row.checkpoint),
    )
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn checkpoint_label(
    checkpoint: Option<(Blake3Digest, ArchiveCheckpointRetention)>,
) -> &'static str {
    checkpoint.map_or("Genetic", |(_, retention)| match retention {
        ArchiveCheckpointRetention::Pinned => "Pinned",
        ArchiveCheckpointRetention::AutomaticPermanent => "Learned",
        ArchiveCheckpointRetention::TemporaryPeak => "Temporary",
    })
}

fn option_q16(value: Option<u32>) -> String {
    value.map(q16_text).unwrap_or_else(|| "Unknown".to_string())
}

fn debug_option<T: std::fmt::Debug>(value: Option<T>) -> String {
    value
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn tick_option(value: Option<Tick>) -> String {
    value
        .map(|tick| tick.raw().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn founder_mode_card(title: &str, description: &str, selected: bool) -> String {
    format!(
        "{} {title}\n{description}",
        if selected { ">" } else { " " }
    )
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
        let survival_ticks = statistics.as_ref().map(|stats| stats.survival_ticks());
        let problem_q16 = combined_metric_value(
            statistics.as_ref(),
            &[
                PassiveMetricKind::LearningSlope,
                PassiveMetricKind::ReversalRecovery,
            ],
        );
        let language_unaided_q16 =
            metric_value(statistics.as_ref(), PassiveMetricKind::UnaidedComprehension);
        let language_assisted_q16 = metric_value(
            statistics.as_ref(),
            PassiveMetricKind::SlmAssistedComprehension,
        );
        rows.push(LineageUiRow {
            digest,
            source_run_id: manifest.genetic.source_run_id.clone(),
            organism_id: manifest.genetic.organism_id,
            deceased: manifest.life.is_some(),
            checkpoint,
            survival_ticks,
            survival: survival_ticks
                .map(|ticks| ticks.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            problem_q16,
            problem_solving: option_q16(problem_q16),
            language_unaided_q16,
            language_unaided: option_q16(language_unaided_q16),
            language_assisted: option_q16(language_assisted_q16),
            overall_q16: overall_score(statistics.as_ref()),
            genome_id: Some(manifest.genetic.genome_id),
            lineage_id: manifest.genetic.lineage_id,
            brain_class_id: Some(manifest.genetic.brain_class_id),
            birth_tick: Some(manifest.genetic.birth_tick),
            death_tick: manifest.life.as_ref().map(|life| life.death_tick),
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
    option_q16(combined_metric_value(statistics, kinds))
}

fn combined_metric_value(
    statistics: Option<&PassiveLifeStatistics>,
    kinds: &[PassiveMetricKind],
) -> Option<u32> {
    let values = statistics
        .into_iter()
        .flat_map(|statistics| kinds.iter().map(|kind| statistics.metric(*kind)))
        .filter_map(MetricReading::value_q16)
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<u32>() / values.len() as u32)
    }
}

fn metric_text(statistics: Option<&PassiveLifeStatistics>, kind: PassiveMetricKind) -> String {
    option_q16(metric_value(statistics, kind))
}

fn metric_value(
    statistics: Option<&PassiveLifeStatistics>,
    kind: PassiveMetricKind,
) -> Option<u32> {
    statistics.and_then(|statistics| statistics.metric(kind).value_q16())
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
    use std::{fs, time::SystemTime};

    use alife_archive::GeneticArchiveInput;
    use alife_core::{
        BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
        SensorProfile,
    };
    use bevy::prelude::{Children, Entity};

    fn temp_lineage_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alife-game-app-lineage-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn row(
        run: &str,
        organism: u64,
        overall_q16: Option<u32>,
        survival_ticks: Option<u64>,
        problem_q16: Option<u32>,
        checkpoint: bool,
    ) -> LineageUiRow {
        LineageUiRow {
            digest: Blake3Digest::from_bytes([organism as u8; 32]),
            source_run_id: run.to_string(),
            organism_id: OrganismId::new(organism).unwrap(),
            deceased: false,
            checkpoint: checkpoint.then(|| {
                (
                    Blake3Digest::from_bytes([organism as u8 + 1; 32]),
                    ArchiveCheckpointRetention::Pinned,
                )
            }),
            survival_ticks,
            survival: survival_ticks
                .map(|ticks| ticks.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            problem_q16,
            problem_solving: problem_q16
                .map(q16_text)
                .unwrap_or_else(|| "Unknown".to_string()),
            language_unaided_q16: None,
            language_unaided: "Unknown".to_string(),
            language_assisted: "Unknown".to_string(),
            overall_q16,
            genome_id: None,
            lineage_id: None,
            brain_class_id: None,
            birth_tick: None,
            death_tick: None,
        }
    }

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
    fn real_lineage_library_manifest_maps_provenance_and_preserves_unknown_metrics() {
        let root = temp_lineage_root("ui-mapping");
        let mut library =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
        let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
        let genome = BrainGenome::scaffold(812_001, capacity.id());
        let development = DevelopmentState::new(
            genome.id,
            Tick::new(4),
            NormalizedScalar::new(0.25).unwrap(),
        );
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let digest = library
            .archive_birth(GeneticArchiveInput {
                source_run_id: "selection-ui-real-run",
                organism_id: OrganismId::new(77).unwrap(),
                birth_tick: Tick::new(4),
                genome: &genome,
                phenotype: &phenotype,
                foundation_asset_bytes: None,
            })
            .unwrap();
        drop(library);

        let rows = load_lineage_rows(&root).unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.digest, digest);
        assert_eq!(row.source_run_id, "selection-ui-real-run");
        assert_eq!(row.organism_id, OrganismId::new(77).unwrap());
        assert_eq!(row.genome_id, Some(genome.id));
        assert_eq!(row.lineage_id, genome.lineage_id);
        assert_eq!(row.brain_class_id, Some(capacity.id()));
        assert_eq!(row.birth_tick, Some(Tick::new(4)));
        assert_eq!(row.death_tick, None);
        assert_eq!(row.checkpoint, None);
        assert_eq!(row.survival, "Unknown");
        assert_eq!(row.problem_solving, "Unknown");
        assert_eq!(row.language_unaided, "Unknown");
        assert_eq!(row.language_assisted, "Unknown");
        assert_eq!(row.overall_q16, None);

        fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn source_data_filters_and_sorting_keep_unknown_values_last() {
        let rows = vec![
            row("run-b", 3, None, None, None, true),
            row("run-a", 2, Some(20_000), Some(200), Some(40_000), false),
            row("run-a", 1, Some(50_000), Some(100), Some(10_000), true),
        ];

        let filtered = filtered_lineage_indices(
            &rows,
            &LineageSourceFilter::Run("run-a".to_string()),
            LineageDataFilter::LearnedCheckpoints,
            LineageSort::ProblemSolving,
        );
        assert_eq!(filtered, vec![2]);

        let sorted = filtered_lineage_indices(
            &rows,
            &LineageSourceFilter::All,
            LineageDataFilter::All,
            LineageSort::Overall,
        );
        assert_eq!(sorted, vec![2, 1, 0]);
    }

    #[test]
    fn founder_cohort_rejects_duplicates_and_overflow_and_requires_four_members() {
        let mut cohort = Vec::new();
        for id in 1..=4 {
            assert_eq!(
                add_founder_selection(
                    &mut cohort,
                    FounderSelection {
                        source_manifest_digest: Blake3Digest::from_bytes([id; 32]),
                        mode: FounderMode::GeneticFounder,
                    },
                ),
                Ok(())
            );
        }
        assert!(founder_cohort_ready(&cohort));
        let duplicate = cohort[0].clone();
        assert_eq!(
            add_founder_selection(&mut cohort, duplicate),
            Err(CohortEditError::Duplicate)
        );

        for id in 5..=16 {
            add_founder_selection(
                &mut cohort,
                FounderSelection {
                    source_manifest_digest: Blake3Digest::from_bytes([id; 32]),
                    mode: FounderMode::GeneticFounder,
                },
            )
            .unwrap();
        }
        assert_eq!(cohort.len(), 16);
        assert_eq!(
            add_founder_selection(
                &mut cohort,
                FounderSelection {
                    source_manifest_digest: Blake3Digest::from_bytes([17; 32]),
                    mode: FounderMode::GeneticFounder,
                },
            ),
            Err(CohortEditError::Full)
        );
    }

    #[test]
    fn closed_laboratory_hides_the_entire_structured_surface() {
        assert_eq!(lineage_panel_visibility(false), Visibility::Hidden);
        assert_eq!(lineage_panel_visibility(true), Visibility::Visible);
    }

    #[test]
    fn target_layouts_keep_primary_sections_inside_the_viewport() {
        for (width, height, minimum_rows) in [(1_920, 1_080, 12), (1_366, 768, 8)] {
            let layout = LineageLabLayout::for_resolution(width, height);
            assert!(layout.critical_font_size >= 12.0);
            assert!(layout.visible_rows >= minimum_rows);
            for section in layout.primary_sections() {
                assert!(section.left >= 0.0 && section.top >= 0.0);
                assert!(section.right() <= 100.0, "{section:?}");
                assert!(section.bottom() <= 100.0, "{section:?}");
                assert!(section.width > 0.0 && section.height > 0.0);
            }
            assert!(!layout.primary_sections_overlap());
        }
    }

    #[test]
    fn lineage_laboratory_is_a_real_section_and_content_node_hierarchy() {
        for (width, height) in [(1_920, 1_080), (1_366, 768)] {
            let layout = LineageLabLayout::for_resolution(width, height);
            let mut app = App::new();
            spawn_ui(&mut app, layout);
            let world = app.world_mut();
            let root = world
                .query_filtered::<Entity, With<ProductionLineageLibraryPanel>>()
                .single(world)
                .unwrap();
            assert_eq!(world.get::<Visibility>(root), Some(&Visibility::Hidden));
            assert!(world.get::<Text>(root).is_none());
            let root_children = world.get::<Children>(root).unwrap().to_vec();

            let sections = world
                .query::<(Entity, &LineageLabSectionMarker, &Node, Option<&Text>)>()
                .iter(world)
                .map(|(entity, marker, node, text)| {
                    (entity, marker.0, node.clone(), text.is_some())
                })
                .collect::<Vec<_>>();
            assert_eq!(sections.len(), 5);
            for (entity, kind, node, has_text) in sections {
                assert!(root_children.contains(&entity));
                assert!(!has_text, "{kind:?} container must not be a Text blob");
                assert_eq!(node.position_type, PositionType::Absolute);
                let expected = layout.section(kind);
                assert_eq!(node.left, Val::Percent(expected.left));
                assert_eq!(node.top, Val::Percent(expected.top));
                assert_eq!(node.width, Val::Percent(expected.width));
                assert_eq!(node.height, Val::Percent(expected.height));
                assert!(expected.right() <= 100.0);
                assert!(expected.bottom() <= 100.0);
                let children = world.get::<Children>(entity).unwrap();
                assert!(
                    children.len() >= 2,
                    "{kind:?} lacks structured content nodes"
                );
            }

            let list_rows = world
                .query::<&LineageLabTextRole>()
                .iter(world)
                .filter(|role| matches!(role, LineageLabTextRole::ListRow(_)))
                .count();
            let cohort_slots = world
                .query::<&LineageLabTextRole>()
                .iter(world)
                .filter(|role| matches!(role, LineageLabTextRole::CohortSlot(_)))
                .count();
            assert_eq!(list_rows, MAX_LIST_ROW_NODES);
            assert_eq!(cohort_slots, MAX_COHORT_SIZE);
        }
    }
}
