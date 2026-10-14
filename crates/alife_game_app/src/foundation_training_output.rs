//! Failure diagnostics belong only to a directory this invocation created.

use std::{error::Error, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) fn in_new_directory<T>(
    output: &Path,
    run: impl FnOnce() -> Result<T>,
    record_failure: impl FnOnce(&dyn Error),
) -> Result<T> {
    // create_dir is the ownership claim. An existence check before running the
    // operation would race another invocation and cannot establish ownership.
    std::fs::create_dir(output)?;
    let result = run();
    if let Err(error) = &result {
        record_failure(error.as_ref());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, collections::BTreeMap, fs, path::PathBuf};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "alife-training-output-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn snapshot(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name().into(), fs::read(entry.path()).unwrap())
            })
            .collect()
    }

    fn checkpoint(directory: &Path) {
        fs::write(
            directory.join("trained.alife-foundation"),
            b"sealed weights",
        )
        .unwrap();
        fs::write(directory.join("actor-checkpoint.json"), b"sealed optimizer").unwrap();
        fs::write(directory.join("failure.json"), b"original diagnostic").unwrap();
        fs::write(directory.join("failure.txt"), b"original warmup diagnostic").unwrap();
    }

    #[test]
    fn source_as_output_is_unchanged_and_never_runs_or_logs() {
        let source = Fixture::new();
        checkpoint(&source.0);
        let before = snapshot(&source.0);
        let ran = Cell::new(false);
        let logged = Cell::new(false);
        let result: Result<()> = in_new_directory(
            &source.0,
            || {
                ran.set(true);
                Err("training rejected".into())
            },
            |_| {
                logged.set(true);
                fs::write(source.0.join("failure.json"), b"overwritten").unwrap();
            },
        );
        assert!(result.is_err());
        assert!(!ran.get());
        assert!(!logged.get());
        assert_eq!(snapshot(&source.0), before);
    }

    #[test]
    fn existing_foreign_destination_and_source_remain_unchanged() {
        let source = Fixture::new();
        let foreign = Fixture::new();
        checkpoint(&source.0);
        checkpoint(&foreign.0);
        let source_before = snapshot(&source.0);
        let foreign_before = snapshot(&foreign.0);
        let result: Result<()> = in_new_directory(
            &foreign.0,
            || Err("training rejected".into()),
            |_| {
                fs::write(foreign.0.join("failure.txt"), b"overwritten").unwrap();
            },
        );
        assert!(result.is_err());
        assert_eq!(snapshot(&source.0), source_before);
        assert_eq!(snapshot(&foreign.0), foreign_before);
    }

    #[test]
    fn failed_operation_records_diagnostic_in_its_new_output() {
        let fixture = Fixture::new();
        let output = fixture.0.join("new-output");
        let result: Result<()> = in_new_directory(
            &output,
            || {
                fs::write(output.join("phase.txt"), b"source-written")?;
                Err("source validation failed".into())
            },
            |error| {
                fs::write(output.join("failure.txt"), error.to_string()).unwrap();
            },
        );
        assert_eq!(result.unwrap_err().to_string(), "source validation failed");
        assert_eq!(
            fs::read(output.join("phase.txt")).unwrap(),
            b"source-written"
        );
        assert_eq!(
            fs::read_to_string(output.join("failure.txt")).unwrap(),
            "source validation failed"
        );
    }

    #[test]
    fn successful_operation_returns_result_without_failure_log() {
        let fixture = Fixture::new();
        let output = fixture.0.join("new-output");
        let result = in_new_directory(&output, || Ok(17), |_| panic!("unexpected failure log"));
        assert_eq!(result.unwrap(), 17);
        assert!(output.is_dir());
        assert!(!output.join("failure.txt").exists());
    }
}
