use alife::profiles::GPU_PROFILE_MINIMUM_2GB;
use alife::{ExperiencePatch, NEURON_COUNT_PER_BRAIN, NeuralWeightSplits};

fn main() {
    let profile = GPU_PROFILE_MINIMUM_2GB;
    let organism_count = 8_u32.min(profile.max_hot_brain_slots);
    let _weights = NeuralWeightSplits::new(NEURON_COUNT_PER_BRAIN);
    let patches: Vec<_> = (0..organism_count)
        .map(|organism_id| ExperiencePatch::new(u64::from(organism_id), 0))
        .collect();

    println!(
        "A-Life demo: {} organisms at {} using {} neurons per brain and {} patches",
        organism_count,
        profile.profile_name,
        profile.neuron_count_per_brain,
        patches.len()
    );
}
