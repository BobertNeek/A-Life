//! Persistent logical neural addresses independent of runtime packing.

use serde::{Deserialize, Serialize};

use crate::blake3_digest::{domain_hasher, Blake3Write};
use crate::{Blake3Digest, LobeKind, LobeLayout, ScaffoldContractError};

use super::{
    BrainPhenotype, CompiledProjection, CompiledSynapse, CompiledSynapseKind, DecoderHeadKind,
};

const ADDRESS_MAP_DOMAIN: &[u8] = b"alife.phenotype.persistent-address-map.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistentNeuronAddress {
    pub lobe: LobeKind,
    pub ordinal: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PersistentProjectionRole {
    Recurrent = 0,
    ActionAndSpeechDecoder = 1,
    MemoryDecoder = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistentProjectionAddress {
    pub source_lobe: LobeKind,
    pub target_lobe: LobeKind,
    pub role: PersistentProjectionRole,
    pub logical_ordinal: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistentDecoderAddress {
    pub head: DecoderHeadKind,
    pub logical_group: u8,
    pub input_lane: u16,
    pub output_index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistentSynapseAddress {
    pub projection: PersistentProjectionAddress,
    pub source: PersistentNeuronAddress,
    pub target: PersistentNeuronAddress,
    pub decoder: Option<PersistentDecoderAddress>,
    pub duplicate_ordinal: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentNeuronAddressEntry {
    address: PersistentNeuronAddress,
    packed_index: u32,
}

impl PersistentNeuronAddressEntry {
    pub const fn address(&self) -> PersistentNeuronAddress {
        self.address
    }
    pub const fn packed_index(&self) -> u32 {
        self.packed_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentProjectionAddressEntry {
    address: PersistentProjectionAddress,
    packed_index: u16,
}

impl PersistentProjectionAddressEntry {
    pub const fn address(&self) -> PersistentProjectionAddress {
        self.address
    }
    pub const fn packed_index(&self) -> u16 {
        self.packed_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentSynapseAddressEntry {
    address: PersistentSynapseAddress,
    packed_index: u32,
}

impl PersistentSynapseAddressEntry {
    pub const fn address(&self) -> PersistentSynapseAddress {
        self.address
    }
    pub const fn packed_index(&self) -> u32 {
        self.packed_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentDecoderAddressEntry {
    address: PersistentDecoderAddress,
    synapse_address: PersistentSynapseAddress,
    packed_synapse_index: u32,
}

impl PersistentDecoderAddressEntry {
    pub const fn address(&self) -> PersistentDecoderAddress {
        self.address
    }
    pub const fn synapse_address(&self) -> PersistentSynapseAddress {
        self.synapse_address
    }
    pub const fn packed_synapse_index(&self) -> u32 {
        self.packed_synapse_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentAddressMap {
    schema_version: u16,
    neurons: Vec<PersistentNeuronAddressEntry>,
    projections: Vec<PersistentProjectionAddressEntry>,
    synapses: Vec<PersistentSynapseAddressEntry>,
    decoders: Vec<PersistentDecoderAddressEntry>,
    digest: Blake3Digest,
}

impl PersistentAddressMap {
    pub(super) fn compile(
        layout: &LobeLayout,
        projections: &[CompiledProjection],
        synapses: &[CompiledSynapse],
    ) -> Result<Self, ScaffoldContractError> {
        let mut neurons = Vec::with_capacity(layout.total_neurons() as usize);
        for region in layout.iter_regions().filter(|region| region.enabled) {
            for ordinal in 0..region.len {
                neurons.push(PersistentNeuronAddressEntry {
                    address: PersistentNeuronAddress {
                        lobe: region.kind,
                        ordinal,
                    },
                    packed_index: region.start + ordinal,
                });
            }
        }
        neurons.sort_by_key(|entry| entry.address);

        let mut projection_rows = Vec::with_capacity(projections.len());
        for (packed_index, projection) in projections.iter().enumerate() {
            let (start, len) = projection.synapse_range();
            let slice = synapses
                .get(start as usize..(start + len) as usize)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let role = projection_role(slice)?;
            projection_rows.push((
                packed_index,
                PersistentProjectionAddress {
                    source_lobe: projection.source_lobe(),
                    target_lobe: projection.target_lobe(),
                    role,
                    logical_ordinal: 0,
                },
            ));
        }
        projection_rows.sort_by_key(|(_, address)| *address);
        let mut prior_key = None;
        let mut logical_ordinal = 0_u16;
        for (_, address) in &mut projection_rows {
            let key = (address.source_lobe, address.target_lobe, address.role);
            if prior_key == Some(key) {
                logical_ordinal = logical_ordinal
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            } else {
                prior_key = Some(key);
                logical_ordinal = 0;
            }
            address.logical_ordinal = logical_ordinal;
        }
        let projection_address_by_packed = projection_rows
            .iter()
            .map(|(packed, address)| (*packed, *address))
            .collect::<std::collections::BTreeMap<_, _>>();
        let projections = projection_rows
            .into_iter()
            .map(|(packed, address)| {
                Ok(PersistentProjectionAddressEntry {
                    address,
                    packed_index: u16::try_from(packed)
                        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;

        let mut synapse_rows = Vec::with_capacity(synapses.len());
        for (packed_index, synapse) in synapses.iter().enumerate() {
            let projection = *projection_address_by_packed
                .get(&usize::from(synapse.route_index()))
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let source = neuron_address(layout, synapse.source())?;
            let target = neuron_address(layout, synapse.target())?;
            let decoder = match synapse.kind() {
                CompiledSynapseKind::Recurrent => None,
                CompiledSynapseKind::Decoder(coordinate) => Some(PersistentDecoderAddress {
                    head: coordinate.head(),
                    logical_group: coordinate.family().raw(),
                    input_lane: coordinate.input_lane(),
                    output_index: coordinate.motor_index(),
                }),
            };
            synapse_rows.push((
                packed_index,
                PersistentSynapseAddress {
                    projection,
                    source,
                    target,
                    decoder,
                    duplicate_ordinal: 0,
                },
            ));
        }
        synapse_rows.sort_by_key(|(_, address)| *address);
        let mut prior_base = None;
        let mut duplicate_ordinal = 0_u16;
        for (_, address) in &mut synapse_rows {
            let base = (
                address.projection,
                address.source,
                address.target,
                address.decoder,
            );
            if prior_base == Some(base) {
                duplicate_ordinal = duplicate_ordinal
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?;
                address.duplicate_ordinal = duplicate_ordinal;
            } else {
                prior_base = Some(base);
                duplicate_ordinal = 0;
            }
        }
        let synapses = synapse_rows
            .iter()
            .map(|(packed, address)| {
                Ok(PersistentSynapseAddressEntry {
                    address: *address,
                    packed_index: u32::try_from(*packed)
                        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let decoders = synapse_rows
            .iter()
            .filter_map(|(packed, address)| {
                address.decoder.map(|decoder| {
                    Ok(PersistentDecoderAddressEntry {
                        address: decoder,
                        synapse_address: *address,
                        packed_synapse_index: u32::try_from(*packed)
                            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
                    })
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;

        let mut value = Self {
            schema_version: 1,
            neurons,
            projections,
            synapses,
            decoders,
            digest: Blake3Digest::default(),
        };
        value.digest = value.recompute_digest()?;
        Ok(value)
    }

    pub fn neurons(&self) -> &[PersistentNeuronAddressEntry] {
        &self.neurons
    }
    pub fn projections(&self) -> &[PersistentProjectionAddressEntry] {
        &self.projections
    }
    pub fn synapses(&self) -> &[PersistentSynapseAddressEntry] {
        &self.synapses
    }
    pub fn decoders(&self) -> &[PersistentDecoderAddressEntry] {
        &self.decoders
    }
    pub const fn digest(&self) -> Blake3Digest {
        self.digest
    }

    pub fn recompute_digest(&self) -> Result<Blake3Digest, ScaffoldContractError> {
        if self.schema_version != 1 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut h = domain_hasher(ADDRESS_MAP_DOMAIN);
        h.write_u16(self.schema_version);
        h.write_len(self.neurons.len());
        for entry in &self.neurons {
            encode_neuron(&mut h, entry.address);
        }
        h.write_len(self.projections.len());
        for entry in &self.projections {
            encode_projection(&mut h, entry.address);
        }
        h.write_len(self.synapses.len());
        for entry in &self.synapses {
            encode_synapse(&mut h, entry.address);
        }
        h.write_len(self.decoders.len());
        for entry in &self.decoders {
            encode_decoder(&mut h, entry.address);
            encode_synapse(&mut h, entry.synapse_address);
        }
        Ok(Blake3Digest::from_hasher(h))
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        if self.digest != self.recompute_digest()?
            || self
                != &Self::compile(
                    phenotype.lobe_layout(),
                    phenotype.projections(),
                    phenotype.synapses(),
                )?
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

fn projection_role(
    synapses: &[CompiledSynapse],
) -> Result<PersistentProjectionRole, ScaffoldContractError> {
    let mut role = None;
    for synapse in synapses {
        let current = match synapse.kind() {
            CompiledSynapseKind::Recurrent => PersistentProjectionRole::Recurrent,
            CompiledSynapseKind::Decoder(coordinate)
                if matches!(
                    coordinate.head(),
                    DecoderHeadKind::ActionCandidate | DecoderHeadKind::SpeechPayload
                ) =>
            {
                PersistentProjectionRole::ActionAndSpeechDecoder
            }
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::MemoryContext =>
            {
                PersistentProjectionRole::MemoryDecoder
            }
            _ => return Err(ScaffoldContractError::PhenotypeCompile),
        };
        if role.is_some_and(|prior| prior != current) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        role = Some(current);
    }
    role.ok_or(ScaffoldContractError::PhenotypeCompile)
}

fn neuron_address(
    layout: &LobeLayout,
    packed_index: u32,
) -> Result<PersistentNeuronAddress, ScaffoldContractError> {
    let region = layout
        .lobe_by_neuron_index(packed_index)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    Ok(PersistentNeuronAddress {
        lobe: region.kind,
        ordinal: packed_index - region.start,
    })
}

fn encode_neuron(h: &mut blake3::Hasher, address: PersistentNeuronAddress) {
    h.write_u16(address.lobe.raw());
    h.write_u32(address.ordinal);
}

fn encode_projection(h: &mut blake3::Hasher, address: PersistentProjectionAddress) {
    h.write_u16(address.source_lobe.raw());
    h.write_u16(address.target_lobe.raw());
    h.write_u8(address.role as u8);
    h.write_u16(address.logical_ordinal);
}

fn encode_decoder(h: &mut blake3::Hasher, address: PersistentDecoderAddress) {
    h.write_u8(address.head.raw());
    h.write_u8(address.logical_group);
    h.write_u16(address.input_lane);
    h.write_u16(address.output_index);
}

fn encode_synapse(h: &mut blake3::Hasher, address: PersistentSynapseAddress) {
    encode_projection(h, address.projection);
    encode_neuron(h, address.source);
    encode_neuron(h, address.target);
    match address.decoder {
        Some(decoder) => {
            h.write_u8(1);
            encode_decoder(h, decoder);
        }
        None => h.write_u8(0),
    }
    h.write_u16(address.duplicate_ordinal);
}
