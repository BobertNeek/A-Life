//! Failure-only stage labels; diagnostics never replace the original result.
use std::{fmt::Display, io::Write};

/// Label an existing error without changing its type, value or successful path.
pub fn gpu_failure_context<T, E: Display>(
    stage: &'static str,
    result: Result<T, E>,
) -> Result<T, E> {
    with_writer(stage, result, std::io::stderr)
}

fn with_writer<T, E: Display, W: Write>(
    stage: &'static str,
    result: Result<T, E>,
    writer: impl FnOnce() -> W,
) -> Result<T, E> {
    if let Err(error) = &result {
        // A closed/broken diagnostic stream must not mask the native error.
        let _ = writeln!(writer(), "[alife-gpu-stage:{stage}] {error}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuRuntimeError;
    use alife_core::ScaffoldContractError;

    #[test]
    fn success_keeps_payload_and_never_opens_diagnostics() {
        let payload = Box::new(42);
        let address = &*payload as *const i32;
        let result = with_writer::<_, GpuRuntimeError, Vec<u8>>("unused", Ok(payload), || {
            panic!("successful stage opened diagnostics")
        });
        assert_eq!(&*result.unwrap() as *const i32, address);
    }

    #[test]
    fn failure_keeps_core_variant_and_writes_exact_stage() {
        let mut bytes = Vec::new();
        let result: Result<(), _> = with_writer(
            "checkpoint.restore_upload",
            Err(GpuRuntimeError::Core(
                ScaffoldContractError::NeuralBackendUnavailable,
            )),
            || &mut bytes,
        );
        assert!(matches!(
            result,
            Err(GpuRuntimeError::Core(
                ScaffoldContractError::NeuralBackendUnavailable
            ))
        ));
        assert_eq!(String::from_utf8(bytes).unwrap(),
            "[alife-gpu-stage:checkpoint.restore_upload] core contract error: the GPU-authoritative neural backend is unavailable\n");
    }

    #[test]
    fn broken_diagnostic_writer_cannot_replace_original_error() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let result: Result<(), _> = with_writer(
            "care.restore",
            Err(std::io::Error::from_raw_os_error(13)),
            || Broken,
        );
        assert_eq!(result.unwrap_err().raw_os_error(), Some(13));
    }
}
