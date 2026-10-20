//! Test output belongs to its resource owners until their destructors finish.
use std::{io, path::Path};

pub(super) fn remove_after_releasing<T>(output: &Path, owners: T) -> io::Result<()> {
    drop(owners);
    std::fs::remove_dir_all(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_archive::{LineageLibrary, LineageLibraryConfig};
    use std::{cell::Cell, path::PathBuf, rc::Rc};

    fn output_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "alife-rehearsal-cleanup-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    struct ArchiveOwners {
        archives: Vec<LineageLibrary>,
        database: PathBuf,
        released: Rc<Cell<bool>>,
    }

    impl Drop for ArchiveOwners {
        fn drop(&mut self) {
            // POSIX can unlink an open SQLite database. This witness makes a
            // delete-before-drop regression fail on those hosts as well.
            assert!(self.database.is_file(), "cleanup preceded archive release");
            self.archives.clear();
            self.released.set(true);
        }
    }

    #[test]
    fn cleanup_releases_both_real_archive_connections_before_removal() {
        let output = output_path("archives");
        let archive_root = output.join("lineage");
        let config = LineageLibraryConfig::profile_default(&archive_root);
        let archives = vec![
            LineageLibrary::open(config.clone()).unwrap(),
            LineageLibrary::open(config).unwrap(),
        ];
        let released = Rc::new(Cell::new(false));
        let owners = ArchiveOwners {
            archives,
            database: archive_root.join("lineage.db"),
            released: Rc::clone(&released),
        };
        assert!(!released.get());
        remove_after_releasing(&output, owners).unwrap();
        assert!(released.get());
        assert!(!output.exists());
    }

    #[test]
    fn cleanup_failure_is_propagated_after_owned_resources_are_released() {
        struct Release(Rc<Cell<bool>>);
        impl Drop for Release {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let output = output_path("missing");
        let released = Rc::new(Cell::new(false));
        let error = remove_after_releasing(&output, Release(Rc::clone(&released))).unwrap_err();
        assert!(released.get());
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
