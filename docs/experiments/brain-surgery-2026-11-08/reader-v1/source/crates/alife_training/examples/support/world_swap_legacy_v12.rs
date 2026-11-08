//! Reconstructed wrapper; constructor body verified exact against published source.
use super::care_candidate;
use alife_core::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn construct() -> Result<FoundationWeightAsset> {
    candidate_with_readouts(2026110901, 0.25, true, 12., true)
}

fn candidate_with_readouts(
    seed: u64,
    gain: f32,
    reader: bool,
    memory_gain: f32,
    family_valence_reader: bool,
) -> Result<FoundationWeightAsset> {
    if family_valence_reader && (!reader || memory_gain != 12.) {
        return Err("the family-value factor requires the frozen target reader and cap 12".into());
    }
    let (_, p, source) = if memory_gain == 12. {
        care_candidate::source_memory_gain(seed)?
    } else if memory_gain == 0.5 {
        care_candidate::source(seed, true)?
    } else {
        return Err("unsupported memory gain".into());
    };
    let (mut weights, _) = care_candidate::whole_care(&p)?;
    for (synapse, weight) in p.synapses().iter().zip(&mut weights) {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c)
            if c.head() == DecoderHeadKind::ActionCandidate)
        {
            *weight *= gain;
        }
    }
    if reader {
        let mut memory_counts = std::collections::BTreeMap::new();
        for synapse in p.synapses() {
            if let CompiledSynapseKind::Decoder(c) = synapse.kind() {
                if c.head() == DecoderHeadKind::MemoryContext {
                    *memory_counts
                        .entry((c.family().raw(), c.input_lane()))
                        .or_insert(0usize) += 1;
                }
            }
        }
        for (i, synapse) in p.synapses().iter().enumerate() {
            let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
                continue;
            };
            if c.head() != DecoderHeadKind::MemoryContext {
                continue;
            }
            let total = match (c.family(), c.input_lane()) {
                (CandidateActionFamily::Ingest, 25) => -16.,
                (CandidateActionFamily::Ingest, 26) => -64.,
                (CandidateActionFamily::Ingest, 32) if family_valence_reader => 32.,
                (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 25) => -8.,
                (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 26) => -32.,
                (CandidateActionFamily::Avoid, 25) => 16.,
                (CandidateActionFamily::Avoid, 26) => 128.,
                _ => 0.,
            };
            let copies = memory_counts[&(c.family().raw(), c.input_lane())];
            weights[i] = total / copies as f32;
        }
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        source.manifest().training_stage(),
    )?)
}
