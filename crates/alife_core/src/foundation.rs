//! Frozen foundation ABI and N2048 trainable layout contracts.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::blake3_digest::{domain_hasher, Blake3Write};
use crate::{
    ActiveTilePolicy, BiologicalPriority, Blake3Digest, BrainCapacityClass, BrainClassId, LobeKind,
    LobeLayout, ProjectionType, ScaffoldContractError, UpdateCadence,
};
use crate::{LanguageCodebookV1, LobeRegion};

const LAYOUT_DOMAIN: &[u8] = b"alife.foundation.layout-abi.v1";
const ROUTE_DOMAIN: &[u8] = b"alife.foundation.route-abi.v1";
const PLASTICITY_DOMAIN: &[u8] = b"alife.foundation.plasticity-abi.v1";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifetimePlasticityBand {
    Fixed = 0,
    Slow = 1,
    Fast = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationSectionPolicy {
    fixed_synapses: u32,
    slow_synapses: u32,
    fast_synapses: u32,
}

impl FoundationSectionPolicy {
    pub const fn new(fixed_synapses: u32, slow_synapses: u32, fast_synapses: u32) -> Self {
        Self {
            fixed_synapses,
            slow_synapses,
            fast_synapses,
        }
    }

    pub const fn count(self, band: LifetimePlasticityBand) -> u32 {
        match band {
            LifetimePlasticityBand::Fixed => self.fixed_synapses,
            LifetimePlasticityBand::Slow => self.slow_synapses,
            LifetimePlasticityBand::Fast => self.fast_synapses,
        }
    }

    pub const fn total_synapses(self) -> u32 {
        self.fixed_synapses + self.slow_synapses + self.fast_synapses
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct N2048FoundationRouteSpec {
    source_lobe: LobeKind,
    target_lobe: LobeKind,
    synapse_count: u32,
    section_policy: FoundationSectionPolicy,
    projection_type: ProjectionType,
    active_tile_policy: ActiveTilePolicy,
    update_cadence: UpdateCadence,
    priority: BiologicalPriority,
}

impl N2048FoundationRouteSpec {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        source_lobe: LobeKind,
        target_lobe: LobeKind,
        synapse_count: u32,
        section_policy: FoundationSectionPolicy,
        projection_type: ProjectionType,
        update_cadence: UpdateCadence,
        priority: BiologicalPriority,
    ) -> Self {
        Self {
            source_lobe,
            target_lobe,
            synapse_count,
            section_policy,
            projection_type,
            active_tile_policy: ActiveTilePolicy::EssentialReservation,
            update_cadence,
            priority,
        }
    }

    pub const fn source_lobe(self) -> LobeKind {
        self.source_lobe
    }
    pub const fn target_lobe(self) -> LobeKind {
        self.target_lobe
    }
    pub const fn synapse_count(self) -> u32 {
        self.synapse_count
    }
    pub const fn section_policy(self) -> FoundationSectionPolicy {
        self.section_policy
    }
    pub const fn projection_type(self) -> ProjectionType {
        self.projection_type
    }
    pub const fn active_tile_policy(self) -> ActiveTilePolicy {
        self.active_tile_policy
    }
    pub const fn update_cadence(self) -> UpdateCadence {
        self.update_cadence
    }
    pub const fn priority(self) -> BiologicalPriority {
        self.priority
    }
}

const fn slow(count: u32) -> FoundationSectionPolicy {
    FoundationSectionPolicy::new(0, count, 0)
}

const fn fast(count: u32) -> FoundationSectionPolicy {
    FoundationSectionPolicy::new(0, 0, count)
}

const N2048_ROUTES: [N2048FoundationRouteSpec; 16] = [
    N2048FoundationRouteSpec::new(
        LobeKind::SensoryGrounding,
        LobeKind::CoreAssociation,
        3_584,
        slow(3_584),
        ProjectionType::FeedForward,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::AuditorySpeech,
        LobeKind::CoreAssociation,
        1_536,
        slow(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::GlyphVision,
        LobeKind::CoreAssociation,
        1_536,
        slow(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::MetabolicDrive,
        LobeKind::HomeostaticRegulation,
        1_024,
        slow(1_024),
        ProjectionType::Homeostatic,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::CoreAssociation,
        1_024,
        slow(1_024),
        ProjectionType::Modulatory,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::MotorArbitration,
        768,
        slow(768),
        ProjectionType::Homeostatic,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::MotorArbitration,
        3_072,
        FoundationSectionPolicy::new(0, 2_048, 1_024),
        ProjectionType::MotorProposal,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::MotorArbitration,
        LobeKind::MotorArbitration,
        1_536,
        slow(1_536),
        ProjectionType::LateralInhibition,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::WorkingMemory,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::WorkingMemory,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::EpisodicMemory,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::EpisodicMemory,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::LexiconConcept,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::LexiconConcept,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::LexiconConcept,
        LobeKind::WorkingMemory,
        768,
        fast(768),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::WorkingMemory,
        LobeKind::LexiconConcept,
        512,
        fast(512),
        ProjectionType::Feedback,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
];

pub struct N2048FoundationLayoutV1;

impl N2048FoundationLayoutV1 {
    pub const NEURON_COUNT: u32 = 2_048;
    pub const RECURRENT_SYNAPSE_COUNT: u32 = 24_576;
    pub const ACTION_DECODER_SYNAPSE_COUNT: u32 = 4_096;
    pub const CANDIDATE_DECODER_SYNAPSE_COUNT: u32 = 3_072;
    pub const CANDIDATE_FAMILY_COUNT: u16 = 8;
    pub const CANDIDATE_MOTOR_UNITS_PER_FAMILY: u16 = 16;
    pub const SPEECH_DECODER_SYNAPSE_COUNT: u32 = 1_024;
    pub const MEMORY_DECODER_SYNAPSE_COUNT: u32 = 4_096;
    pub const MEMORY_DECODER_INPUT_WIDTH: u16 = 64;
    pub const MEMORY_DECODER_OUTPUT_WIDTH: u16 = 64;

    pub fn lobe_layout() -> LobeLayout {
        let lengths = [256, 128, 128, 128, 256, 448, 256, 128, 224, 96];
        let mut cursor = 0_u32;
        let mut regions = Vec::with_capacity(LobeKind::ALL.len());
        for (kind, len) in LobeKind::CORE.into_iter().zip(lengths) {
            regions.push(LobeRegion::enabled(kind, cursor, len));
            cursor += len;
        }
        for kind in LobeKind::ALL.into_iter().skip(LobeKind::CORE.len()) {
            regions.push(LobeRegion::disabled(kind, cursor));
        }
        let layout = LobeLayout { regions };
        debug_assert!(layout.validate_for_neuron_count(Self::NEURON_COUNT).is_ok());
        layout
    }

    pub const fn route_specs() -> &'static [N2048FoundationRouteSpec] {
        &N2048_ROUTES
    }

    pub fn route_abi_digest() -> Blake3Digest {
        route_digest(Self::route_specs())
    }

    pub fn plasticity_abi_digest() -> Blake3Digest {
        plasticity_digest(Self::route_specs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationLayoutId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundationAbiBinding {
    schema_version: u16,
    capacity_class_id: BrainClassId,
    layout_id: FoundationLayoutId,
    layout_digest: Blake3Digest,
    language_codebook: LanguageCodebookV1,
}

impl FoundationAbiBinding {
    pub fn canonical_for_capacity(
        capacity: &BrainCapacityClass,
    ) -> Result<Self, ScaffoldContractError> {
        capacity.validate_contract()?;
        let layout = if capacity.execution().max_neurons() == N2048FoundationLayoutV1::NEURON_COUNT
        {
            N2048FoundationLayoutV1::lobe_layout()
        } else {
            LobeLayout::reference_for_neuron_count(capacity.execution().max_neurons())?
        };
        Ok(Self {
            schema_version: 1,
            capacity_class_id: capacity.id(),
            layout_id: FoundationLayoutId(0xA11F_0000_0000_0000 | u64::from(capacity.id().raw())),
            layout_digest: layout_digest(&layout),
            language_codebook: LanguageCodebookV1::canonical(),
        })
    }

    pub const fn layout_id(&self) -> FoundationLayoutId {
        self.layout_id
    }
    pub const fn capacity_class_id(&self) -> BrainClassId {
        self.capacity_class_id
    }
    pub const fn layout_digest(&self) -> Blake3Digest {
        self.layout_digest
    }
    pub const fn language_codebook(&self) -> &LanguageCodebookV1 {
        &self.language_codebook
    }

    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        self.language_codebook.validate_contract()?;
        if self.schema_version != 1 || self != &Self::canonical_for_capacity(capacity)? {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for FoundationAbiBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            capacity_class_id: BrainClassId,
            layout_id: FoundationLayoutId,
            layout_digest: Blake3Digest,
            language_codebook: LanguageCodebookV1,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            capacity_class_id: w.capacity_class_id,
            layout_id: w.layout_id,
            layout_digest: w.layout_digest,
            language_codebook: w.language_codebook,
        };
        let capacity = BrainCapacityClass::production_for_id(value.capacity_class_id)
            .map_err(D::Error::custom)?;
        value
            .validate_against(&capacity)
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

pub(crate) fn layout_digest(layout: &LobeLayout) -> Blake3Digest {
    let mut h = domain_hasher(LAYOUT_DOMAIN);
    h.write_len(layout.regions.len());
    for region in &layout.regions {
        h.write_u16(region.kind.raw());
        h.write_u32(region.start);
        h.write_u32(region.len);
        h.write_u8(region.enabled as u8);
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn route_digest(routes: &[N2048FoundationRouteSpec]) -> Blake3Digest {
    let mut h = domain_hasher(ROUTE_DOMAIN);
    h.write_len(routes.len());
    for route in routes {
        h.write_u16(route.source_lobe.raw());
        h.write_u16(route.target_lobe.raw());
        h.write_u32(route.synapse_count);
        h.write_u8(route.projection_type.raw());
        h.write_u8(route.active_tile_policy.raw());
        h.write_u8(route.update_cadence.raw());
        h.write_u8(route.priority.raw());
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn plasticity_digest(routes: &[N2048FoundationRouteSpec]) -> Blake3Digest {
    let mut h = domain_hasher(PLASTICITY_DOMAIN);
    h.write_len(routes.len());
    for route in routes {
        h.write_u16(route.source_lobe.raw());
        h.write_u16(route.target_lobe.raw());
        for band in [
            LifetimePlasticityBand::Fixed,
            LifetimePlasticityBand::Slow,
            LifetimePlasticityBand::Fast,
        ] {
            h.write_u8(band as u8);
            h.write_u32(route.section_policy.count(band));
        }
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn procedural_route_digest(
    routes: impl IntoIterator<Item = (LobeKind, LobeKind, u32)>,
) -> Blake3Digest {
    let rows = routes.into_iter().collect::<Vec<_>>();
    let mut h = domain_hasher(ROUTE_DOMAIN);
    h.write_len(rows.len());
    for (source, target, count) in rows {
        h.write_u16(source.raw());
        h.write_u16(target.raw());
        h.write_u32(count);
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn procedural_plasticity_digest(
    routes: impl IntoIterator<Item = (u16, u32, u32)>,
) -> Blake3Digest {
    let rows = routes.into_iter().collect::<Vec<_>>();
    let mut h = domain_hasher(PLASTICITY_DOMAIN);
    h.write_len(rows.len());
    for (route, count, alpha_bits) in rows {
        h.write_u16(route);
        h.write_u32(count);
        h.write_u32(alpha_bits);
    }
    Blake3Digest::from_hasher(h)
}
