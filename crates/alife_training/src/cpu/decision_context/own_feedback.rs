//! Detached CPU own-command evidence, not present perception or goal value.
//!
//! The caller supplies historical nominal and measured unit directions in a
//! common frame, plus measured displacement. This module does not authenticate
//! an actuator, collect observations, advance a World, or install a policy.
//! Event ordering validates caller-supplied references, not their provenance.
//! Five finite signed parameters remain independently mutable and trainable.
//!
//! The f64 numerical contract and standalone bit payloads retain their original
//! identities. They are not production organism checkpoint schemas. No existing
//! profile, recurrent weight, optimizer, or runtime feature changes here.
#[path = "own_event_kernel.rs"]
mod own_event_kernel;
#[path = "own_feedback_kernel.rs"]
mod own_feedback_kernel;

pub use own_event_kernel::{Adam, Event, Gradients, Model, Output, State};
pub use own_feedback_kernel::{backward, forward, Error, Tape};
pub const OWN_EVENT_IDENTITY: &str = own_event_kernel::IDENTITY;
pub const DIRECT_FEEDBACK_IDENTITY: &str = own_feedback_kernel::IDENTITY;

/// Exact selected expensive operations on a *successful detached forward*.
/// This is not a FLOP total, elapsed-time measure, ATP tariff or host admission.
/// Validation, copying, scalar arithmetic, backward and Adam work are not priced.
/// Counts include geometry-validation hypots and sigmoid evaluation even when
/// first-capture stride assignment does not use its result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForwardOperations {
    pub hypot: u8,
    pub tanh: u8,
    pub sigmoid: u8,
    pub direct_shortfall: bool,
}

impl Tape {
    pub fn forward_operations(&self) -> ForwardOperations {
        let e = self.historical_event();
        if !self.own_output().processed {
            return ForwardOperations {
                hypot: 0,
                tanh: 0,
                sigmoid: 0,
                direct_shortfall: false,
            };
        }
        ForwardOperations {
            // Four angular unit checks, optional translation unit check,
            // measured displacement magnitude and stride normalization.
            hypot: 6 + u8::from(e.moving),
            tanh: 7,
            sigmoid: u8::from(e.moving && e.vectors[3] != [0.; 2]),
            direct_shortfall: self.calibrated_moving(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> Event {
        Event {
            sequence: 0,
            start_tick: 0,
            end_tick: 1,
            admitted: true,
            issued: true,
            moving: true,
            vectors: [[1., 0.], [1., 0.], [1., 0.], [0.1, 0.], [1., 0.], [1., 0.]],
        }
    }
    #[test]
    fn forward_operation_branches() {
        let p = Model::default().parameters;
        let t = forward(State::default(), event(), p).unwrap();
        assert_eq!(
            t.forward_operations(),
            ForwardOperations {
                hypot: 7,
                tanh: 7,
                sigmoid: 1,
                direct_shortfall: false
            }
        );
        let s = State {
            stride: 0.2,
            occupied: true,
            ..State::default()
        };
        assert!(
            forward(s, event(), p)
                .unwrap()
                .forward_operations()
                .direct_shortfall
        );
        let mut e = event();
        e.vectors[3] = [0.; 2];
        assert_eq!(forward(s, e, p).unwrap().forward_operations().sigmoid, 0);
        e.moving = false;
        e.vectors[0] = [0.; 2];
        let w = forward(s, e, p).unwrap().forward_operations();
        assert_eq!(w.hypot, 6);
        assert!(!w.direct_shortfall);
        e.issued = false;
        e.vectors = [[f64::NAN; 2]; 6];
        assert_eq!(
            forward(s, e, p).unwrap().forward_operations(),
            ForwardOperations {
                hypot: 0,
                tanh: 0,
                sigmoid: 0,
                direct_shortfall: false
            }
        );
    }
    #[test]
    fn signed_zero_tape_binding_and_unselected_optimizer_identity() {
        let mut m = Model::default();
        m.parameters[2] = -0.;
        let t = forward(State::default(), event(), m.parameters).unwrap();
        let mut altered = m.parameters;
        altered[2] = 0.;
        assert!(backward(&t, altered, [0.; 7], 0., 0.).is_err());
        let before = m.cold_bits().unwrap();
        m.update([f64::MAX; 5], [false; 5], Adam::default())
            .unwrap();
        assert_eq!(before, m.cold_bits().unwrap());
    }
    #[test]
    fn failed_optimizer_is_transactional() {
        let mut m = Model::default();
        m.ages[4] = u64::MAX;
        let before = m.cold_bits().unwrap();
        assert!(m.update([0.1; 5], [true; 5], Adam::default()).is_err());
        assert_eq!(before, m.cold_bits().unwrap());
        assert!(forward(
            State {
                occupied: true,
                stride: 0.,
                ..State::default()
            },
            event(),
            m.parameters
        )
        .is_err());
    }
}
