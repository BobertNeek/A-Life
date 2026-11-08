//! Protected, opt-in CPU need-association port. This does not replace any builtin
//! reader, admit a GPU route, select an action, or change acquired neural state.
//!
//! Both inputs are separately learned **raw** own-history transitions. The
//! reference must be this organism's Rest/NOOP history, never an external control
//! drift vector. The source adapter owns provenance, context and learning checks;
//! these arithmetic input types do not manufacture sealed evidence. Unknown or
//! unsupported inputs execute the same circuit and are suppressed by native gates.
//!
//! Paired native successor, pressure and dendritic stages calculate
//! B(clamp(current + reference)) - B(clamp(current + action)). Shared components
//! cancel only after each complete raw transition has traversed those operators.
//! Confidence is the product of the two source confidences, not their minimum.

use crate::{
    apply_dendritic_conjunctions, cpu_spmv_projection, finalize_cpu_activations,
    ActivationFunction, BrainClassSpec, BrainScaleTier, BrainWorkCounters, CanonicalDigestBuilder,
    CooEntry, CooTile, CpuNeuralState, DendriticBranch, DendriticBranchSet, DendriticInputRef,
    DendriticWorkReceipt, NeuralActivationConfig, NeuralDiagnostics, NeuralProjectionSchema,
    ProjectionTile, ScaffoldContractError, SparseTileCoord, SynapseWeightSplit,
};
use serde::Serialize;
use std::collections::BTreeMap;

pub const NATIVE_NEED_AXIS_COUNT_V1: usize = 11;
pub const NATIVE_NEED_ASSOCIATION_PORT_V1: &str =
    "alife.cpu.own-history-comparative-need-association.v1";
const N: usize = NATIVE_NEED_AXIS_COUNT_V1;
const K: usize = 4;
const CONSTANT: usize = 3 * N;
const CHEMISTRY: &str = "reference-species22-drive-roles-atp-availability.v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NativeNeedAssociationErrorV1 {
    #[error("invalid bounded native need-association recipe")]
    InvalidRecipe,
    #[error("invalid native need-association input")]
    InvalidInput,
    #[error("native need-association projection reported rejected work")]
    RejectedProjection,
    #[error(transparent)]
    Native(#[from] ScaffoldContractError),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NativeNeedConfidenceV1(f32);
impl NativeNeedConfidenceV1 {
    pub fn new(value: f32) -> Result<Self, NativeNeedAssociationErrorV1> {
        if !unit(value) {
            return Err(NativeNeedAssociationErrorV1::InvalidInput);
        }
        Ok(Self(value))
    }
    pub const fn raw(self) -> f32 {
        self.0
    }
}

/// Arithmetic support only. The source adapter must validate own-history origin
/// and contextual support before claiming `OwnHistoryAssociation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeNeedSupportV1 {
    Unsupported,
    OwnHistoryAssociation,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum NativeNeedTransitionV1 {
    Unknown,
    Learned {
        raw_delta: [f32; N],
        confidence: NativeNeedConfidenceV1,
        support: NativeNeedSupportV1,
    },
}
impl NativeNeedTransitionV1 {
    pub fn learned(
        raw_delta: [f32; N],
        confidence: NativeNeedConfidenceV1,
        support: NativeNeedSupportV1,
    ) -> Result<Self, NativeNeedAssociationErrorV1> {
        let transition = Self::Learned {
            raw_delta,
            confidence,
            support,
        };
        transition.validate()?;
        Ok(transition)
    }
    pub const fn raw_delta(self) -> Option<[f32; N]> {
        match self {
            Self::Unknown => None,
            Self::Learned { raw_delta, .. } => Some(raw_delta),
        }
    }
    pub const fn confidence(self) -> Option<NativeNeedConfidenceV1> {
        match self {
            Self::Unknown => None,
            Self::Learned { confidence, .. } => Some(confidence),
        }
    }
    pub const fn support(self) -> Option<NativeNeedSupportV1> {
        match self {
            Self::Unknown => None,
            Self::Learned { support, .. } => Some(support),
        }
    }
    fn validate(self) -> Result<(), NativeNeedAssociationErrorV1> {
        if let Self::Learned {
            raw_delta,
            confidence,
            ..
        } = self
        {
            if raw_delta
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
                || !unit(confidence.raw())
            {
                return Err(NativeNeedAssociationErrorV1::InvalidInput);
            }
        }
        Ok(())
    }
    fn support_coefficient(self) -> f32 {
        match self.support() {
            Some(NativeNeedSupportV1::OwnHistoryAssociation) => 1.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeNeedReferenceKindV1 {
    Rest,
    Noop,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NativeNeedReferenceV1 {
    pub kind: NativeNeedReferenceKindV1,
    pub transition: NativeNeedTransitionV1,
}

/// A separate, prospectively engineered circuit gene, not a change to gain.
/// `PressureQuantumTanh` computes tanh(weighted pressure difference / quantum).
/// Quantum is in weighted burden units and bounded to [0.001, 1], so the
/// normalization projection coefficient never exceeds 1000. It is not a fitted
/// consequence label, a confidence multiplier, or a claim about a measured slope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum NativeNeedNormalizationV1 {
    Identity,
    PressureQuantumTanh { quantum: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NativeNeedAssociationRecipeV1 {
    pub knots: [f32; K],
    pub basis: [f32; K],
    pub magnitudes: [f32; N],
    pub atp_availability_target: f32,
    pub gain: f32,
    pub ceiling: f32,
    pub normalization: NativeNeedNormalizationV1,
}
impl NativeNeedAssociationRecipeV1 {
    /// Reference roles and bounded coefficients from the original pressure
    /// recipe. Only this new comparative CPU port consumes this configuration.
    pub const fn reference_species22() -> Self {
        Self {
            knots: [0.0, 0.25, 0.5, 0.75],
            basis: [0.125, 0.25, 0.25, 0.25],
            magnitudes: [
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                0.0,
                1.0 / 7.0,
                1.0 / 7.0,
                0.0,
                0.0,
                0.0,
            ],
            atp_availability_target: 0.75,
            gain: 0.35,
            ceiling: 0.35,
            normalization: NativeNeedNormalizationV1::Identity,
        }
    }
    pub fn validate(self) -> Result<(), NativeNeedAssociationErrorV1> {
        if !unit(self.gain)
            || self.gain > 0.35
            || !unit(self.ceiling)
            || self.ceiling > 0.35
            || !unit(self.atp_availability_target)
            || self.atp_availability_target == 0.0
            || self.knots.iter().any(|&v| !unit(v))
            || self.knots.windows(2).any(|p| p[0] >= p[1])
            || self.basis.iter().any(|&v| !unit(v))
            || self.basis.iter().sum::<f32>() > 1.0
            || self.magnitudes.iter().any(|&v| !unit(v))
            || self.magnitudes.iter().sum::<f32>() > 2.0
            || [5, 8, 9, 10].iter().any(|&i| self.magnitudes[i] != 0.0)
            || matches!(self.normalization,
                NativeNeedNormalizationV1::PressureQuantumTanh { quantum }
                    if !quantum.is_finite() || !(0.001..=1.0).contains(&quantum))
        {
            return Err(NativeNeedAssociationErrorV1::InvalidRecipe);
        }
        Ok(())
    }
}

/// Execution strategy is separate from the recipe's biological coefficients.
/// Both strategies allocate a validated Nano512 scratch state. Active execution
/// eliminates padded finalizer arithmetic, not allocation or sparse projection
/// validation. No shared-reference caching is implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeNeedExecutionV1 {
    FullScratchReference,
    ActiveCoordinates,
}

/// Measured operator work and explicit scratch-memory touches. Allocation,
/// validation and copies are not silently reported as sparse neural arithmetic.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeNeedAssociationWorkV1 {
    pub projections: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub finalizations: u32,
    pub activation_cells: u32,
    pub previous_activation_copies: u32,
    pub cleared_accumulator_cells: u32,
    pub stale_activation_clears: u32,
    pub stale_previous_activation_clears: u32,
    pub finalizer_coordinate_checks: u32,
    pub input_activation_writes: u32,
    pub initialized_coordinate_cells: u32,
    pub coordinate_tracking_copies: u32,
    pub coordinate_membership_probes: u32,
    pub initialized_scratch_float_cells: u32,
    pub transient_gate_projections: u32,
    pub dendritic: DendriticWorkReceipt,
}
impl NativeNeedAssociationWorkV1 {
    /// Existing ATP policy has no dendrite field. Charge each branch as a
    /// neuron operation (threshold/tanh/output) and each input as a synapse
    /// operation, in addition to the complete scratch finalizations and SpMVs.
    /// `lookup_memory_ops` is the caller's actually executed retrieval work,
    /// including unsuccessful lookups. It is not inferred from evidence status.
    /// Copies, zero initialization, validation, and metadata allocation are
    /// reported separately, not mislabelled as neural updates or elapsed time.
    pub fn brain_work_counters(&self, lookup_memory_ops: u64) -> BrainWorkCounters {
        BrainWorkCounters {
            // These are feed-forward scratch stages, not additional recurrent
            // or biological microsteps. Stage count remains in `finalizations`.
            microsteps: 0,
            neuron_updates: u64::from(self.activation_cells)
                + u64::from(self.dendritic.branches_evaluated),
            tile_visits: u64::from(self.active_tiles),
            synapse_ops: u64::from(self.active_synapses)
                + u64::from(self.dendritic.inputs_evaluated),
            decoder_candidate_ops: 0,
            memory_context_ops: lookup_memory_ops,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeNeedAssociationOutputV1 {
    pub circuit_identity: [u64; 4],
    pub recipe_identity: [u64; 4],
    pub execution: NativeNeedExecutionV1,
    pub current: [f32; N],
    pub action: NativeNeedTransitionV1,
    pub reference: NativeNeedReferenceV1,
    pub action_successor: [f32; N],
    pub reference_successor: [f32; N],
    pub action_successor_clipped: [bool; N],
    pub reference_successor_clipped: [bool; N],
    pub action_pressure: [f32; N],
    pub reference_pressure: [f32; N],
    pub action_burden: [f32; N],
    pub reference_burden: [f32; N],
    pub signed_association: [f32; N],
    pub weighted_difference: f32,
    pub normalized_difference: f32,
    pub readout_before_ceiling: f32,
    pub readout_before_gates: f32,
    pub confidence_product_gated_readout: f32,
    pub readout_clipped: bool,
    pub work: NativeNeedAssociationWorkV1,
}

#[derive(Clone, Debug)]
pub struct NativeNeedAssociationCircuitV1 {
    recipe: NativeNeedAssociationRecipeV1,
    identity: [u64; 4],
    recipe_identity: [u64; 4],
    execution: NativeNeedExecutionV1,
    branches: DendriticBranchSet,
    projections: Vec<NeuralProjectionSchema>,
}
impl NativeNeedAssociationCircuitV1 {
    pub fn compile(
        recipe: NativeNeedAssociationRecipeV1,
    ) -> Result<Self, NativeNeedAssociationErrorV1> {
        Self::compile_with_execution(recipe, NativeNeedExecutionV1::FullScratchReference)
    }
    pub fn compile_with_execution(
        recipe: NativeNeedAssociationRecipeV1,
        execution: NativeNeedExecutionV1,
    ) -> Result<Self, NativeNeedAssociationErrorV1> {
        recipe.validate()?;
        let mut digest = CanonicalDigestBuilder::new(NATIVE_NEED_ASSOCIATION_PORT_V1.as_bytes());
        digest.write_u16(1);
        digest.write_utf8(CHEMISTRY);
        // Fixed semantics are part of the identity, not mutable role labels.
        digest.write_utf8(
            "reference-minus-action;own-history-rest-or-noop;product-confidence;cpu512",
        );
        for value in recipe
            .knots
            .into_iter()
            .chain(recipe.basis)
            .chain(recipe.magnitudes)
            .chain([recipe.atp_availability_target, recipe.gain, recipe.ceiling])
        {
            digest.write_f32(value)?;
        }
        let (normalization_coefficient, normalization_tag) = match recipe.normalization {
            NativeNeedNormalizationV1::Identity => (1.0, 0),
            NativeNeedNormalizationV1::PressureQuantumTanh { quantum } => {
                digest.write_f32(quantum)?;
                (1.0 / quantum, 1)
            }
        };
        digest.write_u8(normalization_tag);
        let mut affine = Vec::new();
        let mut pressure = Vec::new();
        let mut difference = Vec::new();
        let mut weighted = Vec::new();
        let mut branches = Vec::new();
        for lane in 0..N {
            // Input: current, raw reference, raw action. Output: reference, action.
            affine.extend([
                (lane, lane, 1.0),
                (lane, N + lane, 1.0),
                (N + lane, lane, 1.0),
                (N + lane, 2 * N + lane, 1.0),
            ]);
            difference.extend([(lane, lane, 1.0), (lane, N + lane, -1.0)]);
            weighted.push((0, lane, recipe.magnitudes[lane]));
            for offset in [0, N] {
                let index = offset + lane;
                let (weight, intercept) = match lane {
                    0..=4 | 7 => (1.0, 0.0),
                    6 => (-1.0, recipe.atp_availability_target),
                    _ => (0.0, 0.0),
                };
                // Retain zero edges so status/gain/informational controls do the same work.
                pressure.extend([(index, index, weight), (index, CONSTANT, intercept)]);
                for k in 0..K {
                    branches.push(DendriticBranch::new(
                        index as u32,
                        recipe.knots[k],
                        recipe.basis[k],
                        vec![DendriticInputRef::new(index as u32, 1.0)?],
                    )?);
                }
            }
        }
        affine.push((CONSTANT, CONSTANT, 1.0));
        let recipe_identity = digest.finish256();
        let identity = match execution {
            NativeNeedExecutionV1::FullScratchReference => recipe_identity,
            NativeNeedExecutionV1::ActiveCoordinates => {
                let mut execution_digest = CanonicalDigestBuilder::new(
                    b"alife.cpu.need-association.active-coordinate-execution.v1",
                );
                for word in recipe_identity {
                    execution_digest.write_u64(word);
                }
                execution_digest.finish256()
            }
        };
        Ok(Self {
            recipe,
            identity,
            recipe_identity,
            execution,
            branches: DendriticBranchSet::new(branches)?,
            projections: vec![
                projection(&affine)?,
                projection(&pressure)?,
                projection(&difference)?,
                projection(&weighted)?,
                projection(&[(0, 0, normalization_coefficient)])?,
                projection(&[(0, 0, recipe.gain)])?,
            ],
        })
    }
    pub const fn identity(&self) -> [u64; 4] {
        self.identity
    }
    pub const fn recipe_identity(&self) -> [u64; 4] {
        self.recipe_identity
    }
    pub const fn execution(&self) -> NativeNeedExecutionV1 {
        self.execution
    }
    pub const fn recipe(&self) -> NativeNeedAssociationRecipeV1 {
        self.recipe
    }
    pub const fn port_identity(&self) -> &'static str {
        NATIVE_NEED_ASSOCIATION_PORT_V1
    }

    pub fn run(
        &self,
        current: [f32; N],
        action: NativeNeedTransitionV1,
        reference: NativeNeedReferenceV1,
    ) -> Result<NativeNeedAssociationOutputV1, NativeNeedAssociationErrorV1> {
        if current.iter().any(|&v| !unit(v)) {
            return Err(NativeNeedAssociationErrorV1::InvalidInput);
        }
        action.validate()?;
        reference.transition.validate()?;
        let action_delta = action.raw_delta().unwrap_or([0.0; N]);
        let reference_delta = reference.transition.raw_delta().unwrap_or([0.0; N]);
        let mut state = CpuNeuralState::for_brain_class(&brain_spec())?;
        state.activations[..N].copy_from_slice(&current);
        state.activations[N..2 * N].copy_from_slice(&reference_delta);
        state.activations[2 * N..3 * N].copy_from_slice(&action_delta);
        state.activations[CONSTANT] = 1.0;
        let mut finalizer = NativeNeedFinalizerV1::new(self.execution);
        let mut work = NativeNeedAssociationWorkV1 {
            initialized_scratch_float_cells: 3 * state.neuron_count,
            input_activation_writes: (3 * N + 1) as u32,
            initialized_coordinate_cells: finalizer.live.len() as u32,
            ..Default::default()
        };
        project(&self.projections[0], &mut state, &mut work)?;
        finalizer.finalize(
            &AFFINE_TARGETS,
            &mut state,
            ActivationFunction::Identity,
            0.0,
            1.0,
            &mut work,
        )?;
        let reference_successor = lanes(&state, 0);
        let action_successor = lanes(&state, N);
        let reference_successor_clipped =
            std::array::from_fn(|i| current[i] + reference_delta[i] != reference_successor[i]);
        let action_successor_clipped =
            std::array::from_fn(|i| current[i] + action_delta[i] != action_successor[i]);
        project(&self.projections[1], &mut state, &mut work)?;
        finalizer.finalize(
            &PAIRED_TARGETS,
            &mut state,
            ActivationFunction::Relu,
            0.0,
            1.0,
            &mut work,
        )?;
        let reference_pressure = lanes(&state, 0);
        let action_pressure = lanes(&state, N);
        work.dendritic = apply_dendritic_conjunctions(
            &state.activations,
            &mut state.accumulators,
            &self.branches,
        )?;
        finalizer.finalize(
            &PAIRED_TARGETS,
            &mut state,
            ActivationFunction::Identity,
            0.0,
            1.0,
            &mut work,
        )?;
        let reference_burden = lanes(&state, 0);
        let action_burden = lanes(&state, N);
        project(&self.projections[2], &mut state, &mut work)?;
        finalizer.finalize(
            &DIFFERENCE_TARGETS,
            &mut state,
            ActivationFunction::Identity,
            -1.0,
            1.0,
            &mut work,
        )?;
        let signed_association = lanes(&state, 0);
        project(&self.projections[3], &mut state, &mut work)?;
        finalizer.finalize(
            &[0],
            &mut state,
            ActivationFunction::Identity,
            -2.0,
            2.0,
            &mut work,
        )?;
        let weighted_difference = state.activations[0];
        project(&self.projections[4], &mut state, &mut work)?;
        match self.recipe.normalization {
            NativeNeedNormalizationV1::Identity => finalizer.finalize(
                &[0],
                &mut state,
                ActivationFunction::Identity,
                -2.0,
                2.0,
                &mut work,
            )?,
            NativeNeedNormalizationV1::PressureQuantumTanh { .. } => finalizer.finalize(
                &[0],
                &mut state,
                ActivationFunction::Tanh,
                -1.0,
                1.0,
                &mut work,
            )?,
        }
        let normalized_difference = state.activations[0];
        project(&self.projections[5], &mut state, &mut work)?;
        let readout_before_ceiling = state.accumulators[0];
        finalizer.finalize(
            &[0],
            &mut state,
            ActivationFunction::Identity,
            -self.recipe.ceiling,
            self.recipe.ceiling,
            &mut work,
        )?;
        let readout_before_gates = state.activations[0];
        // Dynamic coefficients are transient input bindings, not acquired or
        // genetic mutations. Crucially, no host readout multiplication occurs.
        // Unknown inputs still execute all four one-edge SpMVs/finalizations.
        for coefficient in [
            action.confidence().map_or(0.0, NativeNeedConfidenceV1::raw),
            reference
                .transition
                .confidence()
                .map_or(0.0, NativeNeedConfidenceV1::raw),
            action.support_coefficient(),
            reference.transition.support_coefficient(),
        ] {
            let gate = projection(&[(0, 0, coefficient)])?;
            work.transient_gate_projections += 1;
            project(&gate, &mut state, &mut work)?;
            finalizer.finalize(
                &[0],
                &mut state,
                ActivationFunction::Identity,
                -self.recipe.ceiling,
                self.recipe.ceiling,
                &mut work,
            )?;
        }
        Ok(NativeNeedAssociationOutputV1 {
            circuit_identity: self.identity,
            recipe_identity: self.recipe_identity,
            execution: self.execution,
            current,
            action,
            reference,
            action_successor,
            reference_successor,
            action_successor_clipped,
            reference_successor_clipped,
            action_pressure,
            reference_pressure,
            action_burden,
            reference_burden,
            signed_association,
            weighted_difference,
            normalized_difference,
            readout_before_ceiling,
            readout_before_gates,
            confidence_product_gated_readout: state.activations[0],
            readout_clipped: readout_before_ceiling != readout_before_gates,
            work,
        })
    }
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
fn brain_spec() -> BrainClassSpec {
    BrainClassSpec::for_tier(BrainScaleTier::Nano512)
}
fn lanes(state: &CpuNeuralState, offset: usize) -> [f32; N] {
    std::array::from_fn(|i| state.activations[offset + i])
}
fn projection(
    edges: &[(usize, usize, f32)],
) -> Result<NeuralProjectionSchema, ScaffoldContractError> {
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&brain_spec())?;
    let mut tiles: BTreeMap<(u32, u32), Vec<CooEntry>> = BTreeMap::new();
    for &(target, source, weight) in edges {
        let coord = SparseTileCoord::from_neuron_indices(target as u32, source as u32)?;
        tiles
            .entry((coord.microtile_row, coord.microtile_col))
            .or_default()
            .push(CooEntry::new(
                (target % 16) as u8,
                (source % 16) as u8,
                SynapseWeightSplit::new(weight, 0.0, 0.0, 0.0, 0.0)?,
            )?);
    }
    for ((row, col), entries) in tiles {
        schema.projections[0].tiles.push(ProjectionTile::new_coo(
            0,
            SparseTileCoord::new(row, col)?,
            CooTile::new(entries)?,
        ));
    }
    schema.rebuild_supertile_masks();
    schema.validate()?;
    Ok(schema)
}
fn project(
    schema: &NeuralProjectionSchema,
    state: &mut CpuNeuralState,
    work: &mut NativeNeedAssociationWorkV1,
) -> Result<(), NativeNeedAssociationErrorV1> {
    let report = cpu_spmv_projection(schema, state, NeuralDiagnostics::reference())?;
    if report.overflow_warnings != 0
        || report.range_rejections != 0
        || report.nan_rejections != 0
        || report.mask_skipped_tiles != 0
        || report.unsupported_tiles != 0
    {
        return Err(NativeNeedAssociationErrorV1::RejectedProjection);
    }
    work.projections += 1;
    work.active_tiles += report.active_tiles;
    work.active_synapses += report.active_synapses;
    Ok(())
}

// The only nonzero input locations are loaded by run() before any projection.
// Every later accumulator write is from one of the declared native stages.
const AFFINE_TARGETS: [usize; 23] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, CONSTANT,
];
const PAIRED_TARGETS: [usize; 22] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
];
const DIFFERENCE_TARGETS: [usize; N] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Private scratch owner. Its declared coordinates include zero-valued lanes:
/// neither data-dependent zeros nor unknown/zero-gain inputs skip any work.
/// Unlisted coordinates start at zero and remain zero by construction. This
/// helper cannot be used with arbitrary existing neural state or hidden writes.
struct NativeNeedFinalizerV1 {
    execution: NativeNeedExecutionV1,
    live: Vec<usize>,
    previous_live: Vec<usize>,
}
impl NativeNeedFinalizerV1 {
    fn new(execution: NativeNeedExecutionV1) -> Self {
        Self {
            execution,
            live: match execution {
                NativeNeedExecutionV1::FullScratchReference => Vec::new(),
                NativeNeedExecutionV1::ActiveCoordinates => (0..=CONSTANT).collect(),
            },
            previous_live: Vec::new(),
        }
    }
    fn finalize(
        &mut self,
        targets: &[usize],
        state: &mut CpuNeuralState,
        function: ActivationFunction,
        min: f32,
        max: f32,
        work: &mut NativeNeedAssociationWorkV1,
    ) -> Result<(), ScaffoldContractError> {
        match self.execution {
            NativeNeedExecutionV1::FullScratchReference => {
                finalize_full_scratch(state, function, min, max, work)
            }
            NativeNeedExecutionV1::ActiveCoordinates => {
                self.finalize_active(targets, state, function, min, max, work)
            }
        }
    }
    fn finalize_active(
        &mut self,
        targets: &[usize],
        state: &mut CpuNeuralState,
        function: ActivationFunction,
        min: f32,
        max: f32,
        work: &mut NativeNeedAssociationWorkV1,
    ) -> Result<(), ScaffoldContractError> {
        let count = state.neuron_count as usize;
        // This private port preserves the full valid CPU shape. It does not
        // truncate CpuNeuralState or change the default native finalizer.
        if count != 512
            || state.activations.len() != count
            || state.previous_activations.len() != count
            || state.accumulators.len() != count
            || !state.dendritic_branches.is_empty()
            || targets.iter().any(|&i| i >= count)
            || targets.windows(2).any(|p| p[0] >= p[1])
            || self.live.iter().any(|&i| i >= count)
            || self.live.windows(2).any(|p| p[0] >= p[1])
            || self.previous_live.iter().any(|&i| i >= count)
            || self.previous_live.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        // Inactive zero cells must map to zero. Logistic or non-zero clamps
        // would violate that property and are intentionally not admitted here.
        if !min.is_finite()
            || !max.is_finite()
            || min > 0.0
            || max < 0.0
            || matches!(function, ActivationFunction::Logistic)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        // Validate before mutation, so duplicate/range/nonfinite failures do
        // not leave a partially finalized scratch state.
        for &i in targets {
            crate::validate_finite(state.accumulators[i])?;
        }
        for &i in &self.live {
            crate::validate_finite(state.activations[i])?;
        }
        work.finalizer_coordinate_checks +=
            (targets.len() + self.live.len() + self.previous_live.len()) as u32;
        for &i in &self.previous_live {
            work.coordinate_membership_probes += 1;
            if self.live.binary_search(&i).is_err() {
                state.previous_activations[i] = 0.0;
                work.stale_previous_activation_clears += 1;
            }
        }
        for &i in &self.live {
            state.previous_activations[i] = state.activations[i];
            work.previous_activation_copies += 1;
        }
        for &i in &self.live {
            work.coordinate_membership_probes += 1;
            if targets.binary_search(&i).is_err() {
                state.activations[i] = 0.0;
                work.stale_activation_clears += 1;
            }
        }
        let mut range_rejections = 0;
        for &i in targets {
            let raw = state.accumulators[i];
            let activated = match function {
                ActivationFunction::Identity => raw,
                ActivationFunction::Relu => raw.max(0.0),
                ActivationFunction::Tanh => raw.tanh(),
                ActivationFunction::Logistic => {
                    unreachable!("validated zero-preserving activation")
                }
            };
            let clamped = activated.clamp(min, max);
            if clamped != activated {
                range_rejections += 1;
            }
            state.activations[i] = clamped;
            state.accumulators[i] = 0.0;
        }
        state.update_metadata.range_rejections = state
            .update_metadata
            .range_rejections
            .saturating_add(range_rejections);
        work.finalizations += 1;
        work.activation_cells += targets.len() as u32;
        work.cleared_accumulator_cells += targets.len() as u32;
        work.coordinate_tracking_copies += (self.live.len() + targets.len()) as u32;
        self.previous_live.clone_from(&self.live);
        self.live.clear();
        self.live.extend_from_slice(targets);
        Ok(())
    }
}

fn finalize_full_scratch(
    state: &mut CpuNeuralState,
    function: ActivationFunction,
    min: f32,
    max: f32,
    work: &mut NativeNeedAssociationWorkV1,
) -> Result<(), ScaffoldContractError> {
    finalize_cpu_activations(
        state,
        NeuralActivationConfig {
            function,
            clamp_min: min,
            clamp_max: max,
            clear_accumulators: true,
        },
    )?;
    work.finalizations += 1;
    work.activation_cells += state.neuron_count;
    work.previous_activation_copies += state.neuron_count;
    work.cleared_accumulator_cells += state.neuron_count;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BrainAtpCostModel;

    fn learned(delta: [f32; N], confidence: f32) -> NativeNeedTransitionV1 {
        NativeNeedTransitionV1::learned(
            delta,
            NativeNeedConfidenceV1::new(confidence).unwrap(),
            NativeNeedSupportV1::OwnHistoryAssociation,
        )
        .unwrap()
    }
    fn reference(delta: [f32; N], confidence: f32) -> NativeNeedReferenceV1 {
        NativeNeedReferenceV1 {
            kind: NativeNeedReferenceKindV1::Rest,
            transition: learned(delta, confidence),
        }
    }
    fn current() -> [f32; N] {
        [0.4, 0.35, 0.1, 0.1, 0.15, 0.5, 0.6, 0.1, 0.3, 0.2, 0.4]
    }
    fn circuit() -> NativeNeedAssociationCircuitV1 {
        NativeNeedAssociationCircuitV1::compile(NativeNeedAssociationRecipeV1::reference_species22()).unwrap()
    }
    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 2.0e-7,
            "actual={actual:?} expected={expected:?}"
        );
    }
    // Independent scalar oracle exists only in tests, never in runtime execution.
    fn burden(recipe: NativeNeedAssociationRecipeV1, lane: usize, raw: f32) -> f32 {
        let pressure = match lane {
            0..=4 | 7 => raw,
            6 => (recipe.atp_availability_target - raw).max(0.0),
            _ => 0.0,
        };
        recipe
            .knots
            .into_iter()
            .zip(recipe.basis)
            .map(|(k, w)| w * (pressure - k).max(0.0).tanh())
            .sum()
    }
    fn verify(output: &NativeNeedAssociationOutputV1, recipe: NativeNeedAssociationRecipeV1) {
        let action = output.action.raw_delta().unwrap_or([0.0; N]);
        let reference = output.reference.transition.raw_delta().unwrap_or([0.0; N]);
        let mut weighted = 0.0;
        for i in 0..N {
            let a = (output.current[i] + action[i]).clamp(0.0, 1.0);
            let r = (output.current[i] + reference[i]).clamp(0.0, 1.0);
            close(output.action_successor[i], a);
            close(output.reference_successor[i], r);
            close(output.action_burden[i], burden(recipe, i, a));
            close(output.reference_burden[i], burden(recipe, i, r));
            let difference = burden(recipe, i, r) - burden(recipe, i, a);
            close(output.signed_association[i], difference);
            weighted += difference * recipe.magnitudes[i];
        }
        close(output.weighted_difference, weighted);
        let normalized = match recipe.normalization {
            NativeNeedNormalizationV1::Identity => weighted,
            NativeNeedNormalizationV1::PressureQuantumTanh { quantum } => {
                (weighted / quantum).tanh()
            }
        };
        // The normalization may magnify single-precision projection roundoff.
        assert!((output.normalized_difference - normalized).abs() < 2.0e-5);
        let raw = normalized * recipe.gain;
        assert!((output.readout_before_ceiling - raw).abs() < 7.0e-6);
        let gated = raw.clamp(-recipe.ceiling, recipe.ceiling)
            * output
                .action
                .confidence()
                .map_or(0.0, NativeNeedConfidenceV1::raw)
            * output
                .reference
                .transition
                .confidence()
                .map_or(0.0, NativeNeedConfidenceV1::raw)
            * output.action.support_coefficient()
            * output.reference.transition.support_coefficient();
        assert!((output.confidence_product_gated_readout - gated).abs() < 7.0e-6);
    }

    #[test]
    fn native_need_common_mode_and_raw_vectors() {
        let circuit = circuit();
        let delta = [
            0.009, 0.006, -0.003, 0.005, -0.0075, 0.001, -0.014, 0.003, 0.001, -0.002, 0.002,
        ];
        let out = circuit
            .run(current(), learned(delta, 1.0), reference(delta, 1.0))
            .unwrap();
        assert_eq!(out.action.raw_delta(), Some(delta));
        assert_eq!(out.reference.transition.raw_delta(), Some(delta));
        assert_eq!(out.signed_association, [0.0; N]);
        assert_eq!(out.confidence_product_gated_readout, 0.0);
        verify(&out, circuit.recipe());
    }

    #[test]
    fn native_need_relief_satiation_harm_and_full_vector() {
        let circuit = circuit();
        let drift = [
            0.009, 0.006, 0.0, 0.0, 0.0, 0.001, -0.014, 0.003, 0.001, 0.0, 0.002,
        ];
        for base in [[0.0; N], drift] {
            for (loneliness, social_delta, expected_sign) in
                [(0.15, -0.0075, 1), (0.0, -0.0075, 0), (0.0, 0.0075, -1)]
            {
                let mut state = current();
                state[4] = loneliness;
                let mut delta = base;
                delta[4] += social_delta;
                let out = circuit
                    .run(state, learned(delta, 1.0), reference(base, 1.0))
                    .unwrap();
                verify(&out, circuit.recipe());
                let readout = out.confidence_product_gated_readout;
                match expected_sign {
                    1 => assert!(readout > 0.0),
                    -1 => assert!(readout < 0.0),
                    _ => assert_eq!(readout, 0.0),
                }
                for i in 0..N {
                    if i != 4 {
                        assert_eq!(out.signed_association[i], 0.0);
                    }
                }
                assert_eq!(out.action.raw_delta(), Some(delta));
                assert_eq!(out.reference.transition.raw_delta(), Some(base));
            }
        }
        let mut action = drift;
        action[4] = -0.0075;
        let out = circuit
            .run(current(), learned(action, 1.0), reference(drift, 1.0))
            .unwrap();
        let old_absolute = circuit
            .run(current(), learned(action, 1.0), reference([0.0; N], 1.0))
            .unwrap();
        assert!(out.confidence_product_gated_readout > 0.0);
        assert!(old_absolute.confidence_product_gated_readout < 0.0);
        println!(
            "native_full_vector comparative={} versus_zero_reference={}",
            out.confidence_product_gated_readout, old_absolute.confidence_product_gated_readout
        );
    }

    #[test]
    fn native_need_unknown_unsupported_and_confidence_product() {
        let circuit = circuit();
        let mut delta = [0.0; N];
        delta[4] = -0.0075;
        let full = circuit
            .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
            .unwrap();
        let fractional = circuit
            .run(current(), learned(delta, 0.5), reference([0.0; N], 0.25))
            .unwrap();
        close(
            fractional.confidence_product_gated_readout,
            full.confidence_product_gated_readout * 0.125,
        );
        assert!(
            (fractional.confidence_product_gated_readout
                - full.confidence_product_gated_readout * 0.25)
                .abs()
                > 1.0e-6
        );
        let unsupported = NativeNeedTransitionV1::learned(
            delta,
            NativeNeedConfidenceV1::new(1.0).unwrap(),
            NativeNeedSupportV1::Unsupported,
        )
        .unwrap();
        for (action, reference_transition) in [
            (NativeNeedTransitionV1::Unknown, learned([0.0; N], 1.0)),
            (learned(delta, 1.0), NativeNeedTransitionV1::Unknown),
            (learned(delta, 0.0), learned([0.0; N], 1.0)),
            (learned(delta, 1.0), learned([0.0; N], 0.0)),
            (unsupported, learned([0.0; N], 1.0)),
            (learned(delta, 1.0), unsupported),
        ] {
            let out = circuit
                .run(
                    current(),
                    action,
                    NativeNeedReferenceV1 {
                        kind: NativeNeedReferenceKindV1::Noop,
                        transition: reference_transition,
                    },
                )
                .unwrap();
            assert_eq!(out.confidence_product_gated_readout, 0.0);
            assert_eq!(
                out.work.brain_work_counters(2),
                full.work.brain_work_counters(2)
            );
            verify(&out, circuit.recipe());
        }
    }

    #[test]
    fn native_need_all_axes_and_native_successor_clamps() {
        let circuit = circuit();
        for lane in 0..N {
            for (value, a, r) in [
                (0.0, -1.0, 0.25),
                (0.75, 1.0, -1.0),
                (1.0, 0.5, -0.01),
                (0.25, -0.01, 0.01),
            ] {
                let mut state = current();
                state[lane] = value;
                let mut action = [0.0; N];
                action[lane] = a;
                let mut reference_delta = [0.0; N];
                reference_delta[lane] = r;
                let out = circuit
                    .run(state, learned(action, 1.0), reference(reference_delta, 1.0))
                    .unwrap();
                verify(&out, circuit.recipe());
                assert_eq!(
                    out.action_successor_clipped[lane],
                    !(0.0..=1.0).contains(&(value + a))
                );
                assert_eq!(
                    out.reference_successor_clipped[lane],
                    !(0.0..=1.0).contains(&(value + r))
                );
                if [5, 8, 9, 10].contains(&lane) {
                    assert_eq!(out.confidence_product_gated_readout, 0.0);
                }
            }
        }
    }

    #[test]
    fn native_need_engineered_normalization_identity_and_clipping() {
        let identity = circuit();
        let mut recipe = identity.recipe();
        recipe.normalization = NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 };
        let normalized = NativeNeedAssociationCircuitV1::compile(recipe).unwrap();
        assert_ne!(normalized.identity(), identity.identity());
        assert_eq!(normalized.recipe().gain, 0.35);
        assert_eq!(normalized.recipe().ceiling, 0.35);
        let mut prior = 0.0;
        for relief in [0.001, 0.00375, 0.0075, 0.015, 0.15] {
            let mut delta = [0.0; N];
            delta[4] = -relief;
            let out = normalized
                .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
                .unwrap();
            verify(&out, recipe);
            assert!(out.confidence_product_gated_readout > prior);
            assert!(out.confidence_product_gated_readout <= 0.35);
            println!("native_normalization quantum=0.001 relief={relief} weighted={} normalized={} readout={} secant_per_relief={} ceiling_clipped={}", out.weighted_difference, out.normalized_difference, out.confidence_product_gated_readout, out.confidence_product_gated_readout / relief, out.readout_clipped);
            prior = out.confidence_product_gated_readout;
        }
        recipe.ceiling = 0.01;
        let capped = NativeNeedAssociationCircuitV1::compile(recipe).unwrap();
        assert_ne!(normalized.identity(), capped.identity());
        let mut delta = [0.0; N];
        delta[4] = -0.0075;
        let out = capped
            .run(current(), learned(delta, 0.5), reference([0.0; N], 0.25))
            .unwrap();
        assert!(out.readout_clipped);
        close(out.readout_before_gates, 0.01);
        close(out.confidence_product_gated_readout, 0.00125);
        verify(&out, recipe);
    }

    #[test]
    fn native_need_work_is_full_scratch_and_zero_gain_is_not_free() {
        let circuit = circuit();
        let mut delta = [0.0; N];
        delta[4] = -0.0075;
        let out = circuit
            .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
            .unwrap();
        let mut recipe = circuit.recipe();
        recipe.gain = 0.0;
        let off = NativeNeedAssociationCircuitV1::compile(recipe).unwrap();
        let zero = off
            .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
            .unwrap();
        assert_ne!(circuit.identity(), off.identity());
        assert_eq!(zero.confidence_product_gated_readout, 0.0);
        assert_eq!(out.work, zero.work);
        assert_eq!(out.work.projections, 10);
        assert_eq!(out.work.active_synapses, 128);
        assert_eq!(out.work.active_tiles, 19);
        assert_eq!(out.work.finalizations, 11);
        assert_eq!(out.work.activation_cells, 5632);
        assert_eq!(out.work.previous_activation_copies, 5632);
        assert_eq!(out.work.cleared_accumulator_cells, 5632);
        assert_eq!(out.work.initialized_scratch_float_cells, 1536);
        assert_eq!(out.work.transient_gate_projections, 4);
        assert_eq!(out.work.dendritic.branches_evaluated, 88);
        assert_eq!(out.work.dendritic.inputs_evaluated, 88);
        assert_eq!(out.work.dendritic.work_units, 176);
        let brain = out.work.brain_work_counters(2);
        assert_eq!(brain.microsteps, 0);
        assert_eq!(brain.neuron_updates, 5720);
        assert_eq!(brain.synapse_ops, 216);
        assert_eq!(brain.memory_context_ops, 2);
        let cost = BrainAtpCostModel::production_v1();
        let q24 = cost.neural_cost_q24(&brain).unwrap();
        let q16 = cost.q24_to_atp_q16_round_half_up(q24).unwrap();
        assert_eq!(q24, 188896);
        assert_eq!(q16, 738);
        assert_eq!(
            q24,
            cost.neural_cost_q24(&zero.work.brain_work_counters(2))
                .unwrap()
        );
        println!(
            "native_work={:?} brain={brain:?} atp_q24={q24} atp_q16={q16}",
            out.work
        );
    }

    #[test]
    fn native_need_active_coordinates_exact_full_scratch_equivalence() {
        let mut pairs = 0;
        for normalization in [
            NativeNeedNormalizationV1::Identity,
            NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 },
        ] {
            for gain in [0.0, 0.35] {
                for ceiling in [0.0, 0.01, 0.35] {
                    let mut recipe = NativeNeedAssociationRecipeV1::reference_species22();
                    recipe.normalization = normalization;
                    recipe.gain = gain;
                    recipe.ceiling = ceiling;
                    let full = NativeNeedAssociationCircuitV1::compile(recipe).unwrap();
                    let active = NativeNeedAssociationCircuitV1::compile_with_execution(
                        recipe,
                        NativeNeedExecutionV1::ActiveCoordinates,
                    )
                    .unwrap();
                    assert_ne!(full.identity(), active.identity());
                    assert_eq!(full.recipe_identity(), active.recipe_identity());
                    assert_eq!(full.recipe(), active.recipe());
                    for lane in 0..N {
                        for (value, action_delta, reference_delta) in [
                            (0.0, -1.0, 0.1),
                            (0.25, -0.0075, 0.0),
                            (0.75, 0.0075, -0.01),
                            (1.0, 1.0, -1.0),
                        ] {
                            let mut state = current();
                            state[lane] = value;
                            let mut a = [0.003; N];
                            a[lane] = action_delta;
                            let mut r = [0.003; N];
                            r[lane] = reference_delta;
                            let unsupported = NativeNeedTransitionV1::learned(
                                a,
                                NativeNeedConfidenceV1::new(0.8).unwrap(),
                                NativeNeedSupportV1::Unsupported,
                            )
                            .unwrap();
                            for (action, reference_transition) in [
                                (learned(a, 1.0), learned(r, 1.0)),
                                (learned(a, 0.5), learned(r, 0.25)),
                                (learned(a, 0.0), learned(r, 1.0)),
                                (learned(a, 1.0), learned(r, 0.0)),
                                (NativeNeedTransitionV1::Unknown, learned(r, 1.0)),
                                (learned(a, 1.0), NativeNeedTransitionV1::Unknown),
                                (unsupported, learned(r, 1.0)),
                                (learned(a, 1.0), unsupported),
                            ] {
                                let reference = NativeNeedReferenceV1 {
                                    kind: NativeNeedReferenceKindV1::Rest,
                                    transition: reference_transition,
                                };
                                let expected = full.run(state, action, reference).unwrap();
                                let mut actual = active.run(state, action, reference).unwrap();
                                // Execution identity and truthful cost are intentionally
                                // distinct; every biological intermediate/output is exact.
                                assert_eq!(
                                    actual.execution,
                                    NativeNeedExecutionV1::ActiveCoordinates
                                );
                                actual.circuit_identity = expected.circuit_identity;
                                actual.execution = expected.execution;
                                actual.work = expected.work.clone();
                                assert_eq!(actual, expected);
                                pairs += 1;
                            }
                        }
                    }
                }
            }
        }
        println!("native_active_coordinate_exact_equivalence_pairs={pairs}");
    }

    #[test]
    fn native_need_active_work_and_zero_gain_control() {
        let recipe = NativeNeedAssociationRecipeV1::reference_species22();
        let active = NativeNeedAssociationCircuitV1::compile_with_execution(
            recipe,
            NativeNeedExecutionV1::ActiveCoordinates,
        )
        .unwrap();
        let mut delta = [0.0; N];
        delta[4] = -0.0075;
        let out = active
            .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
            .unwrap();
        let mut zero_recipe = recipe;
        zero_recipe.gain = 0.0;
        let zero = NativeNeedAssociationCircuitV1::compile_with_execution(
            zero_recipe,
            NativeNeedExecutionV1::ActiveCoordinates,
        )
        .unwrap()
        .run(current(), learned(delta, 1.0), reference([0.0; N], 1.0))
        .unwrap();
        assert_eq!(out.work, zero.work);
        let unknown = active
            .run(
                current(),
                NativeNeedTransitionV1::Unknown,
                NativeNeedReferenceV1 {
                    kind: NativeNeedReferenceKindV1::Noop,
                    transition: NativeNeedTransitionV1::Unknown,
                },
            )
            .unwrap();
        assert_eq!(
            out.work.brain_work_counters(2),
            unknown.work.brain_work_counters(2)
        );
        assert_eq!(out.work.projections, 10);
        assert_eq!(out.work.active_synapses, 128);
        assert_eq!(out.work.active_tiles, 19);
        assert_eq!(out.work.finalizations, 11);
        assert_eq!(out.work.activation_cells, 85);
        assert_eq!(out.work.previous_activation_copies, 118);
        assert_eq!(out.work.cleared_accumulator_cells, 85);
        assert_eq!(out.work.stale_activation_clears, 33);
        assert_eq!(out.work.stale_previous_activation_clears, 33);
        assert_eq!(out.work.initialized_scratch_float_cells, 1536);
        assert_eq!(out.work.input_activation_writes, 34);
        assert_eq!(out.work.initialized_coordinate_cells, 34);
        assert_eq!(out.work.coordinate_tracking_copies, 203);
        assert_eq!(out.work.coordinate_membership_probes, 235);
        assert_eq!(out.work.finalizer_coordinate_checks, 320);
        let brain = out.work.brain_work_counters(2);
        assert_eq!(brain.microsteps, 0);
        assert_eq!(brain.neuron_updates, 173);
        assert_eq!(brain.synapse_ops, 216);
        let cost = BrainAtpCostModel::production_v1();
        let q24 = cost.neural_cost_q24(&brain).unwrap();
        let q16 = cost.q24_to_atp_q16_round_half_up(q24).unwrap();
        assert_eq!(q24, 11392);
        assert_eq!(q16, 45);
        println!(
            "native_active_work={:?} brain={brain:?} atp_q24={q24} atp_q16={q16}",
            out.work
        );
    }

    #[test]
    fn native_need_active_finalizer_validates_and_clears_stale_coordinates() {
        let mut full = CpuNeuralState::for_brain_class(&brain_spec()).unwrap();
        for i in 0..=CONSTANT {
            full.activations[i] = i as f32 / 40.0;
        }
        let mut active = full.clone();
        let mut finalizer = NativeNeedFinalizerV1::new(NativeNeedExecutionV1::ActiveCoordinates);
        let mut active_work = NativeNeedAssociationWorkV1::default();
        let mut full_work = NativeNeedAssociationWorkV1::default();
        for (targets, function, min, max) in [
            (vec![0, 3, 33], ActivationFunction::Identity, -1.0, 1.0),
            (vec![2, 8], ActivationFunction::Relu, 0.0, 1.0),
            (vec![8, 33, 511], ActivationFunction::Tanh, -1.0, 1.0),
            (vec![], ActivationFunction::Identity, -1.0, 1.0),
            (vec![], ActivationFunction::Relu, 0.0, 1.0),
            (vec![0, 511], ActivationFunction::Identity, 0.0, 0.0),
        ] {
            for (ordinal, &i) in targets.iter().enumerate() {
                let value = if ordinal % 2 == 0 { -1.5 } else { 1.5 };
                full.accumulators[i] = value;
                active.accumulators[i] = value;
            }
            finalize_full_scratch(&mut full, function, min, max, &mut full_work).unwrap();
            finalizer
                .finalize(&targets, &mut active, function, min, max, &mut active_work)
                .unwrap();
            assert_eq!(active, full);
            assert!(active.accumulators.iter().all(|&v| v == 0.0));
        }
        assert!(active_work.stale_activation_clears > 0);
        assert!(active_work.stale_previous_activation_clears > 0);
        for targets in [vec![0, 0], vec![3, 1], vec![512], vec![usize::MAX]] {
            let before = active.clone();
            let work_before = active_work.clone();
            assert!(finalizer
                .finalize(
                    &targets,
                    &mut active,
                    ActivationFunction::Identity,
                    -1.0,
                    1.0,
                    &mut active_work
                )
                .is_err());
            assert_eq!(active, before);
            assert_eq!(active_work, work_before);
        }
        for (function, min, max) in [
            (ActivationFunction::Logistic, 0.0, 1.0),
            (ActivationFunction::Identity, 0.1, 1.0),
            (ActivationFunction::Identity, -1.0, -0.1),
            (ActivationFunction::Identity, f32::NAN, 1.0),
        ] {
            assert!(finalizer
                .finalize(&[0], &mut active, function, min, max, &mut active_work)
                .is_err());
        }
        active.accumulators[0] = f32::INFINITY;
        assert!(finalizer
            .finalize(
                &[0],
                &mut active,
                ActivationFunction::Identity,
                -1.0,
                1.0,
                &mut active_work
            )
            .is_err());
        assert_eq!(active.accumulators[0], f32::INFINITY);
    }

    #[test]
    fn native_need_rejects_invalid_genes_and_inputs() {
        let base = NativeNeedAssociationRecipeV1::reference_species22();
        for bad in [0.350001, 2048.0, f32::NAN] {
            let mut r = base;
            r.gain = bad;
            assert!(NativeNeedAssociationCircuitV1::compile(r).is_err());
            let mut r = base;
            r.ceiling = bad;
            assert!(NativeNeedAssociationCircuitV1::compile(r).is_err());
        }
        for bad in [0.0, 0.0009, 1.1, f32::NAN] {
            let mut r = base;
            r.normalization = NativeNeedNormalizationV1::PressureQuantumTanh { quantum: bad };
            assert!(NativeNeedAssociationCircuitV1::compile(r).is_err());
        }
        let mut r = base;
        r.magnitudes[9] = 0.01;
        assert!(NativeNeedAssociationCircuitV1::compile(r).is_err());
        assert!(NativeNeedConfidenceV1::new(f32::NAN).is_err());
        let mut delta = [0.0; N];
        delta[2] = 1.1;
        assert!(NativeNeedTransitionV1::learned(
            delta,
            NativeNeedConfidenceV1::new(1.0).unwrap(),
            NativeNeedSupportV1::OwnHistoryAssociation
        )
        .is_err());
        let mut state = current();
        state[0] = -0.01;
        assert!(circuit()
            .run(
                state,
                NativeNeedTransitionV1::Unknown,
                reference([0.0; N], 1.0)
            )
            .is_err());
    }
}
