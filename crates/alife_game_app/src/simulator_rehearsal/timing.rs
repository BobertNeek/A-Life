//! Stage timing covering setup, captures, updates, and resource release.
use super::Result;
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct RunTiming {
    started: Instant,
    clock: Rc<dyn Fn() -> Duration>,
    state: Rc<RefCell<State>>,
    collection_limit: Duration,
    total_limit: Option<Duration>,
}

#[derive(Default, serde::Serialize)]
struct State {
    #[serde(skip)]
    output: Option<std::path::PathBuf>,
    schema: &'static str,
    collection_allowance_seconds: u64,
    qualification_allowance_seconds: Option<u64>,
    status: &'static str,
    elapsed_seconds: f64,
    error: Option<String>,
    training_decisions: usize,
    holdout_decisions: usize,
    stages: Vec<Stage>,
    #[serde(skip)]
    active: Vec<usize>,
}

#[derive(serde::Serialize)]
struct Stage {
    name: String,
    parent: Option<usize>,
    started_seconds: f64,
    /// Null while running; an unfinished stage is never a measured zero.
    elapsed_seconds: Option<f64>,
    status: &'static str,
    error: Option<String>,
}

impl RunTiming {
    pub fn new(collection_seconds: u64, total_seconds: Option<u64>) -> Self {
        let started = Instant::now();
        Self {
            started,
            clock: Rc::new(move || started.elapsed()),
            state: Rc::new(RefCell::new(State {
                status: "running",
                schema: "alife.simulator-rehearsal-timing.v1",
                collection_allowance_seconds: collection_seconds,
                qualification_allowance_seconds: total_seconds,
                ..State::default()
            })),
            collection_limit: Duration::from_secs(collection_seconds),
            total_limit: total_seconds.map(Duration::from_secs),
        }
    }

    pub fn elapsed(&self) -> Duration {
        (self.clock)()
    }

    pub fn collection_expired(&self) -> bool {
        self.elapsed() >= self.collection_limit
    }

    pub fn collection_deadline(&self) -> Instant {
        self.started
            + self
                .collection_limit
                .min(self.total_limit.unwrap_or(self.collection_limit))
    }

    pub fn deadline(&self, fallback: Instant) -> Instant {
        self.total_limit
            .map_or(fallback, |limit| fallback.min(self.started + limit))
    }

    pub fn check(&self) -> Result<()> {
        if self
            .total_limit
            .is_some_and(|limit| self.elapsed() >= limit)
        {
            return Err("whole-process qualification allowance expired".into());
        }
        Ok(())
    }

    /// Called only after this invocation atomically claimed a new directory.
    pub fn own_output(&self, output: &Path) -> Result<()> {
        self.state.borrow_mut().output = Some(output.to_path_buf());
        self.persist()
    }

    pub fn record_decisions(&self, count: usize, holdout: bool) -> Result<()> {
        let mut state = self.state.borrow_mut();
        if holdout {
            state.holdout_decisions += count;
        } else {
            state.training_decisions += count;
        }
        drop(state);
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        use std::io::Write;
        let mut state = self.state.borrow_mut();
        state.elapsed_seconds = self.elapsed().as_secs_f64();
        if let Some(output) = &state.output {
            // The journal is the interruption-safe history: if a process is
            // stopped during a write, readers use its last complete JSON line.
            // Sync before replacing the convenient summary. Never keep a file
            // handle across stages (including Windows owner release).
            let mut event = serde_json::to_vec(&*state)?;
            event.push(b'\n');
            let mut journal = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(output.join("stage-timing.jsonl"))?;
            journal.write_all(&event)?;
            journal.sync_data()?;
            drop(journal);
            std::fs::write(
                output.join("stage-timing.json"),
                serde_json::to_vec_pretty(&*state)?,
            )?;
        }
        Ok(())
    }

    pub fn stage<T>(&self, name: impl Into<String>, work: impl FnOnce() -> Result<T>) -> Result<T> {
        let name = name.into();
        let start = self.elapsed();
        if let Err(error) = self.check() {
            let mut state = self.state.borrow_mut();
            let parent = state.active.last().copied();
            state.stages.push(Stage {
                name,
                parent,
                started_seconds: start.as_secs_f64(),
                elapsed_seconds: None,
                status: "not_admitted",
                error: Some(error.to_string()),
            });
            drop(state);
            self.persist()?;
            return Err(error);
        }
        let id = {
            let mut state = self.state.borrow_mut();
            let id = state.stages.len();
            let parent = state.active.last().copied();
            state.stages.push(Stage {
                name,
                parent,
                started_seconds: start.as_secs_f64(),
                elapsed_seconds: None,
                status: "running",
                error: None,
            });
            state.active.push(id);
            id
        };
        // A failed diagnostic write must unwind the active stack too; it must
        // never admit unrecorded work or turn its parent's exit into a panic.
        let result = self.persist().and_then(|()| work()).and_then(|value| {
            self.check()?;
            Ok(value)
        });
        {
            let mut state = self.state.borrow_mut();
            assert_eq!(state.active.pop(), Some(id));
            let stage = &mut state.stages[id];
            stage.elapsed_seconds = Some(self.elapsed().saturating_sub(start).as_secs_f64());
            stage.status = if result.is_ok() {
                "completed"
            } else {
                "failed"
            };
            stage.error = result.as_ref().err().map(ToString::to_string);
        }
        self.persist()?;
        result
    }

    /// Includes final publication and its checks. A failed bounded run can
    /// retain diagnostics/checkpoint bytes, but never an admissible marker.
    pub fn finish<T>(&self, result: Result<T>) -> Result<T> {
        let result = result.and_then(|value| {
            self.check()?;
            Ok(value)
        });
        {
            let mut state = self.state.borrow_mut();
            state.status = if result.is_ok() {
                "completed"
            } else {
                "failed"
            };
            state.error = result.as_ref().err().map(ToString::to_string);
        }
        let result = self.persist().and(result).and_then(|value| {
            self.check()?;
            Ok(value)
        });
        if let Err(error) = &result {
            let output = self.state.borrow().output.clone();
            if let Some(output) = output {
                let marker = output.join("rehearsal-continuation.json");
                match std::fs::remove_file(marker) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => {
                        return Err(format!("{error}; completion-marker removal failed: {e}").into())
                    }
                }
            }
            let mut state = self.state.borrow_mut();
            state.status = "failed";
            state.error = Some(error.to_string());
            drop(state);
            self.persist()?;
        }
        result
    }
}

pub(super) fn optional_stage<T>(
    timing: Option<&RunTiming>,
    name: &str,
    work: impl FnOnce() -> Result<T>,
) -> Result<T> {
    match timing {
        Some(timing) => timing.stage(name, work),
        None => work(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn clock(total: Option<u64>) -> (RunTiming, Rc<Cell<Duration>>) {
        let mut timing = RunTiming::new(15, total);
        let now = Rc::new(Cell::new(Duration::ZERO));
        let read = now.clone();
        timing.clock = Rc::new(move || read.get());
        (timing, now)
    }
    fn output(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "alife-rehearsal-timing-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn entry_clock_charges_setup_and_exact_deadline_boundary() {
        let (timing, now) = clock(Some(60));
        timing
            .stage("source-and-cold-setup", || {
                now.set(Duration::from_secs(15));
                Ok(())
            })
            .unwrap();
        assert!(timing.collection_expired());
        now.set(Duration::from_secs(59));
        assert!(timing.check().is_ok());
        now.set(Duration::from_secs(60));
        let mut entered = false;
        assert!(timing
            .stage("update", || {
                entered = true;
                Ok(())
            })
            .is_err());
        assert!(!entered);
        assert_eq!(
            timing.collection_deadline(),
            timing.started + Duration::from_secs(15)
        );
        assert_eq!(
            timing.deadline(timing.started + Duration::from_secs(90)),
            timing.started + Duration::from_secs(60)
        );
        let (unbounded, _) = clock(None);
        assert_eq!(unbounded.deadline(timing.started), timing.started);
    }
    #[test]
    fn persistent_nested_failed_stages_preserve_partial_progress() {
        let (timing, now) = clock(Some(60));
        let path = output("failure");
        timing.own_output(&path).unwrap();
        let result: Result<()> = timing.stage("attempt-0", || {
            timing.stage("decision-0", || {
                now.set(Duration::from_secs(4));
                Ok(())
            })?;
            timing.stage("capture", || {
                let active: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(path.join("stage-timing.json"))?)?;
                assert!(active["stages"][2]["elapsed_seconds"].is_null());
                now.set(Duration::from_secs(14));
                Err::<(), _>("capture failed".into())
            })
        });
        assert_eq!(
            timing.finish(result).unwrap_err().to_string(),
            "capture failed"
        );
        let log: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("stage-timing.json")).unwrap())
                .unwrap();
        assert_eq!(log["status"], "failed");
        assert_eq!(log["elapsed_seconds"], 14.0);
        assert_eq!(log["stages"][1]["status"], "completed");
        assert_eq!(log["stages"][2]["parent"], 0);
        assert_eq!(log["stages"][2]["elapsed_seconds"], 10.0);
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn publication_crossing_total_allowance_removes_marker_and_never_reports_complete() {
        let (timing, now) = clock(Some(60));
        let path = output("publication");
        timing.own_output(&path).unwrap();
        let result = timing.stage("publication", || {
            std::fs::write(path.join("rehearsal-continuation.json"), b"complete")?;
            now.set(Duration::from_secs(60));
            Ok(())
        });
        assert!(timing.finish(result).is_err());
        assert!(!path.join("rehearsal-continuation.json").exists());
        let log: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("stage-timing.json")).unwrap())
                .unwrap();
        assert_eq!(log["status"], "failed");
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn atomic_decisions_and_resource_drop_are_charged_even_when_the_stage_times_out() {
        let (timing, now) = clock(Some(60));
        let path = output("release");
        timing.own_output(&path).unwrap();
        struct Owner(Rc<Cell<Duration>>);
        impl Drop for Owner {
            fn drop(&mut self) {
                self.0.set(Duration::from_secs(60));
            }
        }
        let result: Result<()> = timing
            .stage("execution_and_resource_release", || {
                let _owner = Owner(now.clone());
                timing.stage("decision", || {
                    now.set(Duration::from_secs(59));
                    timing.record_decisions(1, false)?;
                    Ok(())
                })
            })
            .and_then(|()| {
                timing.stage("completion_publication", || {
                    panic!("publication must not precede resource release")
                })
            });
        assert!(timing.finish(result).is_err());
        let log: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("stage-timing.json")).unwrap())
                .unwrap();
        assert_eq!(log["training_decisions"], 1);
        assert_eq!(log["stages"][0]["elapsed_seconds"], 60.0);
        assert_eq!(log["stages"][0]["status"], "failed");
        assert_eq!(log["stages"][1]["status"], "completed");
        assert!(!path.join("rehearsal-continuation.json").exists());
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn rejected_source_never_claims_or_removes_foreign_output() {
        let (timing, _) = clock(Some(60));
        let path = output("foreign");
        std::fs::write(path.join("rehearsal-continuation.json"), b"foreign").unwrap();
        let result: Result<()> = timing.stage("admission", || Err("invalid source".into()));
        assert!(timing.finish(result).is_err());
        assert!(!path.join("stage-timing.json").exists());
        assert_eq!(
            std::fs::read(path.join("rehearsal-continuation.json")).unwrap(),
            b"foreign"
        );
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn journal_retains_progress_if_latest_summary_and_trailing_event_are_interrupted() {
        use std::io::Write;
        let (timing, now) = clock(Some(60));
        let path = output("interrupted");
        timing.own_output(&path).unwrap();
        timing
            .stage("decision", || {
                now.set(Duration::from_secs(3));
                timing.record_decisions(1, true)
            })
            .unwrap();
        std::fs::write(path.join("stage-timing.json"), b"{interrupted").unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(path.join("stage-timing.jsonl"))
            .unwrap()
            .write_all(b"{interrupted")
            .unwrap();
        let journal = std::fs::read_to_string(path.join("stage-timing.jsonl")).unwrap();
        let recovered = journal
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .next_back()
            .unwrap();
        assert_eq!(recovered["holdout_decisions"], 1);
        assert_eq!(recovered["stages"][0]["status"], "completed");
        assert_eq!(recovered["elapsed_seconds"], 3.0);
        assert_eq!(recovered["status"], "running"); // no completion was fabricated
        std::fs::remove_dir_all(path).unwrap();
    }
}
