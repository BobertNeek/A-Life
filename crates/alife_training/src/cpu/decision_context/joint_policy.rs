//! Detached, versioned joint policy; never installs a runtime sampler.
use super::{invalid, TrainingError};
#[path = "joint_kernel.rs"]
mod kernel;
/// Observable action tuple plus a diagnostic discarded base-global draw.
/// Do not include `latent_global` as a separate factor in the tuple likelihood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JointSample2054 {
    pub representative: usize,
    pub locomotion: usize,
    pub orientation: usize,
    pub latent_global: usize,
}
/// Ideal f64 likelihoods with explicitly finite/native-order global sampling.
/// Zero pair potential is not pointwise legacy motor-tuple compatibility.
#[derive(Debug, Clone)]
pub struct JointPolicy2054 {
    inner: kernel::Policy,
    candidates: usize,
}
impl JointPolicy2054 {
    pub fn new(
        base: &[f32],
        locomotion_mask: u32,
        orientation_mask: u32,
        forced: &[Option<u8>],
        potential: &[f32],
        temperature: f32,
    ) -> Result<Self, TrainingError> {
        let inner = kernel::Policy::new(
            base,
            locomotion_mask,
            orientation_mask,
            forced,
            potential,
            temperature,
        )
        .map_err(|_| invalid())?;
        Ok(Self {
            inner,
            candidates: base.len(),
        })
    }
    pub fn categories(&self) -> &[f64] {
        self.inner.categories()
    }
    pub fn pairs(&self) -> &[f64] {
        self.inner.pairs()
    }
    pub fn temperature(&self) -> f32 {
        self.inner.temperature()
    }
    pub fn category(&self, candidate: usize) -> Result<usize, TrainingError> {
        if candidate >= self.candidates {
            return Err(invalid());
        }
        Ok(self.inner.category(candidate))
    }
    pub fn forced_slot(&self, candidate: usize) -> Result<Option<u8>, TrainingError> {
        if candidate >= self.candidates {
            return Err(invalid());
        }
        Ok(self.inner.forced_slot(candidate))
    }
    fn validate_indices(&self, category: usize, pair: usize) -> Result<(), TrainingError> {
        if category >= self.categories().len() || pair >= self.pairs().len() {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn logprob(&self, category: usize, pair: usize) -> Result<f64, TrainingError> {
        self.validate_indices(category, pair)?;
        Ok(self.inner.logprob(category, pair))
    }
    pub fn gradient(
        &self,
        category: usize,
        pair: usize,
    ) -> Result<(Vec<f64>, Vec<f64>), TrainingError> {
        self.validate_indices(category, pair)?;
        Ok(self.inner.gradient(category, pair))
    }
    pub fn entropy_gradient(&self) -> (f64, Vec<f64>, Vec<f64>) {
        self.inner.entropy_gradient()
    }
    pub fn sample(&self, uniform: [f32; 3]) -> Result<JointSample2054, TrainingError> {
        let (representative, locomotion, orientation, latent_global) =
            self.inner.sample(uniform).map_err(|_| invalid())?;
        Ok(JointSample2054 {
            representative,
            locomotion,
            orientation,
            latent_global,
        })
    }
}

#[cfg(test)]
#[path = "joint_policy_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "joint_kernel_tests.rs"]
mod kernel_tests;
