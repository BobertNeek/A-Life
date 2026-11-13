//! Versioned, CPU-only research terminal-consumption memory readout.
//!
//! This extra head never reinterprets the frozen 36 decoder lanes. Its input is
//! `terminal_kernel::Context::desirability`, derived from experienced terminal
//! consumption, not an immediate Approach consequence or reinforcement reward.
//! The scalar weight is editable for weight-space trials, fixed during a life.
//! The derivative is provided for future training integration; no new learner,
//! optimizer, eligibility update, ATP tariff, or host admission is implemented here.

pub const SCHEMA: u16 = 1;
pub const MAX_ABS_GAIN: f32 = 64.0;
const MAGIC: [u8; 4] = *b"TC21";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    InvalidInput,
    InvalidWork,
    NonFiniteScore,
}

/// Public recipe for an optional native-host field. Absence is the off policy.
/// The host may serialize these two fields in its versioned research envelope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub schema_version: u16,
    pub gain: f32,
}
impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA {
            return Err(Error::InvalidConfig);
        }
        Head::new(true, self.gain).map(|_| ())
    }
    pub fn score(&self, desirability: f32) -> Result<f32, Error> {
        self.validate()?;
        Ok(Head::new(true, self.gain)?
            .evaluate(true, Some(desirability), Work::default())?
            .delta)
    }
    pub fn derivative(&self, desirability: f32) -> Result<f32, Error> {
        self.validate()?;
        Ok(Head::new(true, self.gain)?
            .evaluate(true, Some(desirability), Work::default())?
            .derivative_gain)
    }
    pub fn to_bytes(&self) -> Result<[u8; 11], Error> {
        self.validate()?;
        Ok(Head::new(true, self.gain)?.to_bytes())
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let head = Head::from_bytes(bytes)?;
        if !head.enabled() {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            schema_version: SCHEMA,
            gain: head.gain(),
        })
    }
}

/// Immutable-by-default recipe; changing the weight is an explicit candidate edit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Head {
    enabled: bool,
    gain: f32,
}

/// Logical reader/head work only. This is not a complete accounting of native
/// codec hashing, validation, indexing, allocation, or biological ATP expenditure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Work {
    pub record_scans: u32,
    pub appearance_comparisons: u32,
    pub head_scalar_primitives: u32,
}
impl Work {
    pub fn reader(record_scans: u32, appearance_comparisons: u32) -> Result<Self, Error> {
        if record_scans > 64 || appearance_comparisons > record_scans * 9 {
            return Err(Error::InvalidWork);
        }
        Ok(Self {
            record_scans,
            appearance_comparisons,
            head_scalar_primitives: 0,
        })
    }
    pub fn logical_units(self) -> u64 {
        u64::from(self.record_scans)
            + u64::from(self.appearance_comparisons)
            + u64::from(self.head_scalar_primitives)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Receipt {
    pub schema: u16,
    pub applied: bool,
    pub gain: f32,
    pub terminal_desirability: Option<f32>,
    pub delta: f32,
    pub derivative_gain: f32,
    pub work: Work,
}

impl Head {
    pub fn new(enabled: bool, gain: f32) -> Result<Self, Error> {
        if !gain.is_finite() || gain.abs() > MAX_ABS_GAIN {
            return Err(Error::InvalidConfig);
        }
        Ok(Self { enabled, gain })
    }
    pub fn set_gain(&mut self, gain: f32) -> Result<(), Error> {
        let next = Self::new(self.enabled, gain)?;
        *self = next;
        Ok(())
    }
    pub const fn gain(self) -> f32 {
        self.gain
    }
    pub const fn enabled(self) -> bool {
        self.enabled
    }

    /// Stable explicit cold format. Preserve the parent's ordinary checkpoint;
    /// store this additional recipe only in the research candidate envelope.
    pub fn to_bytes(self) -> [u8; 11] {
        let mut bytes = [0; 11];
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4..6].copy_from_slice(&SCHEMA.to_le_bytes());
        bytes[6] = u8::from(self.enabled);
        bytes[7..11].copy_from_slice(&self.gain.to_bits().to_le_bytes());
        bytes
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 11
            || bytes[..4] != MAGIC
            || bytes[4..6] != SCHEMA.to_le_bytes()
            || bytes[6] > 1
        {
            return Err(Error::InvalidConfig);
        }
        let gain = f32::from_bits(u32::from_le_bytes(bytes[7..11].try_into().unwrap()));
        Self::new(bytes[6] != 0, gain)
    }

    /// The native adapter must set `targeted_approach` from the authenticated
    /// action family AND nonempty observed target. Other families receive no edit.
    /// Reader work is retained even on no-match; failed reads must be charged by
    /// the caller. Off must be checked before launching the reader.
    pub fn evaluate(
        self,
        targeted_approach: bool,
        terminal_desirability: Option<f32>,
        reader_work: Work,
    ) -> Result<Receipt, Error> {
        if !self.enabled {
            // Strict off route: no validation, read, arithmetic, or scalar work.
            return Ok(Receipt {
                schema: SCHEMA,
                applied: false,
                gain: self.gain,
                terminal_desirability: None,
                delta: 0.0,
                derivative_gain: 0.0,
                work: Work::default(),
            });
        }
        if reader_work.head_scalar_primitives != 0 {
            return Err(Error::InvalidWork);
        }
        let mut work = Work::reader(reader_work.record_scans, reader_work.appearance_comparisons)?;
        let mut receipt = Receipt {
            schema: SCHEMA,
            applied: false,
            gain: self.gain,
            terminal_desirability: None,
            delta: 0.0,
            derivative_gain: 0.0,
            work,
        };
        if !targeted_approach {
            return Ok(receipt);
        }
        let Some(desirability) = terminal_desirability else {
            return Ok(receipt);
        };
        if !desirability.is_finite() || !(-1.0..=1.0).contains(&desirability) {
            return Err(Error::InvalidInput);
        }
        let y = (2.0 * self.gain * desirability).tanh();
        let delta = 0.5 * y;
        let derivative_gain = desirability * (1.0 - y * y);
        // Forward: 3 multiplications + tanh. Derivative: square, subtract, multiply.
        work.head_scalar_primitives = 7;
        receipt.applied = true;
        receipt.terminal_desirability = Some(desirability);
        receipt.delta = delta;
        receipt.derivative_gain = derivative_gain;
        receipt.work = work;
        Ok(receipt)
    }

    /// Identity routes return the original bits, including negative zero.
    pub fn apply(
        self,
        base_score: f32,
        targeted_approach: bool,
        terminal_desirability: Option<f32>,
        reader_work: Work,
    ) -> Result<(f32, Receipt), Error> {
        let receipt = self.evaluate(targeted_approach, terminal_desirability, reader_work)?;
        if !receipt.applied || receipt.delta == 0.0 {
            return Ok((base_score, receipt));
        }
        let result = base_score + receipt.delta;
        if !base_score.is_finite() || !result.is_finite() {
            return Err(Error::NonFiniteScore);
        }
        Ok((result, receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_config_validates_and_roundtrips() {
        let c = Config {
            schema_version: SCHEMA,
            gain: 16.0,
        };
        assert_eq!(Config::from_bytes(&c.to_bytes().unwrap()).unwrap(), c);
        assert!(c.score(0.1).unwrap() > 0.0);
        assert!(c.derivative(0.1).unwrap() > 0.0);
        assert!(
            Config {
                schema_version: 2,
                gain: 16.0
            }
            .score(0.1)
            .is_err()
        );
        assert!(c.score(f32::NAN).is_err());
        assert!(Config::from_bytes(&Head::new(false, 16.0).unwrap().to_bytes()).is_err());
    }
    #[test]
    fn cold_roundtrip_and_strict_validation() {
        for gain in [-64.0, -0.0, 0.0, 16.0, 64.0] {
            for enabled in [false, true] {
                let h = Head::new(enabled, gain).unwrap();
                assert_eq!(
                    Head::from_bytes(&h.to_bytes()).unwrap().gain().to_bits(),
                    gain.to_bits()
                );
                assert_eq!(Head::from_bytes(&h.to_bytes()).unwrap().enabled(), enabled);
            }
        }
        let bytes = Head::new(true, 16.0).unwrap().to_bytes();
        for index in [0, 4, 6] {
            let mut bad = bytes;
            bad[index] = 255;
            assert!(Head::from_bytes(&bad).is_err());
        }
        assert!(Head::from_bytes(&bytes[..10]).is_err());
        assert!(Head::new(true, f32::NAN).is_err());
        assert!(Head::new(true, 65.0).is_err());
    }
    #[test]
    fn mutable_weight_rejects_transactionally() {
        let mut h = Head::new(true, 16.0).unwrap();
        let before = h;
        assert!(h.set_gain(f32::INFINITY).is_err());
        assert_eq!(h, before);
        h.set_gain(4.0).unwrap();
        assert_eq!(h.gain(), 4.0);
    }
    #[test]
    fn derivative_matches_finite_difference_and_zero_is_trainable() {
        for gain in [-4.0, 0.0, 4.0, 16.0] {
            for x in [-0.1, 0.001, 0.03] {
                let h = Head::new(true, gain).unwrap();
                let r = h.evaluate(true, Some(x), Work::default()).unwrap();
                let eps = 0.002;
                let hi = Head::new(true, gain + eps)
                    .unwrap()
                    .evaluate(true, Some(x), Work::default())
                    .unwrap()
                    .delta;
                let lo = Head::new(true, gain - eps)
                    .unwrap()
                    .evaluate(true, Some(x), Work::default())
                    .unwrap()
                    .delta;
                assert!(((hi - lo) / (2.0 * eps) - r.derivative_gain).abs() < 0.00002);
                if gain == 0.0 {
                    assert_eq!(r.derivative_gain, x);
                }
            }
        }
    }
    #[test]
    fn off_nonapproach_unknown_and_zero_are_exact() {
        let bad_work = Work {
            record_scans: u32::MAX,
            appearance_comparisons: u32::MAX,
            head_scalar_primitives: u32::MAX,
        };
        let off = Head::new(false, 16.0).unwrap();
        let (score, r) = off.apply(-0.0, true, Some(f32::NAN), bad_work).unwrap();
        assert_eq!(score.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(r.work, Work::default());
        for (gain, approach, input) in [
            (16.0, false, Some(1.0)),
            (16.0, true, None),
            (0.0, true, Some(0.3)),
        ] {
            let (score, r) = Head::new(true, gain)
                .unwrap()
                .apply(-0.0, approach, input, Work::default())
                .unwrap();
            assert_eq!(score.to_bits(), (-0.0_f32).to_bits());
            assert_eq!(r.delta, 0.0);
        }
    }
    #[test]
    fn bounded_effect_and_accounted_reader_work() {
        let work = Work::reader(64, 576).unwrap();
        for gain in [-64.0, 64.0] {
            for x in [-1.0, 1.0] {
                let r = Head::new(true, gain)
                    .unwrap()
                    .evaluate(true, Some(x), work)
                    .unwrap();
                assert!(r.delta.abs() <= 0.5);
                assert_eq!(r.work.logical_units(), 647);
            }
        }
        let r = Head::new(true, 16.0)
            .unwrap()
            .evaluate(true, None, work)
            .unwrap();
        assert_eq!(r.work.logical_units(), 640);
        assert!(Work::reader(65, 0).is_err());
        assert!(Work::reader(1, 10).is_err());
    }
}
