//! v0 scaffold: CPU sparse neural state and projection oracle, not GPU runtime code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    require_current_version, validate_finite, ActiveTilePolicy, BrainClassId, BrainClassSpec,
    LobeKind, NormalizedScalar, ProjectionType, ScaffoldContractError, SchemaKind, SchemaVersions,
    Tick, UpdateCadence, Validate, WEffective, WeightSplitContract,
};
use crate::{EffectiveWeightSample, LobeRegion};

pub const MICROTILE_EDGE: u32 = 16;
pub const MICROTILE_CELLS: usize = 256;
pub const SUPERTILE_MICROTILES: u32 = 8;
pub const SUPERTILE_EDGE: u32 = MICROTILE_EDGE * SUPERTILE_MICROTILES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseTileCoord {
    pub microtile_row: u32,
    pub microtile_col: u32,
}

impl SparseTileCoord {
    pub fn new(microtile_row: u32, microtile_col: u32) -> Result<Self, ScaffoldContractError> {
        let coord = Self {
            microtile_row,
            microtile_col,
        };
        microtile_row
            .checked_mul(MICROTILE_EDGE)
            .and_then(|start| start.checked_add(MICROTILE_EDGE))
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        microtile_col
            .checked_mul(MICROTILE_EDGE)
            .and_then(|start| start.checked_add(MICROTILE_EDGE))
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        Ok(coord)
    }

    pub fn from_neuron_indices(target: u32, source: u32) -> Result<Self, ScaffoldContractError> {
        Self::new(target / MICROTILE_EDGE, source / MICROTILE_EDGE)
    }

    pub fn target_start(self) -> u32 {
        self.microtile_row.saturating_mul(MICROTILE_EDGE)
    }

    pub fn source_start(self) -> u32 {
        self.microtile_col.saturating_mul(MICROTILE_EDGE)
    }

    pub fn supertile_row(self) -> u32 {
        self.microtile_row / SUPERTILE_MICROTILES
    }

    pub fn supertile_col(self) -> u32 {
        self.microtile_col / SUPERTILE_MICROTILES
    }

    pub fn supertile_local_bit(self) -> u8 {
        let row = self.microtile_row % SUPERTILE_MICROTILES;
        let col = self.microtile_col % SUPERTILE_MICROTILES;
        (row * SUPERTILE_MICROTILES + col) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Microtile;

impl Microtile {
    pub const EDGE: u32 = MICROTILE_EDGE;
    pub const CELLS: usize = MICROTILE_CELLS;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SynapseWeightSplit {
    pub genetic_fixed: f32,
    pub lifetime_consolidated: f32,
    pub alpha: f32,
    pub h_operational: f32,
    pub h_shadow: f32,
}

impl SynapseWeightSplit {
    pub fn new(
        genetic_fixed: f32,
        lifetime_consolidated: f32,
        alpha: f32,
        h_operational: f32,
        h_shadow: f32,
    ) -> Result<Self, ScaffoldContractError> {
        validate_finite(genetic_fixed)?;
        validate_finite(lifetime_consolidated)?;
        NormalizedScalar::new(alpha)?;
        validate_finite(h_operational)?;
        validate_finite(h_shadow)?;
        Ok(Self {
            genetic_fixed,
            lifetime_consolidated,
            alpha,
            h_operational,
            h_shadow,
        })
    }

    pub const fn zero() -> Self {
        Self {
            genetic_fixed: 0.0,
            lifetime_consolidated: 0.0,
            alpha: 0.0,
            h_operational: 0.0,
            h_shadow: 0.0,
        }
    }

    pub fn effective_weight(self) -> Result<f32, ScaffoldContractError> {
        Ok(WEffective::from_components(EffectiveWeightSample {
            genetic_fixed: self.genetic_fixed,
            lifetime_consolidated: self.lifetime_consolidated,
            alpha: NormalizedScalar::new(self.alpha)?,
            h_operational: self.h_operational,
        })?
        .value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparseTileType {
    Dense16x16,
    Coo,
    RowRun,
    ColumnRun,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DenseTile {
    pub weights: Vec<SynapseWeightSplit>,
}

impl DenseTile {
    pub fn new(weights: Vec<SynapseWeightSplit>) -> Result<Self, ScaffoldContractError> {
        if weights.len() != MICROTILE_CELLS {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Self { weights })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CooEntry {
    pub local_target: u8,
    pub local_source: u8,
    pub weights: SynapseWeightSplit,
}

impl CooEntry {
    pub fn new(
        local_target: u8,
        local_source: u8,
        weights: SynapseWeightSplit,
    ) -> Result<Self, ScaffoldContractError> {
        if u32::from(local_target) >= MICROTILE_EDGE || u32::from(local_source) >= MICROTILE_EDGE {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Self {
            local_target,
            local_source,
            weights,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CooTile {
    pub entries: Vec<CooEntry>,
}

impl CooTile {
    pub fn new(entries: Vec<CooEntry>) -> Result<Self, ScaffoldContractError> {
        Ok(Self { entries })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SparseTilePayload {
    Dense(DenseTile),
    Coo(CooTile),
    RowRunUnsupported,
    ColumnRunUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileMetadata {
    pub projection_index: u32,
    pub coord: SparseTileCoord,
    pub tile_type: SparseTileType,
    pub nonzero_count: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionTile {
    pub metadata: TileMetadata,
    pub payload: SparseTilePayload,
}

impl ProjectionTile {
    pub fn new_dense(projection_index: u32, coord: SparseTileCoord, tile: DenseTile) -> Self {
        Self {
            metadata: TileMetadata {
                projection_index,
                coord,
                tile_type: SparseTileType::Dense16x16,
                nonzero_count: MICROTILE_CELLS as u16,
            },
            payload: SparseTilePayload::Dense(tile),
        }
    }

    pub fn new_coo(projection_index: u32, coord: SparseTileCoord, tile: CooTile) -> Self {
        Self {
            metadata: TileMetadata {
                projection_index,
                coord,
                tile_type: SparseTileType::Coo,
                nonzero_count: tile.entries.len() as u16,
            },
            payload: SparseTilePayload::Coo(tile),
        }
    }

    pub fn new_unsupported(
        projection_index: u32,
        coord: SparseTileCoord,
        tile_type: SparseTileType,
    ) -> Self {
        let payload = match tile_type {
            SparseTileType::Dense16x16 => SparseTilePayload::Dense(DenseTile {
                weights: vec![SynapseWeightSplit::zero(); MICROTILE_CELLS],
            }),
            SparseTileType::Coo => SparseTilePayload::Coo(CooTile {
                entries: Vec::new(),
            }),
            SparseTileType::RowRun => SparseTilePayload::RowRunUnsupported,
            SparseTileType::ColumnRun => SparseTilePayload::ColumnRunUnsupported,
        };
        Self {
            metadata: TileMetadata {
                projection_index,
                coord,
                tile_type,
                nonzero_count: 0,
            },
            payload,
        }
    }

    pub fn decode_synapses(&self) -> Result<Vec<DecodedSynapse>, ScaffoldContractError> {
        decode_tile(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecodedSynapse {
    pub target: u32,
    pub source: u32,
    pub effective_weight: f32,
    pub weights: SynapseWeightSplit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupertileMask {
    pub supertile_row: u32,
    pub supertile_col: u32,
    pub active_microtile_mask: u64,
}

impl SupertileMask {
    fn contains(self, coord: SparseTileCoord) -> bool {
        self.supertile_row == coord.supertile_row()
            && self.supertile_col == coord.supertile_col()
            && (self.active_microtile_mask & (1_u64 << coord.supertile_local_bit())) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRoutingRef {
    pub source_lobe: LobeKind,
    pub target_lobe: LobeKind,
    pub projection_type: ProjectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuronRange {
    pub lobe: Option<LobeKind>,
    pub start: u32,
    pub len: u32,
}

impl NeuronRange {
    fn full(neuron_count: u32) -> Self {
        Self {
            lobe: None,
            start: 0,
            len: neuron_count,
        }
    }

    fn from_lobe(region: &LobeRegion) -> Self {
        Self {
            lobe: Some(region.kind),
            start: region.start,
            len: region.len,
        }
    }

    fn end(self) -> Result<u32, ScaffoldContractError> {
        self.start
            .checked_add(self.len)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)
    }

    fn validate(self, neuron_count: u32) -> Result<(), ScaffoldContractError> {
        if self.len == 0
            || !self.start.is_multiple_of(MICROTILE_EDGE)
            || !self.len.is_multiple_of(MICROTILE_EDGE)
            || self.end()? > neuron_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseProjection {
    pub schema_version: u16,
    pub projection_index: u32,
    pub routing_ref: ProjectionRoutingRef,
    pub source_range: NeuronRange,
    pub target_range: NeuronRange,
    pub active_tile_policy: ActiveTilePolicy,
    pub update_cadence: UpdateCadence,
    pub max_active_tiles: u32,
    pub tiles: Vec<ProjectionTile>,
    pub supertile_masks: Vec<SupertileMask>,
}

impl SparseProjection {
    fn full_brain_reference(spec: &BrainClassSpec) -> Self {
        Self {
            schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
            projection_index: 0,
            routing_ref: ProjectionRoutingRef {
                source_lobe: LobeKind::SensoryGrounding,
                target_lobe: LobeKind::SensoryGrounding,
                projection_type: ProjectionType::Recurrent,
            },
            source_range: NeuronRange::full(spec.neuron_count),
            target_range: NeuronRange::full(spec.neuron_count),
            active_tile_policy: ActiveTilePolicy::EssentialReservation,
            update_cadence: UpdateCadence::Hot60Hz,
            max_active_tiles: spec.max_active_microtiles,
            tiles: Vec::new(),
            supertile_masks: Vec::new(),
        }
    }

    fn validate(&self, neuron_count: u32) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::NeuralProjection, self.schema_version)?;
        self.source_range.validate(neuron_count)?;
        self.target_range.validate(neuron_count)?;
        if self.max_active_tiles == 0 || self.tiles.len() > self.max_active_tiles as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }

        for tile in &self.tiles {
            if tile.metadata.projection_index != self.projection_index {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
            validate_tile_payload(tile)?;
            let target_start = tile.metadata.coord.target_start();
            let source_start = tile.metadata.coord.source_start();
            let target_end = target_start
                .checked_add(MICROTILE_EDGE)
                .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
            let source_end = source_start
                .checked_add(MICROTILE_EDGE)
                .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
            if target_start < self.target_range.start
                || target_end > self.target_range.end()?
                || source_start < self.source_range.start
                || source_end > self.source_range.end()?
            {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
        }
        Ok(())
    }

    fn tile_is_active(&self, coord: SparseTileCoord) -> bool {
        self.supertile_masks.is_empty()
            || self.supertile_masks.iter().any(|mask| mask.contains(coord))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuralProjectionSchema {
    pub schema_version: u16,
    pub brain_class_id: BrainClassId,
    pub neuron_count: u32,
    pub microtile_edge: u32,
    pub supertile_edge: u32,
    pub projections: Vec<SparseProjection>,
}

impl NeuralProjectionSchema {
    pub fn empty_for_brain_class(spec: &BrainClassSpec) -> Result<Self, ScaffoldContractError> {
        spec.validate()?;
        let schema = Self {
            schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
            brain_class_id: spec.id,
            neuron_count: spec.neuron_count,
            microtile_edge: MICROTILE_EDGE,
            supertile_edge: SUPERTILE_EDGE,
            projections: vec![SparseProjection::full_brain_reference(spec)],
        };
        schema.validate()?;
        Ok(schema)
    }

    pub fn from_routing_for_fixture(
        spec: &BrainClassSpec,
        weight_split: &WeightSplitContract,
    ) -> Result<Self, ScaffoldContractError> {
        spec.validate()?;
        weight_split.validate_contract()?;
        if weight_split.genetic_fixed.descriptor.brain_class_id != spec.id
            || weight_split.max_active_tiles != spec.max_active_microtiles
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }

        let mut masks = spec.routing_matrix.masks().to_vec();
        masks.sort_by_key(|mask| {
            (
                mask.source_lobe.stable_id().raw(),
                mask.target_lobe.stable_id().raw(),
                projection_type_order(mask.projection_type),
            )
        });

        let mut projections = Vec::with_capacity(masks.len());
        for (index, mask) in masks.into_iter().enumerate() {
            let source = spec
                .lobe_layout
                .region(mask.source_lobe)
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
            let target = spec
                .lobe_layout
                .region(mask.target_lobe)
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
            projections.push(SparseProjection {
                schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
                projection_index: index as u32,
                routing_ref: ProjectionRoutingRef {
                    source_lobe: mask.source_lobe,
                    target_lobe: mask.target_lobe,
                    projection_type: mask.projection_type,
                },
                source_range: NeuronRange::from_lobe(source),
                target_range: NeuronRange::from_lobe(target),
                active_tile_policy: mask.active_tile_policy,
                update_cadence: mask.update_cadence,
                max_active_tiles: spec.max_active_microtiles,
                tiles: Vec::new(),
                supertile_masks: Vec::new(),
            });
        }

        let schema = Self {
            schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
            brain_class_id: spec.id,
            neuron_count: spec.neuron_count,
            microtile_edge: MICROTILE_EDGE,
            supertile_edge: SUPERTILE_EDGE,
            projections,
        };
        schema.validate()?;
        Ok(schema)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::NeuralProjection, self.schema_version)?;
        self.brain_class_id.validate()?;
        if self.neuron_count < 512
            || !self.neuron_count.is_multiple_of(SUPERTILE_EDGE)
            || self.microtile_edge != MICROTILE_EDGE
            || self.supertile_edge != SUPERTILE_EDGE
            || self.projections.is_empty()
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for (index, projection) in self.projections.iter().enumerate() {
            if projection.projection_index != index as u32 {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
            projection.validate(self.neuron_count)?;
        }
        Ok(())
    }

    pub fn rebuild_supertile_masks(&mut self) {
        for projection in &mut self.projections {
            let mut masks: BTreeMap<(u32, u32), u64> = BTreeMap::new();
            for tile in &projection.tiles {
                let coord = tile.metadata.coord;
                let entry = masks
                    .entry((coord.supertile_row(), coord.supertile_col()))
                    .or_insert(0);
                *entry |= 1_u64 << coord.supertile_local_bit();
            }
            projection.supertile_masks = masks
                .into_iter()
                .map(
                    |((supertile_row, supertile_col), active_microtile_mask)| SupertileMask {
                        supertile_row,
                        supertile_col,
                        active_microtile_mask,
                    },
                )
                .collect();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeActivationView {
    pub lobe: LobeKind,
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralUpdateMetadata {
    pub tick: Tick,
    pub max_active_synapses: u32,
    pub max_active_tiles: u32,
    pub overflow_events: u64,
    pub range_rejections: u64,
    pub nan_rejections: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlasticityTraceBuffers {
    pub h_shadow_decay_reservoir: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuNeuralState {
    pub brain_class_id: BrainClassId,
    pub neuron_count: u32,
    pub activations: Vec<f32>,
    pub previous_activations: Vec<f32>,
    pub accumulators: Vec<f32>,
    pub lobe_views: Vec<LobeActivationView>,
    pub projections: Vec<SparseProjection>,
    pub weight_split: WeightSplitContract,
    pub plasticity_traces: PlasticityTraceBuffers,
    pub update_metadata: NeuralUpdateMetadata,
}

impl CpuNeuralState {
    pub fn for_brain_class(spec: &BrainClassSpec) -> Result<Self, ScaffoldContractError> {
        spec.validate()?;
        let neuron_count = spec.neuron_count as usize;
        let weight_split = WeightSplitContract::for_brain_class(
            spec.id,
            spec.max_active_synapses,
            spec.max_active_microtiles,
            1,
        )?;
        let lobe_views = spec
            .lobe_layout
            .enabled_regions()
            .map(|region| LobeActivationView {
                lobe: region.kind,
                start: region.start,
                len: region.len,
            })
            .collect();

        Ok(Self {
            brain_class_id: spec.id,
            neuron_count: spec.neuron_count,
            activations: vec![0.0; neuron_count],
            previous_activations: vec![0.0; neuron_count],
            accumulators: vec![0.0; neuron_count],
            lobe_views,
            projections: Vec::new(),
            weight_split,
            plasticity_traces: PlasticityTraceBuffers {
                h_shadow_decay_reservoir: Vec::new(),
            },
            update_metadata: NeuralUpdateMetadata {
                tick: Tick::ZERO,
                max_active_synapses: spec.max_active_synapses,
                max_active_tiles: spec.max_active_microtiles,
                overflow_events: 0,
                range_rejections: 0,
                nan_rejections: 0,
            },
        })
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        let expected = self.neuron_count as usize;
        if self.activations.len() != expected
            || self.previous_activations.len() != expected
            || self.accumulators.len() != expected
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ActivationFunction {
    Identity,
    Relu,
    Tanh,
    Logistic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralActivationConfig {
    pub function: ActivationFunction,
    pub clamp_min: f32,
    pub clamp_max: f32,
    pub clear_accumulators: bool,
}

impl NeuralActivationConfig {
    pub const fn reference() -> Self {
        Self {
            function: ActivationFunction::Tanh,
            clamp_min: -1.0,
            clamp_max: 1.0,
            clear_accumulators: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OjaUpdateConfig {
    pub learning_rate: f32,
    pub learning_rate_scale: f32,
    pub decay: f32,
    pub shadow_min: f32,
    pub shadow_max: f32,
}

impl OjaUpdateConfig {
    pub const fn reference() -> Self {
        Self {
            learning_rate: 0.01,
            learning_rate_scale: 1.0,
            decay: 1.0,
            shadow_min: -4.0,
            shadow_max: 4.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralDiagnostics {
    pub accumulator_abs_limit: f32,
    pub effective_weight_abs_limit: f32,
}

impl NeuralDiagnostics {
    pub const fn reference() -> Self {
        Self {
            accumulator_abs_limit: 1.0e6,
            effective_weight_abs_limit: 1.0e4,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralUpdateReport {
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub mask_skipped_tiles: u32,
    pub overflow_warnings: u32,
    pub range_rejections: u32,
    pub nan_rejections: u32,
    pub unsupported_tiles: u32,
}

pub fn cpu_spmv_projection(
    schema: &NeuralProjectionSchema,
    state: &mut CpuNeuralState,
    diagnostics: NeuralDiagnostics,
) -> Result<NeuralUpdateReport, ScaffoldContractError> {
    schema.validate()?;
    state.validate_shape()?;
    validate_diagnostics(diagnostics)?;

    let mut report = NeuralUpdateReport::default();
    for projection in &schema.projections {
        for tile in &projection.tiles {
            if !projection.tile_is_active(tile.metadata.coord) {
                report.mask_skipped_tiles = report.mask_skipped_tiles.saturating_add(1);
                continue;
            }
            report.active_tiles = report.active_tiles.saturating_add(1);
            for synapse in tile.decode_synapses()? {
                report.active_synapses = report.active_synapses.saturating_add(1);
                let source = synapse.source as usize;
                let target = synapse.target as usize;
                if source >= state.activations.len() || target >= state.accumulators.len() {
                    return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
                }
                let activation = validate_finite(state.activations[source])?;
                let weight = validate_finite(synapse.effective_weight)?;
                if weight.abs() > diagnostics.effective_weight_abs_limit {
                    report.overflow_warnings = report.overflow_warnings.saturating_add(1);
                }
                let delta = validate_finite(activation * weight)?;
                let next = validate_finite(state.accumulators[target] + delta)?;
                if next.abs() > diagnostics.accumulator_abs_limit {
                    report.overflow_warnings = report.overflow_warnings.saturating_add(1);
                }
                state.accumulators[target] = next;
            }
        }
    }
    state.update_metadata.overflow_events = state
        .update_metadata
        .overflow_events
        .saturating_add(u64::from(report.overflow_warnings));
    Ok(report)
}

pub fn finalize_cpu_activations(
    state: &mut CpuNeuralState,
    config: NeuralActivationConfig,
) -> Result<NeuralUpdateReport, ScaffoldContractError> {
    state.validate_shape()?;
    validate_finite(config.clamp_min)?;
    validate_finite(config.clamp_max)?;
    if config.clamp_min > config.clamp_max {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }

    state.previous_activations.clone_from(&state.activations);
    let mut report = NeuralUpdateReport::default();
    for (activation, accumulator) in state.activations.iter_mut().zip(&mut state.accumulators) {
        let raw = validate_finite(*accumulator)?;
        let activated = match config.function {
            ActivationFunction::Identity => raw,
            ActivationFunction::Relu => raw.max(0.0),
            ActivationFunction::Tanh => raw.tanh(),
            ActivationFunction::Logistic => 1.0 / (1.0 + (-raw).exp()),
        };
        let clamped = validate_finite(activated)?.clamp(config.clamp_min, config.clamp_max);
        if clamped != activated {
            report.range_rejections = report.range_rejections.saturating_add(1);
        }
        *activation = clamped;
        if config.clear_accumulators {
            *accumulator = 0.0;
        }
    }
    state.update_metadata.range_rejections = state
        .update_metadata
        .range_rejections
        .saturating_add(u64::from(report.range_rejections));
    Ok(report)
}

pub fn update_oja_shadow_traces(
    schema: &mut NeuralProjectionSchema,
    state: &CpuNeuralState,
    config: OjaUpdateConfig,
) -> Result<NeuralUpdateReport, ScaffoldContractError> {
    schema.validate()?;
    state.validate_shape()?;
    validate_oja_config(config)?;

    let mut report = NeuralUpdateReport::default();
    for projection in &mut schema.projections {
        let active_masks = projection.supertile_masks.clone();
        for tile in &mut projection.tiles {
            if !tile_active_from_masks(&active_masks, tile.metadata.coord) {
                report.mask_skipped_tiles = report.mask_skipped_tiles.saturating_add(1);
                continue;
            }
            report.active_tiles = report.active_tiles.saturating_add(1);
            match &mut tile.payload {
                SparseTilePayload::Dense(dense) => {
                    for (index, weights) in dense.weights.iter_mut().enumerate() {
                        let local_target = (index / MICROTILE_EDGE as usize) as u32;
                        let local_source = (index % MICROTILE_EDGE as usize) as u32;
                        update_oja_weight(
                            weights,
                            tile.metadata.coord.target_start() + local_target,
                            tile.metadata.coord.source_start() + local_source,
                            state,
                            config,
                        )?;
                        report.active_synapses = report.active_synapses.saturating_add(1);
                    }
                }
                SparseTilePayload::Coo(coo) => {
                    for entry in &mut coo.entries {
                        update_oja_weight(
                            &mut entry.weights,
                            tile.metadata.coord.target_start() + u32::from(entry.local_target),
                            tile.metadata.coord.source_start() + u32::from(entry.local_source),
                            state,
                            config,
                        )?;
                        report.active_synapses = report.active_synapses.saturating_add(1);
                    }
                }
                SparseTilePayload::RowRunUnsupported | SparseTilePayload::ColumnRunUnsupported => {
                    return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
                }
            }
        }
    }
    Ok(report)
}

fn decode_tile(tile: &ProjectionTile) -> Result<Vec<DecodedSynapse>, ScaffoldContractError> {
    match &tile.payload {
        SparseTilePayload::Dense(dense) => {
            if tile.metadata.tile_type != SparseTileType::Dense16x16
                || dense.weights.len() != MICROTILE_CELLS
            {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
            let mut decoded = Vec::with_capacity(MICROTILE_CELLS);
            for (index, weights) in dense.weights.iter().copied().enumerate() {
                let local_target = (index / MICROTILE_EDGE as usize) as u32;
                let local_source = (index % MICROTILE_EDGE as usize) as u32;
                decoded.push(DecodedSynapse {
                    target: checked_local_index(tile.metadata.coord.target_start(), local_target)?,
                    source: checked_local_index(tile.metadata.coord.source_start(), local_source)?,
                    effective_weight: weights.effective_weight()?,
                    weights,
                });
            }
            Ok(decoded)
        }
        SparseTilePayload::Coo(coo) => {
            if tile.metadata.tile_type != SparseTileType::Coo {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
            let mut decoded = Vec::with_capacity(coo.entries.len());
            for entry in &coo.entries {
                decoded.push(DecodedSynapse {
                    target: checked_local_index(
                        tile.metadata.coord.target_start(),
                        u32::from(entry.local_target),
                    )?,
                    source: checked_local_index(
                        tile.metadata.coord.source_start(),
                        u32::from(entry.local_source),
                    )?,
                    effective_weight: entry.weights.effective_weight()?,
                    weights: entry.weights,
                });
            }
            Ok(decoded)
        }
        SparseTilePayload::RowRunUnsupported | SparseTilePayload::ColumnRunUnsupported => {
            Err(ScaffoldContractError::UnsupportedSparseTileFormat)
        }
    }
}

fn validate_tile_payload(tile: &ProjectionTile) -> Result<(), ScaffoldContractError> {
    match (&tile.metadata.tile_type, &tile.payload) {
        (SparseTileType::Dense16x16, SparseTilePayload::Dense(dense)) => {
            if dense.weights.len() == MICROTILE_CELLS {
                Ok(())
            } else {
                Err(ScaffoldContractError::InvalidSparseProjectionSchema)
            }
        }
        (SparseTileType::Coo, SparseTilePayload::Coo(_)) => Ok(()),
        (SparseTileType::RowRun, SparseTilePayload::RowRunUnsupported)
        | (SparseTileType::ColumnRun, SparseTilePayload::ColumnRunUnsupported) => Ok(()),
        _ => Err(ScaffoldContractError::InvalidSparseProjectionSchema),
    }
}

fn checked_local_index(start: u32, local: u32) -> Result<u32, ScaffoldContractError> {
    start
        .checked_add(local)
        .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)
}

fn update_oja_weight(
    weights: &mut SynapseWeightSplit,
    target: u32,
    source: u32,
    state: &CpuNeuralState,
    config: OjaUpdateConfig,
) -> Result<(), ScaffoldContractError> {
    let source = source as usize;
    let target = target as usize;
    if source >= state.previous_activations.len() || target >= state.activations.len() {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    let pre = validate_finite(state.previous_activations[source])?;
    let post = validate_finite(state.activations[target])?;
    let current = validate_finite(weights.h_shadow)?;
    let delta = validate_finite(
        config.learning_rate
            * config.learning_rate_scale
            * (post * pre - config.decay * post * post * current),
    )?;
    weights.h_shadow =
        validate_finite(current + delta)?.clamp(config.shadow_min, config.shadow_max);
    Ok(())
}

fn validate_diagnostics(diagnostics: NeuralDiagnostics) -> Result<(), ScaffoldContractError> {
    validate_finite(diagnostics.accumulator_abs_limit)?;
    validate_finite(diagnostics.effective_weight_abs_limit)?;
    if diagnostics.accumulator_abs_limit <= 0.0 || diagnostics.effective_weight_abs_limit <= 0.0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(())
}

fn validate_oja_config(config: OjaUpdateConfig) -> Result<(), ScaffoldContractError> {
    validate_finite(config.learning_rate)?;
    validate_finite(config.learning_rate_scale)?;
    validate_finite(config.decay)?;
    validate_finite(config.shadow_min)?;
    validate_finite(config.shadow_max)?;
    if config.learning_rate < 0.0
        || config.learning_rate_scale < 0.0
        || config.decay < 0.0
        || config.shadow_min > config.shadow_max
    {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(())
}

fn tile_active_from_masks(masks: &[SupertileMask], coord: SparseTileCoord) -> bool {
    masks.is_empty() || masks.iter().any(|mask| mask.contains(coord))
}

fn projection_type_order(projection_type: ProjectionType) -> u8 {
    match projection_type {
        ProjectionType::FeedForward => 0,
        ProjectionType::Feedback => 1,
        ProjectionType::Recurrent => 2,
        ProjectionType::Modulatory => 3,
        ProjectionType::MotorProposal => 4,
        ProjectionType::Homeostatic => 5,
        ProjectionType::LateralInhibition => 6,
    }
}
