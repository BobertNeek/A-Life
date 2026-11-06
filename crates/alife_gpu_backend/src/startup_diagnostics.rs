//! Best-effort structured device-failure context, outside saved state.
//!
//! Only enums, hardware IDs, known device roles and four validated limit names
//! with decimal values reach the sink. Driver text, adapter names, paths,
//! arbitrary labels and native-code substrings are never emitted.
//!
//! `prepare` runs before device acquisition, outside the device-loss callback.
//! The callback and submit path only copy a fixed-size record through a one-slot
//! `try_send` queue, without initialization, formatting or stderr access. Returned
//! request errors are classified using a bounded temporary before submission. One
//! detached worker may remain blocked on stderr until process exit. A full
//! queue, missing/failed worker, crash or shutdown can lose diagnostics. There
//! is no flush/join on the failure path and no hard real-time latency claim.

use std::{
    error::Error,
    fmt,
    io::{self, Write},
    sync::{
        mpsc::{self, Receiver, SyncSender},
        OnceLock,
    },
};

use alife_core::ScaffoldContractError;

const MAX_PENDING_RECORDS: usize = 1;
const MAX_MESSAGE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdapterContext {
    backend: wgpu::Backend,
    vendor: u32,
    device: u32,
}

impl AdapterContext {
    pub(crate) fn from_info(info: &wgpu::AdapterInfo) -> Self {
        Self {
            backend: info.backend,
            vendor: info.vendor,
            device: info.device,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceRole {
    RequiredClosedLoop,
    OfflineSoftwareTest,
    Other,
}

impl DeviceRole {
    fn from_label(label: &str) -> Self {
        match label {
            "alife-required-closed-loop-device" => Self::RequiredClosedLoop,
            "alife-explicit-software-offline-training-test" => Self::OfflineSoftwareTest,
            _ => Self::Other,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::RequiredClosedLoop => "required_closed_loop",
            Self::OfflineSoftwareTest => "offline_software_test",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestDetail {
    Other,
    OutOfMemory,
    DeviceLost,
    TimestampNormalizer,
    Limit {
        name: &'static str,
        requested: u64,
        allowed: u64,
    },
}

// This bounded temporary is used only to classify a returned request error.
// No part of it is retained in a record, including on truncation/format failure.
#[derive(Default)]
struct Message {
    text: String,
    incomplete: bool,
}

impl fmt::Write for Message {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.incomplete
            || self.text.len() + value.len() > MAX_MESSAGE_BYTES
            || !value.is_ascii()
            || value.bytes().any(|c| c.is_ascii_control())
        {
            self.incomplete = true;
            return Err(fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}

fn classify_request(error: &impl Error) -> RequestDetail {
    use fmt::Write;
    let mut message = Message::default();
    if write!(&mut message, "{error}").is_err() || message.incomplete {
        return RequestDetail::Other;
    }
    // Exact wgpu-core 29 messages, not a claim to recover a HAL/driver cause.
    match message.text.as_str() {
        "Not enough memory left." => RequestDetail::OutOfMemory,
        "Parent device is lost" => RequestDetail::DeviceLost,
        "Failed to initialize Timestamp Normalizer" => RequestDetail::TimestampNormalizer,
        text => parse_limit(text).unwrap_or(RequestDetail::Other),
    }
}

fn parse_limit(text: &str) -> Option<RequestDetail> {
    let (name, values) = text.strip_prefix("Limit '")?.split_once("' value ")?;
    let name = match name {
        "max_bind_groups" => "max_bind_groups",
        "max_storage_buffers_per_shader_stage" => "max_storage_buffers_per_shader_stage",
        "max_buffer_size" => "max_buffer_size",
        "max_storage_buffer_binding_size" => "max_storage_buffer_binding_size",
        _ => return None,
    };
    let (requested, allowed) = values.split_once(" is better than allowed ")?;
    let decimal = |value: &str| -> Option<u64> {
        if value.is_empty()
            || !value.bytes().all(|c| c.is_ascii_digit())
            || (value.len() > 1 && value.starts_with('0'))
        {
            return None;
        }
        value.parse().ok()
    };
    let requested = decimal(requested)?;
    let allowed = decimal(allowed)?;
    (requested > allowed).then_some(RequestDetail::Limit {
        name,
        requested,
        allowed,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Request {
        role: DeviceRole,
        detail: RequestDetail,
    },
    Lost {
        reason: wgpu::DeviceLostReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagnosticRecord {
    context: AdapterContext,
    event: Event,
}

impl DiagnosticRecord {
    fn line(self) -> String {
        let context = self.context;
        let prefix = format!(
            "gpu_device_failure backend={:?} vendor={:#06x} device={:#06x}",
            context.backend, context.vendor, context.device
        );
        match self.event {
            Event::Lost { reason } => format!("{prefix} stage=device_lost reason={reason:?}"),
            Event::Request { role, detail } => {
                let prefix = format!("{prefix} stage=request_device role={}", role.name());
                match detail {
                    RequestDetail::Other => format!("{prefix} kind=other"),
                    RequestDetail::OutOfMemory => format!("{prefix} kind=wgpu_out_of_memory"),
                    RequestDetail::DeviceLost => format!("{prefix} kind=wgpu_device_lost"),
                    RequestDetail::TimestampNormalizer => format!("{prefix} kind=wgpu_timestamp_normalizer"),
                    RequestDetail::Limit { name, requested, allowed } => format!(
                        "{prefix} kind=wgpu_limit limit={name} requested={requested} allowed={allowed}"),
                }
            }
        }
    }
}

#[derive(Default)]
struct DiagnosticSink {
    sender: OnceLock<Option<SyncSender<DiagnosticRecord>>>,
}

impl DiagnosticSink {
    fn prepare_with(&self, start: impl FnOnce(Receiver<DiagnosticRecord>) -> io::Result<()>) {
        // Initialization may wait for OS thread creation; callers must perform
        // it before requesting a device, never from a device-loss callback.
        self.sender.get_or_init(|| {
            let (sender, receiver) = mpsc::sync_channel(MAX_PENDING_RECORDS);
            start(receiver).ok().map(|()| sender)
        });
    }

    fn submit(&self, record: DiagnosticRecord) {
        // get returns None during initialization. try_send drops on saturation
        // or disconnection; there is no blocking send or initialization here.
        if let Some(Some(sender)) = self.sender.get() {
            let _ = sender.try_send(record);
        }
    }
}

static SINK: DiagnosticSink = DiagnosticSink {
    sender: OnceLock::new(),
};

pub(crate) fn prepare() {
    SINK.prepare_with(|receiver| {
        std::thread::Builder::new()
            .name("alife-gpu-diagnostics".into())
            .spawn(move || {
                for record in receiver {
                    emit_to(std::io::stderr().lock(), &record.line());
                }
            })
            .map(drop)
    });
}

pub(crate) fn emit(record: DiagnosticRecord) {
    SINK.submit(record);
}

fn emit_to(mut writer: impl Write, line: &str) {
    let _ = writeln!(writer, "{line}");
}

pub(crate) fn request_device_result<T>(
    result: Result<T, impl Error>,
    context: impl FnOnce() -> AdapterContext,
    label: &str,
    mut report: impl FnMut(DiagnosticRecord),
) -> Result<T, ScaffoldContractError> {
    result.map_err(|error| {
        report(DiagnosticRecord {
            context: context(),
            event: Event::Request {
                role: DeviceRole::from_label(label),
                detail: classify_request(&error),
            },
        });
        ScaffoldContractError::NeuralBackendUnavailable
    })
}

pub(crate) fn device_lost(
    lost: &std::sync::atomic::AtomicBool,
    context: AdapterContext,
    reason: wgpu::DeviceLostReason,
    mut report: impl FnMut(DiagnosticRecord),
) {
    lost.store(true, std::sync::atomic::Ordering::Release);
    if reason != wgpu::DeviceLostReason::Destroyed {
        report(DiagnosticRecord {
            context,
            event: Event::Lost { reason },
        });
    }
}

#[cfg(test)]
mod tests;
