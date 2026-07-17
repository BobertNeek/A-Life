//! Git provenance and same-directory atomic storage for GPU evidence.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use alife_core::BrainClassId;

use super::{
    capacity_slug, GitProvenance, GpuEvidenceError, GpuSliceAAcceptanceReceipt,
    GPU_EVIDENCE_MAX_ARTIFACT_BYTES,
};

pub(super) fn read_git_provenance() -> Result<GitProvenance, GpuEvidenceError> {
    let commit = git_stdout(&["rev-parse", "HEAD"])?;
    let tree = git_stdout(&["rev-parse", "HEAD^{tree}"])?;
    let status = git_stdout_allow_empty(&["status", "--porcelain", "--untracked-files=all"])?;
    if !is_lower_hex_oid(&commit) || !is_lower_hex_oid(&tree) {
        return Err(GpuEvidenceError::Git(
            "Git commit and tree identities must be lowercase 40-hex SHA-1 object IDs".to_string(),
        ));
    }
    Ok(GitProvenance {
        commit,
        tree,
        clean: status.is_empty(),
    })
}

fn git_stdout(args: &[&str]) -> Result<String, GpuEvidenceError> {
    let value = git_stdout_allow_empty(args)?;
    if value.is_empty() {
        return Err(GpuEvidenceError::Git(format!(
            "git {} returned no value",
            args.join(" ")
        )));
    }
    Ok(value)
}

fn git_stdout_allow_empty(args: &[&str]) -> Result<String, GpuEvidenceError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root())
        .output()
        .map_err(|error| GpuEvidenceError::Git(error.to_string()))?;
    if !output.status.success() {
        return Err(GpuEvidenceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_game_app remains under workspace/crates")
        .to_path_buf()
}

pub(super) fn is_lower_hex_oid(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn validate_output_filename(
    path: &Path,
    class_id: BrainClassId,
) -> Result<(), GpuEvidenceError> {
    let expected = format!("gpu-closed-loop-slice-a-{}.json", capacity_slug(class_id)?);
    if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
        return Err(GpuEvidenceError::Contract(
            "Slice A output must use its exact class-qualified artifact filename",
        ));
    }
    Ok(())
}

pub(super) fn atomic_write_receipt(
    path: &Path,
    receipt: &GpuSliceAAcceptanceReceipt,
) -> Result<(), GpuEvidenceError> {
    let parent = path.parent().ok_or(GpuEvidenceError::Contract(
        "GPU evidence output has no parent directory",
    ))?;
    fs::create_dir_all(parent)?;
    let filename =
        path.file_name()
            .and_then(|value| value.to_str())
            .ok_or(GpuEvidenceError::Contract(
                "GPU evidence output filename is not UTF-8",
            ))?;
    let temporary = parent.join(format!(".{filename}.{}.tmp", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > GPU_EVIDENCE_MAX_ARTIFACT_BYTES {
        return Err(GpuEvidenceError::Contract(
            "serialized GPU evidence exceeds its artifact bound",
        ));
    }
    let write_result = (|| -> Result<(), GpuEvidenceError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}
