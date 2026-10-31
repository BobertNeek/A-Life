//! Ordered, bounded replay preparation beside the live collector.
//!
//! Enqueueing a capture does not authenticate a replay reference. Only `finish`
//! returns references, after every capture has passed the ordinary writer and
//! the complete cohort has been sealed. Dropping or discarding never seals.
use super::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

// One capture may wait while one is being converted/encoded. Backpressure
// bounds snapshot retention independently of the requested cohort length.
const QUEUED_CAPTURES: usize = 1;
// Continuous training admits N2048 captures. The cap concerns the two retained
// mutable arrays, not compressed replay bytes. Together with one-slot FIFO it
// limits queued + in-flight arrays to 16 MiB. A 64 MiB concurrent-work allowance
// also covers the writer's existing four-times-array scratch charge and the
// collector's next capture; normal process guards still charge optional caches.
const MAX_CAPTURE_ARRAY_BYTES: u64 = 8 * 1024 * 1024;
const CONCURRENT_REPLAY_HEADROOM: u64 = 64 * 1024 * 1024;

trait ReplayWindow: Send + 'static {
    type Capture: Send + 'static;
    type Finished: Send + 'static;
    fn append(&mut self, capture: Self::Capture) -> std::result::Result<(), String>;
    fn seal(self) -> std::result::Result<Self::Finished, String>;
}

type WorkerResult<T> = std::result::Result<T, String>;

enum Command<W: ReplayWindow> {
    Start(W),
    Append(W::Capture),
    Finish(mpsc::SyncSender<WorkerResult<W::Finished>>),
    Discard(mpsc::SyncSender<WorkerResult<()>>),
    Shutdown,
}

struct OrderedReplayWorker<W: ReplayWindow> {
    commands: mpsc::SyncSender<Command<W>>,
    failure: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
    active: bool,
}

impl<W: ReplayWindow> OrderedReplayWorker<W> {
    fn new() -> Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(QUEUED_CAPTURES);
        let failure = Arc::new(Mutex::new(None));
        let observed_failure = failure.clone();
        let thread = thread::Builder::new()
            .name("alife-replay-writer".into())
            .spawn(move || {
                let mut window: Option<W> = None;
                let mut first_error: Option<String> = None;
                while let Ok(command) = receiver.recv() {
                    match command {
                        Command::Start(next) => {
                            window = Some(next);
                            first_error = None;
                        }
                        Command::Append(capture) => {
                            if first_error.is_none() {
                                let outcome = catch_unwind(AssertUnwindSafe(|| {
                                    window
                                        .as_mut()
                                        .ok_or_else(|| {
                                            "replay worker has no active cohort".to_owned()
                                        })?
                                        .append(capture)
                                }))
                                .unwrap_or_else(|_| Err("replay writer panicked".into()));
                                if let Err(error) = outcome {
                                    if let Ok(mut observed) = observed_failure.lock() {
                                        *observed = Some(error.clone());
                                    }
                                    first_error = Some(error);
                                }
                            }
                        }
                        Command::Finish(reply) => {
                            let result = match (first_error.take(), window.take()) {
                                (Some(error), _) => Err(error),
                                (None, Some(window)) => {
                                    catch_unwind(AssertUnwindSafe(|| window.seal()))
                                        .unwrap_or_else(|_| Err("replay seal panicked".into()))
                                }
                                (None, None) => Err("replay worker has no active cohort".into()),
                            };
                            let _ = reply.send(result);
                        }
                        Command::Discard(reply) => {
                            // Drop buffered file state before acknowledging. The
                            // caller may safely remove its directory afterwards.
                            drop(window.take());
                            let _ = reply.send(first_error.take().map_or(Ok(()), Err));
                        }
                        Command::Shutdown => break,
                    }
                }
                // Incomplete work remains pending evidence, never a usable
                // replay cohort. Drop before the owner joins/removes anything.
                drop(window);
            })?;
        Ok(Self {
            commands,
            failure,
            thread: Some(thread),
            active: false,
        })
    }

    fn start(&mut self, window: W) -> Result<()> {
        if self.active {
            return Err("replay worker already has an active cohort".into());
        }
        *self
            .failure
            .lock()
            .map_err(|_| "replay worker failure lock poisoned")? = None;
        self.commands
            .send(Command::Start(window))
            .map_err(|_| "replay worker stopped before cohort start")?;
        self.active = true;
        Ok(())
    }

    fn check(&self) -> Result<()> {
        if let Some(error) = self
            .failure
            .lock()
            .map_err(|_| "replay worker failure lock poisoned")?
            .as_ref()
        {
            return Err(format!("background replay preparation failed: {error}").into());
        }
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err("replay worker stopped unexpectedly".into());
        }
        Ok(())
    }

    fn append(&mut self, capture: W::Capture) -> Result<()> {
        if !self.active {
            return Err("replay worker has no active cohort".into());
        }
        self.check()?;
        self.commands
            .send(Command::Append(capture))
            .map_err(|_| "replay worker stopped before capture enqueue")?;
        self.check()
    }

    fn finish(&mut self) -> Result<W::Finished> {
        if !self.active {
            return Err("replay worker has no active cohort".into());
        }
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Finish(reply))
            .map_err(|_| "replay worker stopped before cohort finish")?;
        self.active = false;
        response
            .recv()
            .map_err(|_| "replay worker stopped during cohort finish")?
            .map_err(Into::into)
    }

    fn discard(&mut self) -> Result<()> {
        if !self.active {
            return Err("replay worker has no active cohort".into());
        }
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Discard(reply))
            .map_err(|_| "replay worker stopped before cohort discard")?;
        self.active = false;
        response
            .recv()
            .map_err(|_| "replay worker stopped during cohort discard")?
            .map_err(Into::into)
    }
}

impl<W: ReplayWindow> Drop for OrderedReplayWorker<W> {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Capture {
    step: FoundationTrainingStep,
    new_segment: bool,
}

struct CapturedReplayWindow {
    writer: FoundationReplayWriter,
    cohort: CohortReplayWriter,
    references: Vec<FoundationReplayRecordRef>,
}

pub(crate) struct FinishedReplayWindow {
    pub references: Vec<FoundationReplayRecordRef>,
    pub commit: FoundationCohortReplayCommit,
}

impl ReplayWindow for CapturedReplayWindow {
    type Capture = Capture;
    type Finished = FinishedReplayWindow;

    fn append(&mut self, capture: Capture) -> WorkerResult<()> {
        if capture.new_segment {
            self.writer.start_segment().map_err(|e| e.to_string())?;
        }
        let reference = self
            .writer
            .append_cohort(&capture.step, &mut self.cohort)
            .map_err(|e| e.to_string())?;
        self.references.push(reference);
        Ok(())
    }

    fn seal(self) -> WorkerResult<FinishedReplayWindow> {
        let commit = self.cohort.seal().map_err(|e| e.to_string())?;
        Ok(FinishedReplayWindow {
            references: self.references,
            commit,
        })
    }
}

/// One persistent worker for a continuous session. The existing writer retains
/// its process-memory admission checks before conversion and cache allocation.
pub(crate) struct FoundationReplayWorker {
    worker: OrderedReplayWorker<CapturedReplayWindow>,
    budget: Option<FoundationReplayBudget>,
}

impl FoundationReplayWorker {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            worker: OrderedReplayWorker::new()?,
            budget: None,
        })
    }

    pub(crate) fn start(
        &mut self,
        writer: FoundationReplayWriter,
        cohort: CohortReplayWriter,
    ) -> Result<()> {
        writer.budget.check_additional(CONCURRENT_REPLAY_HEADROOM)?;
        let budget = writer.budget;
        self.worker.start(CapturedReplayWindow {
            writer,
            cohort,
            references: Vec::new(),
        })?;
        self.budget = Some(budget);
        Ok(())
    }

    pub(crate) fn check(&self) -> Result<()> {
        self.worker.check()?;
        self.budget
            .ok_or("replay worker has no active cohort")?
            .check_additional(CONCURRENT_REPLAY_HEADROOM)
    }

    pub(crate) fn append(&mut self, step: FoundationTrainingStep, new_segment: bool) -> Result<()> {
        let array_bytes = (step.before.mutable_words.len() as u64)
            .checked_add(step.after_inference.mutable_words.len() as u64)
            .and_then(|words| words.checked_mul(4))
            .ok_or_else(invalid)?;
        if array_bytes > MAX_CAPTURE_ARRAY_BYTES {
            return Err("continuous replay capture exceeds bounded worker array capacity".into());
        }
        self.check()?;
        self.worker.append(Capture { step, new_segment })
    }

    pub(crate) fn finish(&mut self) -> Result<FinishedReplayWindow> {
        let result = self.worker.finish();
        self.budget = None;
        result
    }

    pub(crate) fn discard(&mut self) -> Result<()> {
        let result = self.worker.discard();
        self.budget = None;
        result
    }
}

#[cfg(test)]
mod tests;
