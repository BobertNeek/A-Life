//! Phenotype-bound, opt-in CPU comparative need reader V2.
//!
//! V1 remains untouched. V2 adds the phase3 inherited boredom pressure, verifies
//! every drive role against the actual expressed chemistry, and admits current
//! drives only through an opaque phenotype/state binding. This is not a general
//! biological-value function, a reward maximizer, a GPU route, or a checkpoint.
//!
//! Native operator implementation is deliberately isolated from frozen V1. Its
//! seven original pressure lanes and its declared scratch work are unchanged.

use crate::{
    apply_dendritic_conjunctions, cpu_spmv_projection, finalize_cpu_activations,
    ActivationFunction, BiochemistryState, BrainClassSpec, BrainScaleTier, CanonicalDigestBuilder,
    CooEntry, CooTile, CpuNeuralState, CreaturePhenotype, DendriticBranch, DendriticBranchSet,
    DendriticInputRef, NativeNeedAssociationErrorV1, NativeNeedAssociationWorkV1,
    NativeNeedConfidenceV1, NativeNeedExecutionV1, NativeNeedNormalizationV1,
    NativeNeedReferenceV1, NativeNeedSupportV1, NativeNeedTransitionV1, NeuralActivationConfig,
    NeuralDiagnostics, NeuralProjectionSchema, ProjectionTile, ScaffoldContractError,
    SparseTileCoord, SynapseWeightSplit,
};
use serde::Serialize;
use std::collections::BTreeMap;

mod binding;
use binding::NativeNeedPhenotypeBindingV2;
pub use binding::{NativeNeedBindingWorkV2, NativeNeedBoundCurrentV2, NativeNeedDriveRoleV2};

pub const NATIVE_NEED_ASSOCIATION_PORT_V2: &str =
    "alife.cpu.phenotype-bound-own-history-comparative-need-association.v2";
pub const NATIVE_NEED_AXIS_COUNT_V2: usize = 11;
const N: usize = NATIVE_NEED_AXIS_COUNT_V2;
const K: usize = 4;
const CONSTANT: usize = 3 * N;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NativeNeedAssociationErrorV2 {
    #[error("invalid bounded phenotype-bound need recipe")]
    InvalidRecipe,
    #[error("invalid phenotype-bound need input")]
    InvalidInput,
    #[error("unsupported phase3 biochemical binding: {0}")]
    UnsupportedPhenotype(&'static str),
    #[error("current state does not match the bound phenotype or graph-derived homeostasis")]
    BindingMismatch,
    #[error("native need-association projection reported rejected work")]
    RejectedProjection,
    #[error("canonical phenotype encoding failed")]
    Encoding,
    #[error(transparent)]
    Native(#[from] ScaffoldContractError),
    #[error(transparent)]
    Transition(#[from] NativeNeedAssociationErrorV1),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NativeNeedAssociationRecipeV2 {
    pub roles: [NativeNeedDriveRoleV2; N],
    pub knots: [f32; K],
    pub basis: [f32; K],
    pub magnitudes: [f32; N],
    /// Prospectively engineered availability target, not an inferred optimum.
    pub atp_availability_target: f32,
    pub gain: f32,
    pub ceiling: f32,
    pub normalization: NativeNeedNormalizationV1,
}
impl NativeNeedAssociationRecipeV2 {
    pub const fn phase3_species22() -> Self {
        Self {
            roles: binding::PHASE3_ROLES,
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
                0.08,
                0.0,
            ],
            atp_availability_target: 0.75,
            gain: 0.35,
            ceiling: 0.35,
            normalization: NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 },
        }
    }
    pub fn validate(self) -> Result<(), NativeNeedAssociationErrorV2> {
        let fixed = Self::phase3_species22();
        // Roles, shape, target and phenotype-derived magnitudes are not tuning
        // knobs. Bounded gain/ceiling and normalization retain explicit controls.
        if self.roles != fixed.roles
            || self.knots != fixed.knots
            || self.basis != fixed.basis
            || self.magnitudes != fixed.magnitudes
            || self.atp_availability_target != fixed.atp_availability_target
            || !unit(self.gain)
            || self.gain > 0.35
            || !unit(self.ceiling)
            || self.ceiling > 0.35
            || matches!(self.normalization,
                NativeNeedNormalizationV1::PressureQuantumTanh { quantum }
                    if !quantum.is_finite() || !(0.001..=1.0).contains(&quantum))
        {
            return Err(NativeNeedAssociationErrorV2::InvalidRecipe);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeNeedAssociationOutputV2 {
    pub binding_identity: [u64; 4],
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
pub struct NativeNeedAssociationCircuitV2 {
    recipe: NativeNeedAssociationRecipeV2,
    binding: NativeNeedPhenotypeBindingV2,
    identity: [u64; 4],
    recipe_identity: [u64; 4],
    execution: NativeNeedExecutionV1,
    branches: DendriticBranchSet,
    projections: Vec<NeuralProjectionSchema>,
}
impl NativeNeedAssociationCircuitV2 {
    pub fn compile_bound(
        phenotype: &CreaturePhenotype,
        recipe: NativeNeedAssociationRecipeV2,
        execution: NativeNeedExecutionV1,
    ) -> Result<Self, NativeNeedAssociationErrorV2> {
        recipe.validate()?;
        let binding = NativeNeedPhenotypeBindingV2::new(phenotype, &recipe.roles)?;
        let mut digest = CanonicalDigestBuilder::new(NATIVE_NEED_ASSOCIATION_PORT_V2.as_bytes());
        digest.write_u16(2);
        for word in binding.identity() {
            digest.write_u64(word);
        }
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
                let (weight, intercept) = match recipe.roles[lane] {
                    NativeNeedDriveRoleV2::Burden => (1.0, 0.0),
                    NativeNeedDriveRoleV2::Availability => (-1.0, recipe.atp_availability_target),
                    NativeNeedDriveRoleV2::Regulatory
                    | NativeNeedDriveRoleV2::Opportunity
                    | NativeNeedDriveRoleV2::Unbound => (0.0, 0.0),
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
                    b"alife.cpu.phenotype-bound-need-association.active-coordinate-execution.v2",
                );
                for word in recipe_identity {
                    execution_digest.write_u64(word);
                }
                execution_digest.finish256()
            }
        };
        Ok(Self {
            recipe,
            binding,
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
    pub const fn recipe(&self) -> NativeNeedAssociationRecipeV2 {
        self.recipe
    }
    pub const fn port_identity(&self) -> &'static str {
        NATIVE_NEED_ASSOCIATION_PORT_V2
    }

    pub const fn binding_identity(&self) -> [u64; 4] {
        self.binding.identity()
    }
    pub fn compile_binding_work(&self) -> &NativeNeedBindingWorkV2 {
        self.binding.work()
    }
    /// Bind once per decision, then reuse this immutable snapshot across rows.
    /// This validates and hashes the actual phenotype and rederives homeostasis.
    pub fn bind_current(
        &self,
        phenotype: &CreaturePhenotype,
        biology: &BiochemistryState,
    ) -> Result<NativeNeedBoundCurrentV2, NativeNeedAssociationErrorV2> {
        self.binding.bind_current(phenotype, biology)
    }

    pub fn run(
        &self,
        bound_current: &NativeNeedBoundCurrentV2,
        action: NativeNeedTransitionV1,
        reference: NativeNeedReferenceV1,
    ) -> Result<NativeNeedAssociationOutputV2, NativeNeedAssociationErrorV2> {
        if bound_current.binding_identity() != self.binding.identity() {
            return Err(NativeNeedAssociationErrorV2::BindingMismatch);
        }
        self.run_native(bound_current.drives(), action, reference)
    }

    // No public raw-state bypass. Unit fixtures may exercise this arithmetic
    // directly without fabricating or changing a live physiological state.
    fn run_native(
        &self,
        current: [f32; N],
        action: NativeNeedTransitionV1,
        reference: NativeNeedReferenceV1,
    ) -> Result<NativeNeedAssociationOutputV2, NativeNeedAssociationErrorV2> {
        if current.iter().any(|&v| !unit(v)) {
            return Err(NativeNeedAssociationErrorV2::InvalidInput);
        }
        validate_transition(action)?;
        validate_transition(reference.transition)?;
        let action_delta = action.raw_delta().unwrap_or([0.0; N]);
        let reference_delta = reference.transition.raw_delta().unwrap_or([0.0; N]);
        let mut state = CpuNeuralState::for_brain_class(&brain_spec())?;
        state.activations[..N].copy_from_slice(&current);
        state.activations[N..2 * N].copy_from_slice(&reference_delta);
        state.activations[2 * N..3 * N].copy_from_slice(&action_delta);
        state.activations[CONSTANT] = 1.0;
        let mut finalizer = NativeNeedFinalizerV2::new(self.execution);
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
            support_coefficient(action),
            support_coefficient(reference.transition),
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
        Ok(NativeNeedAssociationOutputV2 {
            binding_identity: self.binding.identity(),
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
) -> Result<(), NativeNeedAssociationErrorV2> {
    let report = cpu_spmv_projection(schema, state, NeuralDiagnostics::reference())?;
    if report.overflow_warnings != 0
        || report.range_rejections != 0
        || report.nan_rejections != 0
        || report.mask_skipped_tiles != 0
        || report.unsupported_tiles != 0
    {
        return Err(NativeNeedAssociationErrorV2::RejectedProjection);
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
struct NativeNeedFinalizerV2 {
    execution: NativeNeedExecutionV1,
    live: Vec<usize>,
    previous_live: Vec<usize>,
}
impl NativeNeedFinalizerV2 {
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

fn validate_transition(value: NativeNeedTransitionV1) -> Result<(), NativeNeedAssociationErrorV2> {
    if let NativeNeedTransitionV1::Learned {
        raw_delta,
        confidence,
        support,
    } = value
    {
        NativeNeedTransitionV1::learned(raw_delta, confidence, support)?;
    }
    Ok(())
}
fn support_coefficient(value: NativeNeedTransitionV1) -> f32 {
    if value.support() == Some(NativeNeedSupportV1::OwnHistoryAssociation) {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
