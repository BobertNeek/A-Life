//! Ephemeral process-wide barrier for native restore inspection. This is never
//! serialized or enabled during ordinary simulation. Admission itself remains
//! unchanged; forbidden work fails before its first state transition.
//!
//! This flag is **not a quiescence or synchronization barrier**. The caller must
//! stop ordinary work before entry and join all spawned workers before taking
//! an attempts receipt or dropping the scope. An operation already admitted
//! while inactive can finish after entry; a paused attempt can increment after
//! a premature receipt/drop/reset. Nesting and concurrent owners are rejected.
//! The standalone restore utility owns an otherwise quiescent process. Embedding
//! this scope alongside a running simulation is unsupported: concurrent work
//! intentionally errors, and existing infallible wrappers may panic on error.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static OWNED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static ATTEMPTS: [AtomicU64; 7] = [const { AtomicU64::new(0) }; 7];

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum Operation {
    Decision,
    NeuralForward,
    WorldAdvance,
    WorldAction,
    Credit,
    ActorAdam,
    CriticAdam,
}
impl Operation {
    pub const NAMES: [&'static str; 7] = [
        "decision",
        "neural_forward",
        "world_advance",
        "world_action",
        "credit",
        "actor_adam",
        "critic_adam",
    ];
}

/// Called before work, including from any worker thread. Ordinary execution
/// performs an inactive atomic read at this hook; one ordinary tick can encounter
/// several hooks. This is not a measured performance-overhead claim.
pub fn permitted(operation: Operation) -> bool {
    if !ACTIVE.load(Ordering::SeqCst) {
        return true;
    }
    ATTEMPTS[operation as usize].fetch_add(1, Ordering::SeqCst);
    false
}

#[must_use]
pub struct Scope {
    _private: (),
}
impl Scope {
    pub fn enter() -> Result<Self, &'static str> {
        OWNED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "a process-wide load-only scope is already active")?;
        for count in &ATTEMPTS {
            count.store(0, Ordering::SeqCst);
        }
        ACTIVE.store(true, Ordering::SeqCst);
        Ok(Self { _private: () })
    }
    pub fn attempts(&self) -> [u64; 7] {
        std::array::from_fn(|i| ATTEMPTS[i].load(Ordering::SeqCst))
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::SeqCst);
        OWNED.store(false, Ordering::SeqCst);
    }
}
