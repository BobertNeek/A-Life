//! Observe existing wgpu-hal profiling spans; never replace compiler/PSO calls.
use super::{append_trace, append_trace_file, stage};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use tracing::{span::Id, Metadata, Subscriber};
use tracing_subscriber::{
    layer::{Context, Layer},
    prelude::*,
    registry::LookupSpan,
};

thread_local! {
    static SHADER: RefCell<Option<serde_json::Value>> = const { RefCell::new(None) };
    static ACTIVE: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}
const NAMES: [&str; 4] = [
    "compile_dxc",
    "compile_fxc",
    "naga::back::hlsl::write",
    "ID3D12Device::CreateComputePipelineState",
];
pub(super) fn set_shader(shader: serde_json::Value) {
    SHADER.with(|s| *s.borrow_mut() = Some(shader));
}
pub(super) fn reset() {
    SHADER.with(|s| *s.borrow_mut() = None);
    ACTIVE.with(|s| s.borrow_mut().clear());
}
pub(super) fn shader() -> Option<serde_json::Value> {
    SHADER.with(|s| s.borrow().clone())
}
pub(super) fn active() -> Option<&'static str> {
    ACTIVE.with(|s| s.borrow().last().copied())
}
#[derive(Default)]
struct NativeStages {
    output: Option<std::path::PathBuf>,
    sink: Option<Arc<Mutex<Vec<serde_json::Value>>>>,
}
impl NativeStages {
    fn emit(&self, event: &str, metadata: &Metadata<'static>, stack_match: bool) {
        let record = serde_json::json!({"event":event,"scope":metadata.name(),"target":metadata.target(),"thread":format!("{:?}",std::thread::current().id()),"shader":SHADER.with(|s|s.borrow().clone()),"stack_match":stack_match,"unwinding":std::thread::panicking()});
        eprintln!("compiler_probe_native_stage={record}");
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().push(record);
        } else if let Some(path) = &self.output {
            append_trace_file(path, &record.to_string());
        } else {
            append_trace("runtime-native-stages.jsonl", &record.to_string());
        }
    }
}
impl<S> Layer<S> for NativeStages
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.is_span() && NAMES.contains(&metadata.name())
    }
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let meta = span.metadata();
            if meta.name() == "naga::back::hlsl::write" {
                SHADER.with(|s| *s.borrow_mut() = None);
            }
            ACTIVE.with(|s| s.borrow_mut().push(meta.name()));
            self.emit("enter", meta, true);
        }
    }
    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let meta = span.metadata();
            let matched = ACTIVE.with(|s| s.borrow_mut().pop() == Some(meta.name()));
            self.emit("exit", meta, matched);
            // A later precompiled/internal pipeline may have no HLSL emission.
            // Never attribute it to the input consumed by this completed PSO.
            if meta.name() == "ID3D12Device::CreateComputePipelineState" {
                SHADER.with(|s| *s.borrow_mut() = None);
            }
        }
    }
}
pub(super) fn install_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(NativeStages::default()),
        )
        .expect("diagnostic process must own tracing subscriber");
    });
    stage("native_stage_observer_installed");
}

#[test]
fn observer_distinguishes_compile_exit_from_native_pso_entry() {
    fn compile(fail: bool) -> Result<(), u32> {
        profiling::scope!("compile_dxc");
        if fail {
            Err(0x8007_000e)
        } else {
            Ok(())
        }
    }
    fn pipeline(fail_compile: bool) -> Result<(), u32> {
        compile(fail_compile)?;
        let _pso = {
            profiling::scope!("ID3D12Device::CreateComputePipelineState");
            Err::<(), u32>(0x8007_000e)
        };
        _pso
    }
    let events = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(NativeStages {
        output: None,
        sink: Some(events.clone()),
    });
    reset();
    set_shader(serde_json::json!({"file":"exact-target.hlsl","sha256":"test-input-hash"}));
    tracing::subscriber::with_default(subscriber, || {
        assert_eq!(pipeline(true), Err(0x8007_000e));
        {
            let first = events.lock().unwrap();
            assert_eq!(first.len(), 2);
            assert_eq!(first[0]["scope"], "compile_dxc");
            assert_eq!(first[1]["event"], "exit");
        }
        assert_eq!(pipeline(false), Err(0x8007_000e));
        set_shader(serde_json::json!({"file":"stale-input.hlsl","sha256":"stale-hash"}));
        {
            profiling::scope!("naga::back::hlsl::write");
            assert!(SHADER.with(|s| s.borrow().is_none()));
        }
        set_shader(serde_json::json!({"file":"new-target.hlsl","sha256":"new-hash"}));
        assert!(compile(false).is_ok());
    });
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 10);
    assert_eq!(events[4]["event"], "enter");
    assert_eq!(
        events[4]["scope"],
        "ID3D12Device::CreateComputePipelineState"
    );
    for event in events.iter().take(6) {
        assert_eq!(event["shader"]["file"], "exact-target.hlsl");
        assert_eq!(event["stack_match"], true);
    }
    assert!(events[6]["shader"].is_null());
    assert!(events[7]["shader"].is_null());
    assert_eq!(events[8]["shader"]["file"], "new-target.hlsl");
    assert_eq!(events[9]["shader"]["file"], "new-target.hlsl");
    assert_eq!(active(), None);
    reset();
}

#[test]
fn native_pso_exit_does_not_label_a_later_opaque_pipeline_with_stale_input() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(NativeStages {
        output: None,
        sink: Some(events.clone()),
    });
    reset();
    set_shader(serde_json::json!({"file":"consumed.hlsl","sha256":"known-hash"}));
    tracing::subscriber::with_default(subscriber, || {
        {
            profiling::scope!("ID3D12Device::CreateComputePipelineState");
        }
        assert!(shader().is_none());
        {
            profiling::scope!("ID3D12Device::CreateComputePipelineState");
        }
    });
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0]["shader"]["file"], "consumed.hlsl");
    assert_eq!(events[1]["shader"]["file"], "consumed.hlsl");
    assert!(events[2]["shader"].is_null());
    assert!(events[3]["shader"].is_null());
    reset();
}

#[test]
fn concurrent_native_stage_records_keep_json_lines_and_thread_shader_identity() {
    let dir = std::env::temp_dir().join(format!("alife-native-stage-json-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("stages.jsonl");
    std::fs::File::create_new(&path).unwrap();
    let dispatcher = tracing::Dispatch::new(tracing_subscriber::registry().with(NativeStages {
        output: Some(path.clone()),
        sink: None,
    }));
    std::thread::scope(|scope| {
        for name in ["thread-A.hlsl", "thread-B.hlsl"] {
            let dispatcher = dispatcher.clone();
            scope.spawn(move || {
                tracing::dispatcher::with_default(&dispatcher, || {
                    reset();
                    set_shader(serde_json::json!({"file":name,"sha256":name}));
                    for _ in 0..32 {
                        profiling::scope!("compile_dxc");
                        std::thread::yield_now();
                    }
                    assert_eq!(active(), None);
                    reset();
                })
            });
        }
    });
    let lines = std::fs::read_to_string(&path).unwrap();
    let records: Vec<serde_json::Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 128);
    let mut identity =
        std::collections::BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    for record in records {
        assert_eq!(record["stack_match"], true);
        identity
            .entry(record["thread"].as_str().unwrap().into())
            .or_default()
            .insert(record["shader"]["file"].as_str().unwrap().into());
    }
    assert_eq!(identity.len(), 2);
    assert!(identity.values().all(|files| files.len() == 1));
    let files: std::collections::BTreeSet<_> =
        identity.values().flat_map(|v| v.iter().cloned()).collect();
    assert_eq!(
        files,
        std::collections::BTreeSet::from(["thread-A.hlsl".into(), "thread-B.hlsl".into()])
    );
    std::fs::remove_dir_all(dir).unwrap();
}
