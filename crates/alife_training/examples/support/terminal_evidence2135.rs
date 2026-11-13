//! LOCAL-only evidence persistence. Never overwrites or retries a run directory.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
pub struct LocalEvidence {
    directory: PathBuf,
    last_valid: Option<Vec<u8>>,
}
impl LocalEvidence {
    pub fn begin(root: &Path, id: &str) -> io::Result<Self> {
        if id.is_empty()
            || id.len() > 96
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(io::Error::other("simple unique evidence_id required"));
        }
        fs::create_dir_all(root)?;
        // Raw evidence must not be uploaded. Never alter an existing ignore file.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(".gitignore"))
        {
            Ok(mut file) => {
                file.write_all(b"*\n")?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let directory = root.join(id);
        fs::create_dir(&directory)?;
        Ok(Self {
            directory,
            last_valid: None,
        })
    }
    pub fn save(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err(io::Error::other("invalid evidence filename"));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.directory.join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        #[cfg(unix)]
        fs::File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
    pub fn remember_valid(&mut self, bytes: Vec<u8>) {
        self.last_valid = Some(bytes);
    }
    pub fn retain_error_checkpoint(&self) -> io::Result<bool> {
        if let Some(bytes) = &self.last_valid {
            self.save("ERROR-LAST-VALID.cp.json", bytes)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "terminal-evidence-{}-{}-{tag}-LOCAL",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn create_only_and_error_checkpoint_preserve_bytes() {
        let root = root("retention");
        let mut e = LocalEvidence::begin(&root, "seed1-active").unwrap();
        e.save("BIRTH.cp.json", b"native-birth").unwrap();
        assert!(e.save("BIRTH.cp.json", b"overwrite").is_err());
        assert!(LocalEvidence::begin(&root, "seed1-active").is_err());
        e.remember_valid(b"latest-native-state".to_vec());
        assert!(e.retain_error_checkpoint().unwrap());
        assert_eq!(
            fs::read(e.directory().join("ERROR-LAST-VALID.cp.json")).unwrap(),
            b"latest-native-state"
        );
        assert_eq!(
            fs::read(e.directory().join("BIRTH.cp.json")).unwrap(),
            b"native-birth"
        );
        assert!(e.retain_error_checkpoint().is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn new_roots_are_created_and_unsafe_ids_reject() {
        let root = root("guard");
        let nested = root.join("ordinary-name");
        assert!(LocalEvidence::begin(&nested, "../reuse").is_err());
        let e = LocalEvidence::begin(&nested, "id").unwrap();
        assert_eq!(fs::read(nested.join(".gitignore")).unwrap(), b"*\n");
        assert!(e.directory().is_dir());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_native_state_is_explicit() {
        let root = root("early-error");
        let e = LocalEvidence::begin(&root, "id").unwrap();
        assert!(!e.retain_error_checkpoint().unwrap());
        assert!(e.save("../escape", b"bad").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
