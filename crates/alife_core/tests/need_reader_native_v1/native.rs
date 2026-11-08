//! Fixture compiler and staged execution of real core operators. No scalar
//! pressure/burden/evaluate call occurs in this module. No acquired state enters.
use std::collections::BTreeMap;

use super::scalar_reference::{ModelError, Recipe, Role, SyntheticEvidence};
use alife_core::*;

pub(super) const N: usize = 11;
pub(super) const K: usize = 4;
const CONSTANT: usize = 22;
const SCHEMA: u16 = 1;
const FOUNDATION: &str = "detached-reference-species22-need-reader-v1";
const CHEMISTRY: &str = "reference-drive-roles-v1-atp-availability";
const BOREDOM: &str = "species22/play-stimulation/drive-extension0/boredom/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum GeneId {
    Knot(u8),
    Basis(u8),
    Magnitude(u8),
    AvailabilityTarget(u8),
    Gain,
    Ceiling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Current,
    Successor,
}

/// Stable logical branch identity, never a persistent or packed neuron offset.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BranchIdentity {
    pub lane: u8,
    pub role: Role,
    pub side: Side,
    pub basis: u8,
    pub threshold_gene: GeneId,
    pub weight_gene: GeneId,
}

pub(super) struct Gene {
    pub recipe: Recipe,
    pub typed_boredom_binding: Option<&'static str>,
}
impl Gene {
    pub fn reference() -> Self {
        Self {
            recipe: Recipe::reference(),
            typed_boredom_binding: None,
        }
    }
    pub fn typed_boredom() -> Self {
        let mut gene = Self::reference();
        gene.recipe.roles[9] = Role::TypedBoredomFixture;
        gene.recipe.magnitudes[9] = 0.08;
        gene.typed_boredom_binding = Some(BOREDOM);
        gene
    }
    fn validate(&self) -> Result<(), ModelError> {
        self.recipe.validate()?;
        // This is a reference-species fixture, not authority to reinterpret an
        // evolved chemistry. Informational extensions cannot evade typed binding.
        if self
            .recipe
            .roles
            .iter()
            .enumerate()
            .any(|(lane, role)| !match lane {
                0 | 1 | 2 | 3 | 4 | 7 => matches!(role, Role::Burden),
                6 => matches!(role, Role::Availability { .. }),
                9 => matches!(role, Role::Informational | Role::TypedBoredomFixture),
                _ => matches!(role, Role::Informational),
            })
        {
            return Err(ModelError::InvalidRecipe);
        }
        if self.recipe.roles.iter().enumerate().any(|(i, role)| {
            *role == Role::TypedBoredomFixture
                && (i != 9 || self.typed_boredom_binding != Some(BOREDOM))
        }) || (self.typed_boredom_binding.is_some()
            && (self.typed_boredom_binding != Some(BOREDOM)
                || self.recipe.roles[9] != Role::TypedBoredomFixture))
        {
            return Err(ModelError::InvalidRecipe);
        }
        Ok(())
    }
}

pub(super) struct Circuit {
    pub recipe: Recipe,
    pub genes: BTreeMap<GeneId, f32>,
    pub identity: [u64; 4],
    pub branches: DendriticBranchSet,
    pub branch_ids: Vec<BranchIdentity>,
    pub projections: Vec<NeuralProjectionSchema>,
}

fn projection(edges: &[(usize, usize, f32)]) -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    let mut tiles: BTreeMap<(u32, u32), Vec<CooEntry>> = BTreeMap::new();
    for &(target, source, weight) in edges {
        let coord = SparseTileCoord::from_neuron_indices(target as u32, source as u32).unwrap();
        tiles
            .entry((coord.microtile_row, coord.microtile_col))
            .or_default()
            .push(
                CooEntry::new(
                    (target % 16) as u8,
                    (source % 16) as u8,
                    SynapseWeightSplit::new(weight, 0.0, 0.0, 0.0, 0.0).unwrap(),
                )
                .unwrap(),
            );
    }
    for ((row, col), entries) in tiles {
        schema.projections[0].tiles.push(ProjectionTile::new_coo(
            0,
            SparseTileCoord::new(row, col).unwrap(),
            CooTile::new(entries).unwrap(),
        ));
    }
    schema.rebuild_supertile_masks();
    schema.validate().unwrap();
    schema
}

impl Circuit {
    pub fn compile(gene: Gene) -> Result<Self, ModelError> {
        gene.validate()?;
        let recipe = gene.recipe;
        let mut genes = BTreeMap::new();
        for k in 0..K {
            genes.insert(GeneId::Knot(k as u8), recipe.knots[k]);
            genes.insert(GeneId::Basis(k as u8), recipe.basis[k]);
        }
        for (lane, role) in recipe.roles.iter().enumerate() {
            genes.insert(GeneId::Magnitude(lane as u8), recipe.magnitudes[lane]);
            if let Role::Availability { target } = role {
                genes.insert(GeneId::AvailabilityTarget(lane as u8), *target);
            }
        }
        genes.insert(GeneId::Gain, recipe.gain);
        genes.insert(GeneId::Ceiling, recipe.ceiling);
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-DETACHED-NEED-READER-TEST-V1");
        digest.write_u16(SCHEMA);
        digest.write_bytes(FOUNDATION.as_bytes());
        digest.write_bytes(CHEMISTRY.as_bytes());
        digest.write_bytes(gene.typed_boredom_binding.unwrap_or("").as_bytes());
        for (lane, role) in recipe.roles.iter().enumerate() {
            digest.write_u64(lane as u64);
            digest.write_u16(match role {
                Role::Burden => 1,
                Role::Availability { .. } => 2,
                Role::Informational => 3,
                Role::TypedBoredomFixture => 4,
            });
        }
        for (id, value) in &genes {
            let (tag, index) = match *id {
                GeneId::Knot(i) => (1, i),
                GeneId::Basis(i) => (2, i),
                GeneId::Magnitude(i) => (3, i),
                GeneId::AvailabilityTarget(i) => (4, i),
                GeneId::Gain => (5, 0),
                GeneId::Ceiling => (6, 0),
            };
            digest.write_u16(tag);
            digest.write_u16(u16::from(index));
            digest.write_u32(value.to_bits());
        }
        let mut affine = Vec::new();
        let mut pressure = Vec::new();
        let mut difference = Vec::new();
        let mut weighted = Vec::new();
        let mut branches = Vec::new();
        let mut branch_ids = Vec::new();
        for lane in 0..N {
            affine.extend([
                (lane, lane, 1.0),
                (N + lane, lane, 1.0),
                (N + lane, N + lane, 1.0),
            ]);
            difference.extend([(lane, lane, 1.0), (lane, N + lane, -1.0)]);
            weighted.push((0, lane, genes[&GeneId::Magnitude(lane as u8)]));
        }
        affine.push((CONSTANT, CONSTANT, 1.0));
        for side in [Side::Current, Side::Successor] {
            for lane in 0..N {
                let index = lane + if side == Side::Current { 0 } else { N };
                let (weight, intercept) = match recipe.roles[lane] {
                    Role::Burden | Role::TypedBoredomFixture => (1.0, 0.0),
                    Role::Availability { .. } => {
                        (-1.0, genes[&GeneId::AvailabilityTarget(lane as u8)])
                    }
                    Role::Informational => (0.0, 0.0),
                };
                // Keep zero edges: matched gain/status/role controls do identical work.
                pressure.extend([(index, index, weight), (index, CONSTANT, intercept)]);
                for basis in 0..K {
                    let threshold_gene = GeneId::Knot(basis as u8);
                    let weight_gene = GeneId::Basis(basis as u8);
                    branches.push(
                        DendriticBranch::new(
                            index as u32,
                            genes[&threshold_gene],
                            genes[&weight_gene],
                            vec![DendriticInputRef::new(index as u32, 1.0).unwrap()],
                        )
                        .unwrap(),
                    );
                    branch_ids.push(BranchIdentity {
                        lane: lane as u8,
                        role: recipe.roles[lane],
                        side,
                        basis: basis as u8,
                        threshold_gene,
                        weight_gene,
                    });
                }
            }
        }
        Ok(Self {
            recipe,
            genes,
            identity: digest.finish256(),
            branches: DendriticBranchSet::new(branches).unwrap(),
            branch_ids,
            projections: vec![
                projection(&affine),
                projection(&pressure),
                projection(&difference),
                projection(&weighted),
                projection(&[(0, 0, recipe.gain)]),
            ],
        })
    }

    pub fn run(
        &self,
        current: [f32; N],
        evidence: SyntheticEvidence,
    ) -> Result<Output, ModelError> {
        let unit = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        if current.iter().any(|&v| !unit(v)) {
            return Err(ModelError::InvalidInput);
        }
        let (known, supported, delta, confidence, age, similarity) = match evidence {
            SyntheticEvidence::Unknown => (false, false, [0.0; N], 0.0, None, None),
            SyntheticEvidence::MeasuredFixture {
                delta,
                confidence,
                age_ticks,
                similarity,
                arithmetic_supported,
            } => {
                if delta
                    .iter()
                    .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
                    || !unit(confidence)
                    || similarity.is_some_and(|v| !unit(v))
                {
                    return Err(ModelError::InvalidInput);
                }
                (
                    true,
                    arithmetic_supported,
                    delta,
                    confidence,
                    Some(age_ticks),
                    similarity,
                )
            }
        };
        // Scratch only: no persistent brain, lifetime weights, optimizer or bank accepted.
        let mut state =
            CpuNeuralState::for_brain_class(&BrainClassSpec::for_tier(BrainScaleTier::Nano512))
                .unwrap();
        state.activations[..N].copy_from_slice(&current);
        state.activations[N..2 * N].copy_from_slice(&delta);
        state.activations[CONSTANT] = 1.0;
        let mut work = Work::default();
        project(&self.projections[0], &mut state, &mut work);
        finalize(
            &mut state,
            ActivationFunction::Identity,
            0.0,
            1.0,
            &mut work,
        );
        let successor = state.activations[N..2 * N].try_into().unwrap();
        let clipped_successor =
            std::array::from_fn(|lane| current[lane] + delta[lane] != state.activations[N + lane]);
        project(&self.projections[1], &mut state, &mut work);
        finalize(&mut state, ActivationFunction::Relu, 0.0, 1.0, &mut work);
        let pressure: [f32; 2 * N] = state.activations[..2 * N].try_into().unwrap();
        // These are the actual core dendritic conjunctions, not oracle outputs.
        work.dendritic = apply_dendritic_conjunctions(
            &state.activations,
            &mut state.accumulators,
            &self.branches,
        )
        .unwrap();
        finalize(
            &mut state,
            ActivationFunction::Identity,
            0.0,
            1.0,
            &mut work,
        );
        let burden: [f32; 2 * N] = state.activations[..2 * N].try_into().unwrap();
        project(&self.projections[2], &mut state, &mut work);
        finalize(
            &mut state,
            ActivationFunction::Identity,
            -1.0,
            1.0,
            &mut work,
        );
        let association: [f32; N] = state.activations[..N].try_into().unwrap();
        project(&self.projections[3], &mut state, &mut work);
        finalize(
            &mut state,
            ActivationFunction::Identity,
            -2.0,
            2.0,
            &mut work,
        );
        project(&self.projections[4], &mut state, &mut work);
        let unclipped_readout = state.accumulators[0];
        finalize(
            &mut state,
            ActivationFunction::Identity,
            -self.recipe.ceiling,
            self.recipe.ceiling,
            &mut work,
        );
        let native_readout = state.activations[0];
        // Diagnostic only. Dynamic confidence/status gates have NOT been compiled.
        let diagnostic_supported = if known && supported {
            native_readout
        } else {
            0.0
        };
        Ok(Output {
            current,
            delta,
            successor,
            pressure,
            burden,
            association,
            native_readout,
            diagnostic_supported,
            diagnostic_confidence_gated: diagnostic_supported * confidence,
            known,
            supported,
            confidence,
            age,
            similarity,
            clipped_successor,
            readout_clipped: unclipped_readout != native_readout,
            work,
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Work {
    pub projections: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub finalizations: u32,
    pub activation_cells: u32,
    pub dendritic: DendriticWorkReceipt,
}
#[derive(Debug)]
pub(super) struct Output {
    pub current: [f32; N],
    pub delta: [f32; N],
    pub successor: [f32; N],
    pub pressure: [f32; 2 * N],
    pub burden: [f32; 2 * N],
    pub association: [f32; N],
    pub native_readout: f32,
    pub diagnostic_supported: f32,
    pub diagnostic_confidence_gated: f32,
    pub known: bool,
    pub supported: bool,
    pub confidence: f32,
    pub age: Option<u64>,
    pub similarity: Option<f32>,
    pub clipped_successor: [bool; N],
    pub readout_clipped: bool,
    pub work: Work,
}
fn project(schema: &NeuralProjectionSchema, state: &mut CpuNeuralState, work: &mut Work) {
    let report = cpu_spmv_projection(schema, state, NeuralDiagnostics::reference()).unwrap();
    assert_eq!(report.overflow_warnings, 0);
    assert_eq!(report.range_rejections, 0);
    assert_eq!(report.nan_rejections, 0);
    assert_eq!(report.mask_skipped_tiles, 0);
    work.projections += 1;
    work.active_tiles += report.active_tiles;
    work.active_synapses += report.active_synapses;
}
fn finalize(
    state: &mut CpuNeuralState,
    function: ActivationFunction,
    min: f32,
    max: f32,
    work: &mut Work,
) {
    finalize_cpu_activations(
        state,
        NeuralActivationConfig {
            function,
            clamp_min: min,
            clamp_max: max,
            clear_accumulators: true,
        },
    )
    .unwrap();
    work.finalizations += 1;
    work.activation_cells += state.neuron_count;
}
