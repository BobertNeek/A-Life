//! Private immutable binding, with a schema-scoped canonical serde encoding.
//! No user state is edited and no missing phenotype meaning is inferred.

use super::*;
use crate::{
    BiochemicalDriveChannel as Drive, BiochemicalPhenotype, BiochemicalSourceLocus as Source,
    BiochemicalTargetLocus as Target, ChemicalSpeciesId, EndocrineProfile, Validate,
};

mod encoding;

/// These are meanings of bounded receptor outputs, not signs of raw chemicals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeNeedDriveRoleV2 {
    Burden,
    Availability,
    Regulatory,
    Opportunity,
    Unbound,
}
pub(super) const PHASE3_ROLES: [NativeNeedDriveRoleV2; N] = {
    use NativeNeedDriveRoleV2::*;
    [
        Burden,
        Burden,
        Burden,
        Burden,
        Burden,
        Regulatory,
        Availability,
        Burden,
        Opportunity,
        Burden,
        Unbound,
    ]
};

/// Binding is non-neural work. Counts describe actual traversals/calls, not
/// fictional recurrent microsteps, elapsed time, or sparse scratch arithmetic.
/// The host must account for this separately once per decision. Encoded bytes
/// are a byte count, not an assertion that every byte is a neural operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeNeedBindingWorkV2 {
    pub phenotype_hash_passes: u32,
    pub encoded_values: u64,
    pub encoded_bytes: u64,
    pub phase3_graph_comparisons: u32,
    pub state_validation_calls: u32,
    pub homeostasis_derivations: u32,
    pub homeostatic_receptor_evaluations: u32,
    pub compared_homeostasis_components: u32,
    pub copied_drive_components: u32,
}

/// Immutable, non-serializable input capability. There is no public constructor
/// from an arbitrary drive vector and no mutable access to its fields.
#[derive(Clone, Debug)]
pub struct NativeNeedBoundCurrentV2 {
    binding_identity: [u64; 4],
    drives: [f32; N],
    source_genome_id: crate::GenomeId,
    source_tick: crate::Tick,
    work: NativeNeedBindingWorkV2,
}
impl NativeNeedBoundCurrentV2 {
    pub const fn binding_identity(&self) -> [u64; 4] {
        self.binding_identity
    }
    pub const fn drives(&self) -> [f32; N] {
        self.drives
    }
    pub const fn source_genome_id(&self) -> crate::GenomeId {
        self.source_genome_id
    }
    pub const fn source_tick(&self) -> crate::Tick {
        self.source_tick
    }
    pub fn work(&self) -> &NativeNeedBindingWorkV2 {
        &self.work
    }
}

#[derive(Clone, Debug)]
pub(super) struct NativeNeedPhenotypeBindingV2 {
    identity: [u64; 4],
    phenotype_identity: [u64; 4],
    work: NativeNeedBindingWorkV2,
}
impl NativeNeedPhenotypeBindingV2 {
    pub(super) fn new(
        phenotype: &CreaturePhenotype,
        roles: &[NativeNeedDriveRoleV2; N],
    ) -> Result<Self, NativeNeedAssociationErrorV2> {
        phenotype.brain_genome.validate_contract()?;
        let graph = &phenotype.chemistry.biochemical;
        graph.validate_contract()?;
        if roles != &PHASE3_ROLES {
            return Err(NativeNeedAssociationErrorV2::InvalidRecipe);
        }
        validate_declared_roles(graph)?;
        // Compare the actual complete expressed graph, not a string or a few
        // lane names. This includes all species bounds/decays, reactions,
        // emitter directions/expression, receptor signs and the value profile.
        // This candidate intentionally does not support mutated chemistry.
        let expected = expected_phase3_graph()?;
        if graph != &expected {
            return Err(NativeNeedAssociationErrorV2::UnsupportedPhenotype(
                "actual expressed graph differs from admitted phase3 chemistry",
            ));
        }
        let (phenotype_identity, mut work) = encoding::phenotype_identity(phenotype)?;
        work.phase3_graph_comparisons = 1;
        let mut digest = CanonicalDigestBuilder::new(
            b"alife.cpu.need-association.phenotype-drive-role-binding.v2",
        );
        for word in phenotype_identity {
            digest.write_u64(word);
        }
        // Explicit role/source/value declaration is part of the identity.
        for (lane, role) in roles.iter().enumerate() {
            digest.write_u8(lane as u8);
            digest.write_u8(match role {
                NativeNeedDriveRoleV2::Burden => 0,
                NativeNeedDriveRoleV2::Availability => 1,
                NativeNeedDriveRoleV2::Regulatory => 2,
                NativeNeedDriveRoleV2::Opportunity => 3,
                NativeNeedDriveRoleV2::Unbound => 4,
            });
            match lane {
                0..=8 => {
                    digest.write_some();
                    digest.write_u16(lane as u16 + 1);
                }
                9 => {
                    digest.write_some();
                    digest.write_u16(22);
                }
                _ => digest.write_none(),
            }
            digest.write_f32(graph.value_profile().drives[lane])?;
        }
        Ok(Self {
            identity: digest.finish256(),
            phenotype_identity,
            work,
        })
    }
    pub(super) const fn identity(&self) -> [u64; 4] {
        self.identity
    }
    pub(super) fn work(&self) -> &NativeNeedBindingWorkV2 {
        &self.work
    }
    pub(super) fn bind_current(
        &self,
        phenotype: &CreaturePhenotype,
        biology: &BiochemistryState,
    ) -> Result<NativeNeedBoundCurrentV2, NativeNeedAssociationErrorV2> {
        let (identity, mut work) = encoding::phenotype_identity(phenotype)?;
        if identity != self.phenotype_identity {
            return Err(NativeNeedAssociationErrorV2::BindingMismatch);
        }
        biology.validate_against(phenotype)?;
        work.state_validation_calls = 1;
        let graph = &phenotype.chemistry.biochemical;
        let derived = biology.graph_state().derive_homeostasis(graph)?;
        work.homeostasis_derivations = 1;
        work.homeostatic_receptor_evaluations = graph
            .receptors()
            .iter()
            .filter(|r| matches!(r.target, Target::Drive(_) | Target::Endocrine(_)))
            .count() as u32;
        // Compare the full homeostatic snapshot, including its graph tick.
        // validate_against alone does not catch forged public drive fields.
        if derived != biology.homeostasis {
            return Err(NativeNeedAssociationErrorV2::BindingMismatch);
        }
        work.compared_homeostasis_components = 23; // tick plus 11+11 scalars
        let drives = derived.drives.to_array();
        if drives.iter().any(|&x| !unit(x)) {
            return Err(NativeNeedAssociationErrorV2::InvalidInput);
        }
        work.copied_drive_components = N as u32;
        Ok(NativeNeedBoundCurrentV2 {
            binding_identity: self.identity,
            drives,
            source_genome_id: biology.source_genome_id,
            source_tick: biology.tick,
            work,
        })
    }
}

fn validate_declared_roles(
    graph: &BiochemicalPhenotype,
) -> Result<(), NativeNeedAssociationErrorV2> {
    let channels = [
        Drive::Hunger,
        Drive::Fatigue,
        Drive::Fear,
        Drive::Pain,
        Drive::Loneliness,
        Drive::Curiosity,
        Drive::BrainAtp,
        Drive::TemperatureStress,
        Drive::Reproductive,
        Drive::Extension0,
        Drive::Extension1,
    ];
    for (lane, channel) in channels.into_iter().enumerate() {
        let mut receptors = graph
            .receptors()
            .iter()
            .filter(|r| r.target == Target::Drive(channel));
        let first = receptors.next();
        let expected_value = match lane {
            0..=4 | 7 => -1.0 / 7.0,
            6 => 1.0 / 7.0,
            9 => -0.08,
            _ => 0.0,
        };
        let matches = if lane == 10 {
            first.is_none()
        } else {
            first.is_some_and(|r| {
                r.source.0 == (if lane == 9 { 22 } else { lane as u16 + 1 })
                    && r.gain == 1.0
                    && r.threshold == 0.0
                    && r.nominal == 0.0
                    && !r.digital
            }) && receptors.next().is_none()
        };
        if !matches || graph.value_profile().drives[lane] != expected_value {
            return Err(NativeNeedAssociationErrorV2::UnsupportedPhenotype(
                "drive receptor/value role",
            ));
        }
    }
    // Pin the new need's meaning independently of a future change to the
    // inherited constructor. Praise remains endocrine species21, not drive9.
    let boredom = graph
        .species()
        .iter()
        .find(|s| s.id == ChemicalSpeciesId(22));
    if !boredom.is_some_and(|s| {
        s.kind == crate::ChemicalSpeciesKind::Regulatory
            && s.minimum == 0.0
            && s.maximum == 1.0
            && s.baseline == 0.0
            && s.decay_retention == 1.0
    }) {
        return Err(NativeNeedAssociationErrorV2::UnsupportedPhenotype(
            "boredom species",
        ));
    }
    let emitters = graph
        .emitters()
        .iter()
        .filter(|e| e.target == ChemicalSpeciesId(22))
        .collect::<Vec<_>>();
    if emitters.len() != 2
        || ![(Source::Awake, 0.0002), (Source::PlayStimulation, -0.2)]
            .into_iter()
            .all(|(source, gain)| {
                emitters.iter().any(|e| {
                    e.source == source
                        && e.gain == gain
                        && e.threshold == 0.0
                        && e.cadence_ticks == 1
                        && e.response == crate::EmitterResponse::Analogue
                        && !e.inverted
                        && e.developmental_expression_floor == 1.0
                })
            })
    {
        return Err(NativeNeedAssociationErrorV2::UnsupportedPhenotype(
            "boredom emitter meaning",
        ));
    }
    Ok(())
}

/// Validation oracle constructed from the existing inherited graph operators.
/// Mirrors only the admitted source calibration in world/new_game.rs, not a
/// second chemical simulator. Actual caller phenotype is still what gets bound.
pub(super) fn expected_phase3_graph() -> Result<BiochemicalPhenotype, ScaffoldContractError> {
    let graph = BiochemicalPhenotype::early_mammal_reference(EndocrineProfile::baseline(), 0.75)?;
    let hunger = graph
        .receptors()
        .iter()
        .find(|r| r.target == Target::Drive(Drive::Hunger))
        .ok_or(ScaffoldContractError::PhenotypeCompile)?
        .source;
    let hunger_emitter = graph
        .emitters()
        .iter()
        .position(|e| e.source == Source::EnergyDeficit && e.target == hunger)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let repair = graph
        .receptors()
        .iter()
        .position(|r| r.target == Target::OrganRepair)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let graph = graph
        .with_emitter_gain(hunger_emitter, 0.03)?
        .with_receptor_nominal(repair, 0.02)?
        .with_waking_recovery()?
        .with_health_distress()?
        .with_player_reward()?
        .with_play_stimulation()?;
    let damage = graph
        .emitters()
        .iter()
        .position(|e| e.source == Source::Damage && e.target == ChemicalSpeciesId(4))
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    graph.with_emitter_expression_floor(damage, 1.0)
}
