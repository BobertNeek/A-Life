//! Best-effort, bounded startup context; never part of a save or GPU authority.
//! Keep only the first line of each of at most four error-chain entries, so
//! formatted WGSL excerpts are omitted. Causes already discarded by wgpu/HAL
//! cannot be recovered here.

use std::{error::Error, fmt, io::Write};

use alife_core::ScaffoldContractError;

use crate::GpuClosedLoopError;

const MAX_RECORD_BYTES: usize = 4096;
const MAX_CAUSES: usize = 4;

pub(crate) fn emit(record: &str) {
    // A closed stderr must not turn an unavailable accelerator into a panic.
    let _ = writeln!(std::io::stderr().lock(), "{record}");
}

struct FirstLine(String);

impl fmt::Write for FirstLine {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for character in value.chars() {
            if character == '\n' || character == '\r' || self.0.len() + character.len_utf8() > 256 {
                return Err(fmt::Error);
            }
            self.0.push(character);
        }
        Ok(())
    }
}

fn safe_field(value: impl fmt::Display) -> String {
    use fmt::Write;
    let mut line = FirstLine(String::new());
    let _ = write!(&mut line, "{value}");
    // Native errors may mention a local compiler path. Never reproduce paths,
    // URLs or account addresses; keep ordinary error text and native codes.
    let redacted = line
        .0
        .split_whitespace()
        .map(|word| {
            if word.contains(['/', '\\', '@']) {
                "[redacted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{redacted:?}")
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
    info: &wgpu::AdapterInfo,
    mut report: impl FnMut(&str),
) -> Result<T, ScaffoldContractError> {
    result.map_err(|error| {
        report(&failure_record(
            info,
            "request_device",
            "alife-required-closed-loop-device",
            &error,
        ));
        ScaffoldContractError::NeuralBackendUnavailable
    })
}

pub(crate) enum AdapterAttempt<T> {
    Skipped,
    RequestFailed,
    Ready(T, wgpu::AdapterInfo),
}

/// The production ordered fallback loop, also usable without a hardware device.
pub(crate) fn select_adapter<A, T>(
    candidates: impl IntoIterator<Item = A>,
    mut attempt: impl FnMut(A) -> Result<AdapterAttempt<T>, ScaffoldContractError>,
    mut report: impl FnMut(&str),
) -> Result<Option<T>, ScaffoldContractError> {
    let mut failures = 0_usize;
    for candidate in candidates {
        match attempt(candidate)? {
            AdapterAttempt::Skipped => {}
            AdapterAttempt::RequestFailed => failures = failures.saturating_add(1),
            AdapterAttempt::Ready(device, info) => {
                if failures != 0 {
                    report(&format!("gpu_startup outcome=fallback_selected {} stage=adapter_selection failed_requests={failures}", adapter_fields(&info)));
                }
                return Ok(Some(device));
            }
        }
    }
    report(&format!(
        "gpu_startup outcome=unavailable stage=adapter_selection failed_requests={failures}"
    ));
    Ok(None)
}

pub(crate) fn scoped_result<T>(
    value: T,
    info: &wgpu::AdapterInfo,
    stage: &str,
    label: &str,
    errors: [Option<wgpu::Error>; 3],
    mut report: impl FnMut(&str),
) -> Result<T, GpuClosedLoopError> {
    let mut failure = None;
    for error in errors.into_iter().flatten() {
        report(&failure_record(info, stage, label, &error));
        let category = match error {
            wgpu::Error::Validation { .. } => GpuClosedLoopError::LayoutMismatch,
            wgpu::Error::Internal { .. } | wgpu::Error::OutOfMemory { .. } => {
                GpuClosedLoopError::SubmissionFailed
            }
        };
        failure = Some(match failure {
            Some(GpuClosedLoopError::SubmissionFailed) => GpuClosedLoopError::SubmissionFailed,
            _ => category,
        });
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

pub(crate) fn scoped<T>(
    device: &wgpu::Device,
    stage: &str,
    label: &str,
    create: impl FnOnce() -> T,
) -> Result<T, GpuClosedLoopError> {
    let out_of_memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = create();
    // Pop every scope, including after an error, in reverse push order.
    let errors = [
        pollster::block_on(validation.pop()),
        pollster::block_on(internal.pop()),
        pollster::block_on(out_of_memory.pop()),
    ];
    scoped_result(value, &device.adapter_info(), stage, label, errors, emit)
}

#[cfg(test)]
mod tests;
