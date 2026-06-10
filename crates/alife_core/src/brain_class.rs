//! Scalable brain class skeletons.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BrainClassId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainScaleTier {
    Nano512,
    Small1024,
    Standard2048,
    Large4096,
    Cognitive32768,
    Student131k,
    Ascended1M,
    Ascended5M,
    ResearchCustom,
}
