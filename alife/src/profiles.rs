#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainResidencyState {
    HotGpu60Hz,
    WarmGpuTimeSliced,
    ColdHostCompressed,
    SleepCompactionGpu,
    DormantDiskBacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemoryProfileManifest {
    pub profile_name: &'static str,
    pub total_gpu_memory_budget_mb: u32,
    pub renderer_reserve_mb: u32,
    pub neural_heap_mb: u32,
    pub scratch_heap_mb: u32,
    pub replay_heap_mb: u32,
    pub sensory_cache_mb: u32,
    pub slm_cache_mb: u32,
    pub max_world_organisms: u32,
    pub max_hot_brain_slots: u32,
    pub max_warm_brain_slots: u32,
    pub neuron_count_per_brain: u32,
    pub max_active_microtiles_per_brain: u32,
    pub max_active_synapses_per_brain: u32,
    pub sleep_compaction_jobs: u32,
    pub structural_recompaction_jobs: u32,
}

pub const GPU_PROFILE_MINIMUM_2GB: GpuMemoryProfileManifest = GpuMemoryProfileManifest {
    profile_name: "Minimum 2GB",
    total_gpu_memory_budget_mb: 2048,
    renderer_reserve_mb: 384,
    neural_heap_mb: 1152,
    scratch_heap_mb: 192,
    replay_heap_mb: 128,
    sensory_cache_mb: 128,
    slm_cache_mb: 64,
    max_world_organisms: 500,
    max_hot_brain_slots: 96,
    max_warm_brain_slots: 192,
    neuron_count_per_brain: 2048,
    max_active_microtiles_per_brain: 128,
    max_active_synapses_per_brain: 16_384,
    sleep_compaction_jobs: 4,
    structural_recompaction_jobs: 2,
};

pub const GPU_PROFILE_HIGH_8GB: GpuMemoryProfileManifest = GpuMemoryProfileManifest {
    profile_name: "High 8GB",
    total_gpu_memory_budget_mb: 8192,
    renderer_reserve_mb: 768,
    neural_heap_mb: 5632,
    scratch_heap_mb: 768,
    replay_heap_mb: 512,
    sensory_cache_mb: 384,
    slm_cache_mb: 128,
    max_world_organisms: 500,
    max_hot_brain_slots: 500,
    max_warm_brain_slots: 0,
    neuron_count_per_brain: 2048,
    max_active_microtiles_per_brain: 512,
    max_active_synapses_per_brain: 65_536,
    sleep_compaction_jobs: 32,
    structural_recompaction_jobs: 16,
};

pub const GPU_PROFILES: &[GpuMemoryProfileManifest] =
    &[GPU_PROFILE_MINIMUM_2GB, GPU_PROFILE_HIGH_8GB];
