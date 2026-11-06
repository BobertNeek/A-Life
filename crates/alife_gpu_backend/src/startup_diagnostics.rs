//! Best-effort failure context, outside saved state and GPU authority.
//!
//! At most four first-line causes are retained. Fields are bounded UTF-8 and
//! escaped, and any field containing a path/address marker is redacted in full.
//! This is a conservative heuristic, not a general secret scrubber. Causes
//! already discarded by wgpu/HAL cannot be recovered. A stalled stderr can
//! still block reporting; device loss is marked before attempting that IO.

use std::{error::Error, fmt, io::Write};

use alife_core::ScaffoldContractError;

const MAX_RECORD_BYTES: usize = 4096;
const MAX_CAUSES: usize = 4;

pub(crate) fn emit(record: &str) {
    emit_to(std::io::stderr().lock(), record);
}

fn emit_to(mut writer: impl Write, record: &str) {
    // A broken diagnostic sink must not replace the original backend error.
    let _ = writeln!(writer, "{record}");
}

#[derive(Default)]
struct FirstLine {
    text: String,
    stopped: bool,
}

impl fmt::Write for FirstLine {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.stopped {
            return Err(fmt::Error);
        }
        for character in value.chars() {
            if character == '\n'
                || character == '\r'
                || self.text.len() + character.len_utf8() > 256
            {
                self.stopped = true;
                return Err(fmt::Error);
            }
            self.text.push(character);
        }
        Ok(())
    }
}

fn safe_field(value: impl fmt::Display) -> String {
    use fmt::Write;
    let mut line = FirstLine::default();
    let _ = write!(&mut line, "{value}");
    // Redact the whole field, including spaces in a compiler path. This may
    // also omit useful text or native codes on the same line.
    if line.text.contains(['/', '\\', '@']) {
        return "\"[redacted]\"".into();
    }
    format!("{:?}", line.text)
}

fn adapter_fields(info: &wgpu::AdapterInfo) -> String {
    format!(
        "backend={:?} adapter={} vendor={:#06x} device={:#06x}",
        info.backend,
        safe_field(&info.name),
        info.vendor,
        info.device
    )
}

fn failure_record(info: &wgpu::AdapterInfo, stage: &str, label: &str, error: &dyn Error) -> String {
    let mut record = format!(
        "gpu_startup outcome=failed {} stage={} label={} causes=[",
        adapter_fields(info),
        safe_field(stage),
        safe_field(label)
    );
    let mut cause = Some(error);
    for index in 0..MAX_CAUSES {
        let Some(error) = cause else { break };
        if index != 0 {
            record.push_str(", ");
        }
        record.push_str(&safe_field(error));
        cause = error.source();
    }
    record.push(']');
    if record.len() > MAX_RECORD_BYTES {
        let mut end = MAX_RECORD_BYTES - "...[truncated]".len();
        while !record.is_char_boundary(end) {
            end -= 1;
        }
        record.truncate(end);
        record.push_str("...[truncated]");
    }
    record
}

pub(crate) fn request_device_result<T>(
    result: Result<T, impl Error>,
    info: impl FnOnce() -> wgpu::AdapterInfo,
    label: &str,
    mut report: impl FnMut(&str),
) -> Result<T, ScaffoldContractError> {
    result.map_err(|error| {
        report(&failure_record(&info(), "request_device", label, &error));
        ScaffoldContractError::NeuralBackendUnavailable
    })
}

#[derive(Debug)]
struct DeviceLostMessage<'a>(wgpu::DeviceLostReason, &'a str);

impl fmt::Display for DeviceLostMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "reason={:?}: {}", self.0, self.1)
    }
}

impl Error for DeviceLostMessage<'_> {}

pub(crate) fn device_lost(
    lost: &std::sync::atomic::AtomicBool,
    info: &wgpu::AdapterInfo,
    reason: wgpu::DeviceLostReason,
    message: &str,
    mut report: impl FnMut(&str),
) {
    lost.store(true, std::sync::atomic::Ordering::Release);
    if reason != wgpu::DeviceLostReason::Destroyed {
        // Asynchronous loss cannot be attributed to a particular pipeline.
        report(&failure_record(
            info,
            "device_lost",
            "device_lost_callback",
            &DeviceLostMessage(reason, message),
        ));
    }
}

#[cfg(test)]
mod tests;
