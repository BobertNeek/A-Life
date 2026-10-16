//! Opt-in wall-clock spans for the serial training command, never live gameplay.
//! This diagnostic sidecar is deliberately outside canonical training identities.

use std::{
    cell::RefCell,
    io::Write,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

#[derive(Default)]
pub(super) struct CycleStageTiming(Option<Rc<RefCell<Recorder>>>);

struct Recorder {
    clock: Box<dyn Fn() -> Duration>,
    output: Option<std::path::PathBuf>,
    context: Value,
    active: Vec<ActiveSpan>,
    spans: Vec<Value>,
    covered: Duration,
    next_id: usize,
}

struct ActiveSpan {
    id: usize,
    name: &'static str,
    parent: Option<usize>,
    start: Duration,
    children: Duration,
}

pub(super) struct StageSpan {
    recorder: Option<Rc<RefCell<Recorder>>>,
    id: usize,
}

impl CycleStageTiming {
    pub(super) fn from_environment(context: impl FnOnce() -> Value) -> Self {
        if std::env::var_os("ALIFE_FOUNDATION_PROFILE").is_none() {
            return Self::default();
        }
        let start = Instant::now();
        Self::with_clock(Box::new(move || start.elapsed()), context())
    }

    fn with_clock(clock: Box<dyn Fn() -> Duration>, context: Value) -> Self {
        Self(Some(Rc::new(RefCell::new(Recorder {
            clock,
            output: None,
            context,
            active: Vec::new(),
            spans: Vec::new(),
            covered: Duration::ZERO,
            next_id: 0,
        }))))
    }

    pub(super) fn enabled(&self) -> bool {
        self.0.is_some()
    }

    // Call only inside in_new_directory's operation, after its atomic claim.
    pub(super) fn own_output(&self, output: &Path) {
        if let Some(recorder) = &self.0 {
            recorder.borrow_mut().output = Some(output.to_owned());
        }
    }

    pub(super) fn annotate(&self, context: impl FnOnce() -> Value) {
        if let Some(recorder) = &self.0 {
            let mut recorder = recorder.borrow_mut();
            if let (Some(target), Value::Object(extra)) =
                (recorder.context.as_object_mut(), context())
            {
                target.extend(extra);
            }
        }
    }

    pub(super) fn stage(&self, name: &'static str) -> StageSpan {
        let Some(recorder) = &self.0 else {
            return StageSpan {
                recorder: None,
                id: 0,
            };
        };
        let mut state = recorder.borrow_mut();
        let id = state.next_id;
        state.next_id += 1;
        let parent = state.active.last().map(|span| span.id);
        let start = (state.clock)();
        state.active.push(ActiveSpan {
            id,
            name,
            parent,
            start,
            children: Duration::ZERO,
        });
        StageSpan {
            recorder: Some(Rc::clone(recorder)),
            id,
        }
    }

    fn receipt(&self, error: Option<&dyn std::error::Error>) -> Option<Value> {
        let recorder = self.0.as_ref()?.borrow();
        let total = (recorder.clock)();
        Some(json!({
            "schema_version": 1,
            "time_basis": "monotonic_host_wall_clock_around_existing_calls_not_gpu_kernel_time",
            "status": if error.is_some() { "error" } else { "complete" },
            "error": error.map(ToString::to_string),
            "context": recorder.context,
            "native_total_seconds": total.as_secs_f64(),
            "exclusive_covered_seconds": recorder.covered.as_secs_f64(),
            "unassigned_seconds": total.saturating_sub(recorder.covered).as_secs_f64(),
            "spans": recorder.spans,
        }))
    }

    pub(super) fn finish(&self, error: Option<&dyn std::error::Error>) {
        let Some(receipt) = self.receipt(error) else {
            return;
        };
        let output = self
            .0
            .as_ref()
            .and_then(|state| state.borrow().output.clone());
        // Admission failures have no owned destination. Never create or overwrite
        // a source/foreign directory just to persist profiling diagnostics.
        let Some(output) = output else {
            return;
        };
        let result = serde_json::to_vec_pretty(&receipt)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(output.join("stage-timing.json"), bytes));
        if let Err(error) = result {
            // Profiling failure must not replace a training error, or turn a
            // successfully sealed cycle into a failed cycle.
            let _ = writeln!(
                std::io::stderr(),
                "could not write optional cycle stage timing: {error}"
            );
        }
    }
}

impl StageSpan {
    pub(super) fn finish(mut self) {
        self.close("complete");
    }

    fn close(&mut self, status: &'static str) {
        let Some(recorder) = self.recorder.take() else {
            return;
        };
        let mut state = recorder.borrow_mut();
        let Some(span) = state.active.pop() else {
            return;
        };
        debug_assert_eq!(span.id, self.id, "cycle spans must close in stack order");
        let elapsed = (state.clock)().saturating_sub(span.start);
        let exclusive = elapsed.saturating_sub(span.children);
        state.covered += exclusive;
        if let Some(parent) = state.active.last_mut() {
            parent.children += elapsed;
        }
        state.spans.push(json!({
            "id": span.id,
            "name": span.name,
            "parent_id": span.parent,
            "call_count": 1,
            "status": status,
            "elapsed_seconds": elapsed.as_secs_f64(),
            "exclusive_seconds": exclusive.as_secs_f64(),
        }));
    }
}

impl Drop for StageSpan {
    fn drop(&mut self) {
        // A normal ?/early return closes the active span without consuming or
        // transforming the original error. Aborted processes cannot flush.
        self.close("interrupted");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn fixture() -> (CycleStageTiming, Rc<Cell<u64>>) {
        let clock = Rc::new(Cell::new(0));
        let source = Rc::clone(&clock);
        (
            CycleStageTiming::with_clock(
                Box::new(move || Duration::from_secs(source.get())),
                json!({"seed": 7}),
            ),
            clock,
        )
    }

    #[test]
    fn nested_spans_are_exclusive_and_remainder_is_explicit() {
        let (timing, clock) = fixture();
        clock.set(2);
        let parent = timing.stage("collection");
        clock.set(3);
        let child = timing.stage("capture");
        clock.set(5);
        child.finish();
        clock.set(8);
        parent.finish();
        clock.set(10);
        let receipt = timing.receipt(None).unwrap();
        assert_eq!(receipt["native_total_seconds"], 10.0);
        assert_eq!(receipt["exclusive_covered_seconds"], 6.0);
        assert_eq!(receipt["unassigned_seconds"], 4.0);
        assert_eq!(receipt["spans"][0]["parent_id"], 0);
        assert_eq!(receipt["spans"][1]["exclusive_seconds"], 4.0);
    }

    #[test]
    fn measured_zero_and_absent_stage_are_different() {
        let (timing, _) = fixture();
        timing.stage("zero").finish();
        let receipt = timing.receipt(None).unwrap();
        assert_eq!(receipt["spans"][0]["elapsed_seconds"], 0.0);
        assert_eq!(receipt["spans"][0]["call_count"], 1);
        assert_eq!(receipt["spans"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn early_error_records_partial_spans_and_retains_original_error() {
        let (timing, clock) = fixture();
        let operation = || -> Result<(), Box<dyn std::error::Error>> {
            let _parent = timing.stage("update");
            clock.set(2);
            let _child = timing.stage("ppo");
            clock.set(5);
            Err("original training failure".into())
        };
        let result = operation();
        let error = result.unwrap_err();
        let receipt = timing.receipt(Some(error.as_ref())).unwrap();
        assert_eq!(receipt["error"], "original training failure");
        assert_eq!(receipt["status"], "error");
        assert_eq!(receipt["exclusive_covered_seconds"], 5.0);
        for span in receipt["spans"].as_array().unwrap() {
            assert_eq!(span["status"], "interrupted");
        }
    }

    #[test]
    fn disabled_timing_does_not_evaluate_context_or_create_output() {
        let timing = CycleStageTiming::default();
        assert!(!timing.enabled());
        timing.annotate(|| panic!("disabled metadata must not run"));
        timing.stage("ignored").finish();
        timing.own_output(Path::new("/this/path/is/not/created"));
        timing.finish(None);
        assert!(timing.receipt(None).is_none());
    }

    #[test]
    fn receipt_roundtrips_and_records_enabled_work() {
        let (timing, _) = fixture();
        timing.annotate(|| json!({"actual_decisions": 16, "terminal_death_tick": null}));
        timing.stage("ppo").finish();
        let receipt = timing.receipt(None).unwrap();
        let decoded: Value =
            serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert_eq!(decoded, receipt);
        assert_eq!(decoded["context"]["actual_decisions"], 16);
    }

    struct Directory(std::path::PathBuf);

    impl Directory {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "alife-cycle-stage-test-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn existing_output_is_never_claimed_or_overwritten_by_profiler() {
        let directory = Directory::new();
        let output = &directory.0;
        std::fs::write(output.join("stage-timing.json"), b"foreign timing").unwrap();
        let (timing, clock) = fixture();
        let admission = timing.stage("source_admission");
        clock.set(2);
        admission.finish();
        let result: Result<(), Box<dyn std::error::Error>> =
            crate::foundation_training_output::in_new_directory(
                output,
                || {
                    timing.own_output(output);
                    panic!("must not run");
                },
                |_| panic!("must not log into foreign output"),
            );
        timing.finish(result.as_ref().err().map(|error| error.as_ref()));
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(output.join("stage-timing.json")).unwrap(),
            b"foreign timing"
        );
    }

    #[test]
    fn owned_error_writes_partial_receipt_and_keeps_training_error() {
        let directory = Directory::new();
        let output = directory.0.join("owned");
        let (timing, clock) = fixture();
        let result: Result<(), Box<dyn std::error::Error>> =
            crate::foundation_training_output::in_new_directory(
                &output,
                || {
                    timing.own_output(&output);
                    let _span = timing.stage("backend_runtime_setup");
                    clock.set(3);
                    Err("original GPU setup error".into())
                },
                |_| {},
            );
        timing.finish(result.as_ref().err().map(|error| error.as_ref()));
        assert_eq!(result.unwrap_err().to_string(), "original GPU setup error");
        let receipt: Value =
            serde_json::from_slice(&std::fs::read(output.join("stage-timing.json")).unwrap())
                .unwrap();
        assert_eq!(receipt["error"], "original GPU setup error");
        assert_eq!(receipt["spans"][0]["status"], "interrupted");
        assert_eq!(receipt["spans"][0]["elapsed_seconds"], 3.0);
    }

    #[test]
    fn optional_write_failure_does_not_change_success_or_error() {
        let directory = Directory::new();
        let (timing, _) = fixture();
        timing.own_output(&directory.0);
        std::fs::create_dir(directory.0.join("stage-timing.json")).unwrap();
        let success: Result<u8, Box<dyn std::error::Error>> = Ok(9);
        timing.finish(success.as_ref().err().map(|error| error.as_ref()));
        assert_eq!(success.unwrap(), 9);
        let failure: Result<(), Box<dyn std::error::Error>> = Err("original failure".into());
        timing.finish(failure.as_ref().err().map(|error| error.as_ref()));
        assert_eq!(failure.unwrap_err().to_string(), "original failure");
    }

    #[test]
    fn enabled_and_disabled_diagnostics_leave_canonical_bytes_equal() {
        let directory = Directory::new();
        for enabled in [false, true] {
            let output = directory.0.join(if enabled { "on" } else { "off" });
            std::fs::create_dir(&output).unwrap();
            let timing = if enabled {
                fixture().0
            } else {
                CycleStageTiming::default()
            };
            timing.own_output(&output);
            let span = timing.stage("final_receipt_write");
            std::fs::write(output.join("cycle.json"), b"canonical cycle bytes").unwrap();
            std::fs::write(output.join("actor-checkpoint.json"), b"canonical GPU bytes").unwrap();
            span.finish();
            timing.finish(None);
            assert_eq!(output.join("stage-timing.json").is_file(), enabled);
        }
        for name in ["cycle.json", "actor-checkpoint.json"] {
            assert_eq!(
                std::fs::read(directory.0.join("on").join(name)).unwrap(),
                std::fs::read(directory.0.join("off").join(name)).unwrap(),
            );
        }
    }
}
