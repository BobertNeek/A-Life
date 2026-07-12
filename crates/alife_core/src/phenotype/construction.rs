//! Deterministic orchestration for immutable GPU phenotype construction.

use crate::{
    ActivationFunction, AlphaStoragePolicy, BrainGenome, GenomeSeedSet, ScaffoldContractError,
};

use super::{
    BrainCapacityClass, BrainPhenotype, CompiledBudgets, GlobalPhenotypeBudgetReceipt,
    NeuronDynamics, PhenotypeCompilerInputs,
};

pub(super) fn compile(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
) -> Result<BrainPhenotype, ScaffoldContractError> {
    inputs.validate_against(capacity)?;
    let genome = inputs.genome();
    let development = inputs.development();
    if !development.open_critical_periods.is_empty() {
        return Err(compile_error());
    }
    validate_supported_inputs(genome, capacity)?;
    let layout = super::layout_compile::compile_layout(
        genome,
        development,
        capacity.execution().max_neurons(),
    )?;
    let encoder =
        super::io_compile::compile_encoder(genome, development, &layout, inputs.sensor_profile())?;
    let (mut projections, mut synapses, mut receipts) =
        super::topology_compile::compile_recurrent(genome, &layout, capacity)?;
    let (decoder, decoder_projection, decoder_synapses, decoder_receipt) =
        super::io_compile::compile_decoder(
            genome,
            development,
            &layout,
            capacity,
            u16::try_from(projections.len()).map_err(|_| compile_error())?,
            u32::try_from(synapses.len()).map_err(|_| compile_error())?,
        )?;
    projections.push(decoder_projection);
    synapses.extend(decoder_synapses);
    receipts.push(decoder_receipt);
    super::topology_compile::validate_alpha_matches(genome, &projections, &synapses, &layout)?;

    let recurrent = receipts
        .iter()
        .map(|receipt| receipt.recurrent_synapses)
        .sum::<u32>();
    let action = receipts
        .iter()
        .map(|receipt| receipt.action_decoder_synapses)
        .sum::<u32>();
    let active_tiles = receipts
        .iter()
        .map(|receipt| receipt.active_tiles)
        .sum::<u32>();
    let total = recurrent.checked_add(action).ok_or_else(compile_error)?;
    let execution = capacity.execution();
    let budgets = CompiledBudgets {
        capacity_class_id: capacity.id(),
        execution_abi_digest: capacity.canonical_digest(),
        routes: receipts,
        global: GlobalPhenotypeBudgetReceipt {
            neuron_count: execution.max_neurons(),
            active_tiles,
            recurrent_synapses: recurrent,
            action_decoder_synapses: action,
            memory_decoder_synapses: 0,
            total_synapses: total,
            immutable_payload_words: total,
            candidate_capacity: execution.max_candidates(),
            object_slot_capacity: execution.max_object_slots(),
            memory_context_capacity: execution.max_memory_context_records(),
            decoder_input_lanes: execution.candidate_feature_count(),
            replay_event_capacity: execution.max_replay_events(),
            replay_eligibility_sample_capacity: execution.max_replay_eligibility_samples(),
        },
    };
    budgets.validate_against(capacity)?;
    let dynamics = (0..execution.max_neurons())
        .map(|_| NeuronDynamics::new(0.0, 0.25, ActivationFunction::Tanh, 0.95, 0.01, 1.0))
        .collect();
    let microstep_count = match development.maturation.raw() {
        value if value < 1.0 / 3.0 => 2,
        value if value < 2.0 / 3.0 => 3,
        _ => 4,
    };
    BrainPhenotype::try_new(
        inputs,
        capacity,
        execution.max_neurons(),
        microstep_count,
        layout,
        projections,
        synapses,
        dynamics,
        encoder,
        decoder,
        budgets,
    )
}

fn validate_supported_inputs(
    genome: &BrainGenome,
    capacity: &BrainCapacityClass,
) -> Result<(), ScaffoldContractError> {
    let expected_seeds = GenomeSeedSet::from_species_seed(genome.species_seed, capacity.id());
    let baseline = BrainGenome::scaffold(genome.species_seed, capacity.id());
    if genome.seeds != expected_seeds
        || genome.genetic_prior_seed != expected_seeds.genetic_prior_seed
        || genome.id.0 != expected_seeds.genome_id_seed
        || genome.plasticity_mask != baseline.plasticity_mask
        || genome.endocrine_constants != baseline.endocrine_constants
        || genome.drive_thresholds != baseline.drive_thresholds
        || genome.mutation_rates != baseline.mutation_rates
        || genome.crossover != baseline.crossover
        || genome.developmental_schedule != baseline.developmental_schedule
        || genome.inheritance != baseline.inheritance
        || genome.alpha_mask.storage_policy != AlphaStoragePolicy::HierarchicalSparse
        || genome.alpha_mask.dense_reference_opt_in
    {
        return Err(compile_error());
    }
    Ok(())
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
