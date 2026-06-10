pub mod profiles;

pub const NEURON_COUNT_PER_BRAIN: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuronLobe {
    Sensory,
    Associative,
    EpisodicMemory,
    Value,
    Motor,
    Homeostatic,
}

impl NeuronLobe {
    pub const ALL: [NeuronLobe; 6] = [
        NeuronLobe::Sensory,
        NeuronLobe::Associative,
        NeuronLobe::EpisodicMemory,
        NeuronLobe::Value,
        NeuronLobe::Motor,
        NeuronLobe::Homeostatic,
    ];

    pub const fn neuron_range(self) -> core::ops::Range<u32> {
        match self {
            NeuronLobe::Sensory => 0..384,
            NeuronLobe::Associative => 384..1152,
            NeuronLobe::EpisodicMemory => 1152..1536,
            NeuronLobe::Value => 1536..1664,
            NeuronLobe::Motor => 1664..1920,
            NeuronLobe::Homeostatic => 1920..NEURON_COUNT_PER_BRAIN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperiencePatchPhase {
    Ingest,
    Activate,
    Consolidate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperiencePatch {
    pub organism_id: u64,
    pub sequence_id: u64,
    pub phase: ExperiencePatchPhase,
    pub active_microtiles: Vec<u16>,
}

impl ExperiencePatch {
    pub fn new(organism_id: u64, sequence_id: u64) -> Self {
        Self {
            organism_id,
            sequence_id,
            phase: ExperiencePatchPhase::Ingest,
            active_microtiles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionCommand {
    pub organism_id: u64,
    pub motor_intent: MotorIntent,
}

impl ActionCommand {
    pub const fn hold_position(organism_id: u64) -> Self {
        Self {
            organism_id,
            motor_intent: MotorIntent::HoldPosition,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorIntent {
    HoldPosition,
    Move { heading_quantum: i16, intensity: u8 },
    Interact { target_slot: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WGeneticFixed {
    pub neuron_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WConsolidatedHabit {
    pub neuron_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HOperational {
    pub neuron_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralWeightSplits {
    pub neuron_count: u32,
    pub w_genetic_fixed: WGeneticFixed,
    pub w_consolidated_habit: WConsolidatedHabit,
    pub h_operational: HOperational,
}

impl NeuralWeightSplits {
    pub fn new(neuron_count: u32) -> Self {
        Self {
            neuron_count,
            // W_genetic_fixed stores immutable inherited priors and must never be mutated.
            w_genetic_fixed: WGeneticFixed { neuron_count },
            w_consolidated_habit: WConsolidatedHabit { neuron_count },
            h_operational: HOperational { neuron_count },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::profiles::{BrainResidencyState, GPU_PROFILE_HIGH_8GB, GPU_PROFILE_MINIMUM_2GB};
    use crate::{
        ActionCommand, ExperiencePatch, NEURON_COUNT_PER_BRAIN, NeuralWeightSplits, NeuronLobe,
    };

    #[test]
    fn minimum_profile_reserves_exactly_two_gb_of_gpu_memory() {
        assert_eq!(GPU_PROFILE_MINIMUM_2GB.profile_name, "Minimum 2GB");
        assert_eq!(GPU_PROFILE_MINIMUM_2GB.total_gpu_memory_budget_mb, 2048);
        assert_eq!(GPU_PROFILE_MINIMUM_2GB.neuron_count_per_brain, 2048);
        assert_eq!(GPU_PROFILE_MINIMUM_2GB.max_world_organisms, 500);
    }

    #[test]
    fn high_profile_keeps_all_world_organisms_hot() {
        assert_eq!(GPU_PROFILE_HIGH_8GB.profile_name, "High 8GB");
        assert_eq!(GPU_PROFILE_HIGH_8GB.total_gpu_memory_budget_mb, 8192);
        assert_eq!(GPU_PROFILE_HIGH_8GB.max_hot_brain_slots, 500);
        assert_eq!(GPU_PROFILE_HIGH_8GB.max_warm_brain_slots, 0);
    }

    #[test]
    fn residency_states_cover_hot_warm_cold_sleep_and_dormant_brains() {
        let states = [
            BrainResidencyState::HotGpu60Hz,
            BrainResidencyState::WarmGpuTimeSliced,
            BrainResidencyState::ColdHostCompressed,
            BrainResidencyState::SleepCompactionGpu,
            BrainResidencyState::DormantDiskBacked,
        ];

        assert_eq!(states.len(), 5);
    }

    #[test]
    fn neural_runtime_stubs_preserve_fixed_shape_brain_contract() {
        let patch = ExperiencePatch::new(42, 7);
        let command = ActionCommand::hold_position(42);
        let splits = NeuralWeightSplits::new(NEURON_COUNT_PER_BRAIN);

        assert_eq!(patch.organism_id, 42);
        assert_eq!(patch.sequence_id, 7);
        assert_eq!(command.organism_id, 42);
        assert_eq!(splits.neuron_count, 2048);
        assert_eq!(NeuronLobe::ALL.len(), 6);
    }
}
