//! Developer evidence ingestion for production GPU brain-class promotion.
//!
//! This module never executes a neural tick. It accepts only already validated
//! A/B/C/D, benchmark, and gate bindings, then derives promotion from the
//! complete matrix without configuration overrides.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use alife_core::{
    BrainCapacityClass, BrainClassId, CanonicalDigestBuilder, PhenotypeHash, SensorProfile,
};
use alife_gpu_backend::GpuHardwareReceipt;
use alife_world::persistence::GpuBackendProvenanceSave;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

const PROMOTION_SCHEMA_VERSION: u16 = 1;
const SLICE_ARTIFACT_SCHEMA_VERSION: u16 = 1;
const PROFILE_SCHEMA_VERSION: u16 = 1;
const PASSING_STATUS_RAW: u16 = 1;
const MAX_STATUS_RAW: u16 = 4;
const EXPECTED_POPULATIONS: [u32; 6] = [1, 10, 50, 100, 250, 500];

const ADAPTER_DOMAIN: &[u8] = b"alife.gpu.promotion.adapter.v1";
const ARTIFACT_BINDING_DOMAIN: &[u8] = b"alife.gpu.promotion.artifact-binding.v1";
const BENCHMARK_ROW_DOMAIN: &[u8] = b"alife.gpu.promotion.benchmark-row.v1";
const BENCHMARK_ROWS_DOMAIN: &[u8] = b"alife.gpu.promotion.benchmark-rows.v1";
const INPUT_DOMAIN: &[u8] = b"alife.gpu.promotion.input.v1";
const ROW_DOMAIN: &[u8] = b"alife.gpu.promotion.row.v1";
const MANIFEST_DOMAIN: &[u8] = b"alife.gpu.promotion.manifest.v1";

const GATE_SLICE_A: u64 = 1 << 0;
const GATE_SLICE_B: u64 = 1 << 1;
const GATE_SLICE_C_PRIVILEGED: u64 = 1 << 2;
const GATE_SLICE_C_GROUNDED: u64 = 1 << 3;
const GATE_SLICE_D_PRIVILEGED: u64 = 1 << 4;
const GATE_SLICE_D_GROUNDED: u64 = 1 << 5;
const GATE_BENCHMARK: u64 = 1 << 6;
const GATE_GLOBAL: u64 = 1 << 7;
const REQUIRED_GATE_BITS: u64 = GATE_SLICE_A
    | GATE_SLICE_B
    | GATE_SLICE_C_PRIVILEGED
    | GATE_SLICE_C_GROUNDED
    | GATE_SLICE_D_PRIVILEGED
    | GATE_SLICE_D_GROUNDED
    | GATE_BENCHMARK
    | GATE_GLOBAL;

#[derive(Debug, Error)]
pub enum PromotionEvidenceError {
    #[error("GPU promotion evidence is inconsistent: {0}")]
    Contract(&'static str),
    #[error("GPU promotion evidence is inconsistent: {0}")]
    Detail(String),
    #[error(transparent)]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error(transparent)]
    Evidence(#[from] crate::GpuEvidenceError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromotionArtifactPaths {
    pub slice_a: Vec<PathBuf>,
    pub slice_b: Vec<PathBuf>,
    pub slice_c: Vec<PathBuf>,
    pub slice_d: Vec<PathBuf>,
    pub benchmark: PathBuf,
    pub gates: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitObjectId(pub [u8; 20]);

impl GitObjectId {
    pub fn from_hex(value: &str) -> Result<Self, PromotionEvidenceError> {
        if value.len() != 40
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(PromotionEvidenceError::Contract(
                "Git object IDs must be lowercase 40-hex strings",
            ));
        }
        let mut bytes = [0_u8; 20];
        for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(chunk[0])? << 4) | hex_nibble(chunk[1])?;
        }
        Ok(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        let mut value = String::with_capacity(40);
        for byte in self.0 {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing to a String cannot fail");
        }
        value
    }
}

impl Serialize for GitObjectId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for GitObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAdapterBinding {
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub identity_digest: [u64; 4],
}

impl EvidenceAdapterBinding {
    pub fn new(
        vendor_id: u32,
        device_id: u32,
        backend_api_raw: u16,
        driver_digest: [u64; 4],
        feature_digest: [u64; 4],
        limits_digest: [u64; 4],
    ) -> Result<Self, PromotionEvidenceError> {
        let mut binding = Self {
            vendor_id,
            device_id,
            backend_api_raw,
            driver_digest,
            feature_digest,
            limits_digest,
            identity_digest: [0; 4],
        };
        binding.identity_digest = binding.recompute_identity_digest();
        binding.validate()?;
        Ok(binding)
    }

    pub fn validate(self) -> Result<(), PromotionEvidenceError> {
        if self.vendor_id == 0
            || self.device_id == 0
            || self.backend_api_raw != 1
            || any_zero([
                self.driver_digest,
                self.feature_digest,
                self.limits_digest,
                self.identity_digest,
            ])
            || self.identity_digest != self.recompute_identity_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "adapter binding is invalid",
            ));
        }
        Ok(())
    }

    fn recompute_identity_digest(self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(ADAPTER_DOMAIN);
        digest.write_u32(self.vendor_id);
        digest.write_u32(self.device_id);
        digest.write_u16(self.backend_api_raw);
        write_digest4(&mut digest, self.driver_digest);
        write_digest4(&mut digest, self.feature_digest);
        write_digest4(&mut digest, self.limits_digest);
        digest.finish256()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceArtifactBinding {
    pub slice_raw: u16,
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub artifact_schema: u16,
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub artifact_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
    pub status_raw: u16,
}

impl EvidenceArtifactBinding {
    fn key(self) -> (u16, u16, u16) {
        (self.class_id_raw, self.slice_raw, self.profile_id_raw)
    }

    fn validate(self) -> Result<(), PromotionEvidenceError> {
        let capacity = production_capacity(self.class_id_raw)?;
        self.adapter.validate()?;
        let profile_is_valid = match self.slice_raw {
            1 | 2 => self.profile_id_raw == 0 && self.profile_schema == 0,
            3 | 4 => {
                matches!(
                    self.profile_id_raw,
                    value if value == SensorProfile::PrivilegedAffordanceV1.raw()
                        || value == SensorProfile::GroundedObjectSlotsV1.raw()
                ) && self.profile_schema == PROFILE_SCHEMA_VERSION
            }
            _ => false,
        };
        if !profile_is_valid
            || self.artifact_schema != SLICE_ARTIFACT_SCHEMA_VERSION
            || !(1..=MAX_STATUS_RAW).contains(&self.status_raw)
            || self.artifact_digest == [0; 4]
            || self.phenotype_hash.0 == [0; 4]
            || self.phenotype_manifest_digest == [0; 4]
            || self.capacity_digest != capacity.canonical_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "slice artifact binding is malformed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkRowBinding {
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub population: u32,
    pub status_raw: u16,
    pub row_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
}

impl BenchmarkRowBinding {
    fn key(self) -> (u16, u16, u32) {
        (self.class_id_raw, self.profile_id_raw, self.population)
    }

    fn validate(self) -> Result<(), PromotionEvidenceError> {
        let capacity = production_capacity(self.class_id_raw)?;
        self.adapter.validate()?;
        if !matches!(
            self.profile_id_raw,
            value if value == SensorProfile::PrivilegedAffordanceV1.raw()
                || value == SensorProfile::GroundedObjectSlotsV1.raw()
        ) || self.profile_schema != PROFILE_SCHEMA_VERSION
            || !EXPECTED_POPULATIONS.contains(&self.population)
            || !(1..=MAX_STATUS_RAW).contains(&self.status_raw)
            || any_zero([
                self.row_digest,
                self.phenotype_hash.0,
                self.phenotype_manifest_digest,
                self.capacity_digest,
                self.protocol_digest,
            ])
            || self.capacity_digest != capacity.canonical_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "benchmark row binding is malformed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkManifestBinding {
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub manifest_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
    pub row_bindings_digest: [u64; 4],
}

impl BenchmarkManifestBinding {
    pub fn digest_rows(rows: &[BenchmarkRowBinding]) -> [u64; 4] {
        let mut rows = rows.to_vec();
        rows.sort_unstable_by_key(|row| row.key());
        let mut digest = CanonicalDigestBuilder::new(BENCHMARK_ROWS_DOMAIN);
        digest.write_sequence_len(rows.len());
        for row in rows {
            encode_benchmark_row(&mut digest, row);
        }
        digest.finish256()
    }

    fn validate(self, rows: &[BenchmarkRowBinding]) -> Result<(), PromotionEvidenceError> {
        self.adapter.validate()?;
        if any_zero([
            self.manifest_digest,
            self.protocol_digest,
            self.row_bindings_digest,
        ]) || self.row_bindings_digest != Self::digest_rows(rows)
        {
            return Err(PromotionEvidenceError::Contract(
                "benchmark manifest binding is inconsistent",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidenceBinding {
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub receipt_digest: [u64; 4],
    pub gate_script_digest: [u64; 4],
    pub commands_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
}

impl GateEvidenceBinding {
    fn validate(self) -> Result<(), PromotionEvidenceError> {
        self.adapter.validate()?;
        if any_zero([
            self.receipt_digest,
            self.gate_script_digest,
            self.commands_digest,
        ]) {
            return Err(PromotionEvidenceError::Contract(
                "gate receipt binding is inconsistent",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCommandReceipt {
    pub command_id: u16,
    pub argv_utf8: Vec<u8>,
    pub started_monotonic_ns: u64,
    pub ended_monotonic_ns: u64,
    pub exit_code: i32,
    pub stdout_digest: [u64; 4],
    pub stderr_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateCommandCapture {
    pub command_id: u16,
    pub argv_utf8: Vec<u8>,
    pub started_monotonic_ns: u64,
    pub ended_monotonic_ns: u64,
    pub exit_code: i32,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuGateCaptureManifest {
    pub schema_version: u16,
    pub git_commit: String,
    pub source_tree_digest: String,
    pub commands: Vec<GateCommandCapture>,
}

pub fn canonical_gate_command_argv(command_id: u16) -> Option<Vec<u8>> {
    let arguments: &[&str] = match command_id {
        1 => &["cargo", "fmt", "--all", "--", "--check"],
        2 => &[
            "cargo",
            "check",
            "--workspace",
            "--all-targets",
            "--all-features",
            "-j",
            "1",
        ],
        3 => &["cargo", "test", "--workspace", "--all-features", "-j", "1"],
        4 => &[
            "cargo",
            "test",
            "-p",
            "alife_core",
            "--test",
            "production_brain_budgets",
            "--test",
            "phenotype_compiler",
            "--test",
            "brain_topology",
        ],
        5 => &[
            "cargo",
            "test",
            "-p",
            "alife_gpu_backend",
            "--features",
            "gpu-tests",
            "--test",
            "closed_loop_runtime",
            "--test",
            "closed_loop_admission",
            "--test",
            "closed_loop_activity",
            "--test",
            "closed_loop_gpu_behavior",
            "--test",
            "closed_loop_eligibility",
            "--test",
            "closed_loop_fast_plasticity",
            "--test",
            "closed_loop_sleep",
            "--test",
            "closed_loop_memory_context",
            "--",
            "--nocapture",
        ],
        6 => &[
            "cargo",
            "test",
            "-p",
            "alife_world",
            "--test",
            "gpu_brain_persistence",
            "--test",
            "gpu_brain_vnext_migration",
            "--test",
            "gpu_memory_grounding_persistence",
        ],
        7 => &[
            "cargo",
            "test",
            "-p",
            "alife_game_app",
            "--features",
            "gpu-runtime gpu-tests",
            "--test",
            "gpu_closed_loop_acceptance",
            "--test",
            "gpu_learning_sleep_acceptance",
            "--test",
            "gpu_memory_grounding_acceptance",
            "--test",
            "gpu_sleep_restore",
            "--test",
            "gpu_closed_loop_soak",
            "--test",
            "gpu_brain_authority_audit",
            "--test",
            "gpu_closed_loop_promotion",
            "-j",
            "1",
            "--",
            "--nocapture",
        ],
        8 => &[
            "cargo",
            "test",
            "-p",
            "alife_tools",
            "--test",
            "benchmark_tiers",
        ],
        9 => &[
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "scripts/docs_check.ps1",
        ],
        10 => &[
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "scripts/check_core_boundaries.ps1",
        ],
        11 => &["internal", "authority-scan-v1"],
        12 => &["git", "diff", "--check", "origin/main...HEAD"],
        _ => return None,
    };
    Some(join_argv(arguments))
}

impl GateCommandReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: u16,
        argv_utf8: Vec<u8>,
        started_monotonic_ns: u64,
        ended_monotonic_ns: u64,
        exit_code: i32,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Result<Self, PromotionEvidenceError> {
        let receipt = Self {
            command_id,
            argv_utf8,
            started_monotonic_ns,
            ended_monotonic_ns,
            exit_code,
            stdout_digest: captured_stream_digest(b"stdout", command_id, stdout),
            stderr_digest: captured_stream_digest(b"stderr", command_id, stderr),
        };
        receipt.validate_shape()?;
        Ok(receipt)
    }

    fn validate_shape(&self) -> Result<(), PromotionEvidenceError> {
        if !(1..=12).contains(&self.command_id)
            || self.argv_utf8.is_empty()
            || self.argv_utf8.len() > 2_048
            || self.argv_utf8.first() == Some(&0)
            || self.argv_utf8.last() == Some(&0)
            || self.argv_utf8.windows(2).any(|pair| pair == [0, 0])
            || std::str::from_utf8(&self.argv_utf8).is_err()
            || self.started_monotonic_ns >= self.ended_monotonic_ns
            || any_zero([self.stdout_digest, self.stderr_digest])
        {
            return Err(PromotionEvidenceError::Contract(
                "gate command receipt is malformed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuGateAdapterReceipt {
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub adapter_name_len: u16,
    #[serde(with = "fixed_bytes_128")]
    pub adapter_name_utf8: [u8; 128],
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub identity_digest: [u64; 4],
}

impl GpuGateAdapterReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vendor_id: u32,
        device_id: u32,
        backend_api_raw: u16,
        adapter_name: &str,
        driver_digest: [u64; 4],
        feature_digest: [u64; 4],
        limits_digest: [u64; 4],
    ) -> Result<Self, PromotionEvidenceError> {
        let bytes = adapter_name.as_bytes();
        if bytes.is_empty() || bytes.len() > 128 {
            return Err(PromotionEvidenceError::Contract(
                "gate adapter name is outside its UTF-8 bound",
            ));
        }
        let mut adapter_name_utf8 = [0_u8; 128];
        adapter_name_utf8[..bytes.len()].copy_from_slice(bytes);
        let binding = EvidenceAdapterBinding::new(
            vendor_id,
            device_id,
            backend_api_raw,
            driver_digest,
            feature_digest,
            limits_digest,
        )?;
        let receipt = Self {
            vendor_id,
            device_id,
            backend_api_raw,
            adapter_name_len: u16::try_from(bytes.len()).map_err(|_| {
                PromotionEvidenceError::Contract("gate adapter name length does not fit u16")
            })?,
            adapter_name_utf8,
            driver_digest,
            feature_digest,
            limits_digest,
            identity_digest: binding.identity_digest,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn binding(self) -> Result<EvidenceAdapterBinding, PromotionEvidenceError> {
        self.validate()?;
        EvidenceAdapterBinding::new(
            self.vendor_id,
            self.device_id,
            self.backend_api_raw,
            self.driver_digest,
            self.feature_digest,
            self.limits_digest,
        )
    }

    pub fn from_hardware(hardware: &GpuHardwareReceipt) -> Result<Self, PromotionEvidenceError> {
        if hardware.backend_api != "vulkan" {
            return Err(PromotionEvidenceError::Contract(
                "gate evidence must come from Vulkan",
            ));
        }
        Self::new(
            hardware.vendor_id,
            hardware.device_id,
            1,
            &hardware.adapter_name,
            hardware.driver_digest,
            hardware.feature_digest,
            hardware.limits_digest,
        )
    }

    pub fn adapter_name(self) -> Result<String, PromotionEvidenceError> {
        self.validate()?;
        let len = usize::from(self.adapter_name_len);
        Ok(std::str::from_utf8(&self.adapter_name_utf8[..len])
            .map_err(|_| PromotionEvidenceError::Contract("gate adapter name is not UTF-8"))?
            .to_string())
    }

    fn validate(self) -> Result<(), PromotionEvidenceError> {
        let len = usize::from(self.adapter_name_len);
        if len == 0
            || len > self.adapter_name_utf8.len()
            || std::str::from_utf8(&self.adapter_name_utf8[..len]).is_err()
            || self.adapter_name_utf8[len..].iter().any(|byte| *byte != 0)
        {
            return Err(PromotionEvidenceError::Contract(
                "gate adapter display identity is malformed",
            ));
        }
        let binding = EvidenceAdapterBinding {
            vendor_id: self.vendor_id,
            device_id: self.device_id,
            backend_api_raw: self.backend_api_raw,
            driver_digest: self.driver_digest,
            feature_digest: self.feature_digest,
            limits_digest: self.limits_digest,
            identity_digest: self.identity_digest,
        };
        binding.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopGateReceipt {
    pub schema_version: u16,
    pub git_commit: GitObjectId,
    pub source_tree_digest: GitObjectId,
    pub adapter: GpuGateAdapterReceipt,
    pub gate_script_digest: [u64; 4],
    pub commands: Vec<GateCommandReceipt>,
    pub commands_digest: [u64; 4],
    pub passed: bool,
    pub receipt_digest: [u64; 4],
}

impl GpuClosedLoopGateReceipt {
    pub fn new(
        git_commit: GitObjectId,
        source_tree_digest: GitObjectId,
        adapter: GpuGateAdapterReceipt,
        gate_script_bytes: &[u8],
        commands: Vec<GateCommandReceipt>,
    ) -> Result<Self, PromotionEvidenceError> {
        if gate_script_bytes.is_empty() {
            return Err(PromotionEvidenceError::Contract(
                "gate script bytes are empty",
            ));
        }
        let mut receipt = Self {
            schema_version: 1,
            git_commit,
            source_tree_digest,
            adapter,
            gate_script_digest: captured_stream_digest(b"gate-script", 0, gate_script_bytes),
            commands_digest: commands_digest(&commands),
            commands,
            passed: true,
            receipt_digest: [0; 4],
        };
        receipt.receipt_digest = receipt.recompute_digest();
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), PromotionEvidenceError> {
        self.adapter.validate()?;
        if self.schema_version != 1
            || self.commands.len() != 12
            || !self.passed
            || any_zero([
                self.gate_script_digest,
                self.commands_digest,
                self.receipt_digest,
            ])
            || self.commands_digest != commands_digest(&self.commands)
            || self.receipt_digest != self.recompute_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "gate receipt header or digest is inconsistent",
            ));
        }
        for (index, command) in self.commands.iter().enumerate() {
            command.validate_shape()?;
            if command.command_id != index as u16 + 1
                || command.exit_code != 0
                || canonical_gate_command_argv(command.command_id).as_deref()
                    != Some(command.argv_utf8.as_slice())
            {
                return Err(PromotionEvidenceError::Contract(
                    "gate commands are missing, reordered, changed, or failed",
                ));
            }
        }
        Ok(())
    }

    pub fn binding(&self) -> Result<GateEvidenceBinding, PromotionEvidenceError> {
        self.validate()?;
        Ok(GateEvidenceBinding {
            evidence_commit: self.git_commit,
            source_tree: self.source_tree_digest,
            receipt_digest: self.receipt_digest,
            gate_script_digest: self.gate_script_digest,
            commands_digest: self.commands_digest,
            adapter: self.adapter.binding()?,
        })
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(b"alife.gpu.promotion.gate-receipt.v1");
        digest.write_u16(self.schema_version);
        write_oid(&mut digest, self.git_commit);
        write_oid(&mut digest, self.source_tree_digest);
        encode_gate_adapter_receipt(&mut digest, self.adapter);
        write_digest4(&mut digest, self.gate_script_digest);
        write_digest4(&mut digest, self.commands_digest);
        digest.write_bool(self.passed);
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionEvidenceInputs {
    pub promotion_commit: GitObjectId,
    pub source_tree_digest: GitObjectId,
    pub adapter: EvidenceAdapterBinding,
    pub gate: GateEvidenceBinding,
    pub benchmark: BenchmarkManifestBinding,
    pub artifact_bindings: Vec<EvidenceArtifactBinding>,
    pub benchmark_rows: Vec<BenchmarkRowBinding>,
    pub trusted_ancestor_commits: Vec<GitObjectId>,
    input_digest: [u64; 4],
}

impl PromotionEvidenceInputs {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        promotion_commit: GitObjectId,
        source_tree_digest: GitObjectId,
        adapter: EvidenceAdapterBinding,
        gate: GateEvidenceBinding,
        benchmark: BenchmarkManifestBinding,
        artifact_bindings: Vec<EvidenceArtifactBinding>,
        benchmark_rows: Vec<BenchmarkRowBinding>,
        trusted_ancestor_commits: Vec<GitObjectId>,
    ) -> Result<Self, PromotionEvidenceError> {
        let mut inputs = Self {
            promotion_commit,
            source_tree_digest,
            adapter,
            gate,
            benchmark,
            artifact_bindings,
            benchmark_rows,
            trusted_ancestor_commits,
            input_digest: [0; 4],
        };
        inputs.input_digest = inputs.recompute_digest();
        Ok(inputs)
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(INPUT_DOMAIN);
        write_oid(&mut digest, self.promotion_commit);
        write_oid(&mut digest, self.source_tree_digest);
        encode_adapter(&mut digest, self.adapter);
        encode_gate(&mut digest, self.gate);
        encode_benchmark_manifest(&mut digest, self.benchmark);
        digest.write_sequence_len(self.artifact_bindings.len());
        for binding in &self.artifact_bindings {
            encode_artifact(&mut digest, *binding);
        }
        digest.write_sequence_len(self.benchmark_rows.len());
        for binding in &self.benchmark_rows {
            encode_benchmark_row(&mut digest, *binding);
        }
        digest.write_sequence_len(self.trusted_ancestor_commits.len());
        for commit in &self.trusted_ancestor_commits {
            write_oid(&mut digest, *commit);
        }
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionEvidenceRow {
    pub class_id_raw: u16,
    pub canonical_capacity_digest: [u64; 4],
    pub artifact_bindings: Vec<EvidenceArtifactBinding>,
    pub benchmark_rows: Vec<BenchmarkRowBinding>,
    pub required_gate_bits: u64,
    pub passed_gate_bits: u64,
    pub row_digest: [u64; 4],
}

impl PromotionEvidenceRow {
    pub const fn all_required_gates_pass(&self) -> bool {
        self.passed_gate_bits == self.required_gate_bits
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(ROW_DOMAIN);
        digest.write_u16(self.class_id_raw);
        write_digest4(&mut digest, self.canonical_capacity_digest);
        digest.write_sequence_len(self.artifact_bindings.len());
        for binding in &self.artifact_bindings {
            encode_artifact(&mut digest, *binding);
        }
        digest.write_sequence_len(self.benchmark_rows.len());
        for row in &self.benchmark_rows {
            encode_benchmark_row(&mut digest, *row);
        }
        digest.write_u64(self.required_gate_bits);
        digest.write_u64(self.passed_gate_bits);
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopPromotionManifest {
    pub schema_version: u16,
    pub promotion_commit: GitObjectId,
    pub source_tree_digest: GitObjectId,
    pub adapter: EvidenceAdapterBinding,
    pub gate: GateEvidenceBinding,
    pub benchmark: BenchmarkManifestBinding,
    pub rows: Vec<PromotionEvidenceRow>,
    pub promoted_classes: Vec<BrainClassId>,
    pub manifest_digest: [u64; 4],
}

impl GpuClosedLoopPromotionManifest {
    pub fn validate(&self) -> Result<(), PromotionEvidenceError> {
        self.adapter.validate()?;
        self.gate.validate()?;
        let expected_classes = production_class_ids();
        if self.schema_version != PROMOTION_SCHEMA_VERSION
            || self.rows.len() != expected_classes.len()
        {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest rows are inconsistent",
            ));
        }
        let mut all_benchmark_rows = Vec::with_capacity(36);
        for (row, class) in self.rows.iter().zip(expected_classes) {
            self.validate_row(row, class)?;
            all_benchmark_rows.extend_from_slice(&row.benchmark_rows);
        }
        self.benchmark.validate(&all_benchmark_rows)?;
        let derived = self
            .rows
            .iter()
            .filter(|row| row.all_required_gates_pass())
            .map(|row| BrainClassId(row.class_id_raw))
            .collect::<Vec<_>>();
        if self.promoted_classes != derived
            || self.gate.evidence_commit != self.promotion_commit
            || self.gate.source_tree != self.source_tree_digest
            || self.gate.adapter != self.adapter
            || self.benchmark.source_tree != self.source_tree_digest
            || self.benchmark.adapter != self.adapter
            || self.manifest_digest != self.recompute_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest identity or digest is inconsistent",
            ));
        }
        Ok(())
    }

    fn validate_row(
        &self,
        row: &PromotionEvidenceRow,
        class: BrainClassId,
    ) -> Result<(), PromotionEvidenceError> {
        let capacity = production_capacity(class.raw())?;
        if row.class_id_raw != class.raw()
            || row.canonical_capacity_digest != capacity.canonical_digest()
            || row.required_gate_bits != REQUIRED_GATE_BITS
            || row.row_digest != row.recompute_digest()
        {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest row identity or digest is inconsistent",
            ));
        }

        let expected_artifacts = BTreeSet::from([
            (class.raw(), 1, 0),
            (class.raw(), 2, 0),
            (class.raw(), 3, SensorProfile::PrivilegedAffordanceV1.raw()),
            (class.raw(), 3, SensorProfile::GroundedObjectSlotsV1.raw()),
            (class.raw(), 4, SensorProfile::PrivilegedAffordanceV1.raw()),
            (class.raw(), 4, SensorProfile::GroundedObjectSlotsV1.raw()),
        ]);
        let mut artifact_keys = BTreeSet::new();
        let mut passed_gate_bits = GATE_GLOBAL;
        for binding in &row.artifact_bindings {
            binding.validate()?;
            if binding.class_id_raw != class.raw()
                || binding.capacity_digest != row.canonical_capacity_digest
                || binding.source_tree != self.source_tree_digest
                || binding.adapter != self.adapter
                || !artifact_keys.insert(binding.key())
            {
                return Err(PromotionEvidenceError::Contract(
                    "promotion manifest artifact row is inconsistent",
                ));
            }
            if binding.status_raw == PASSING_STATUS_RAW {
                passed_gate_bits |= artifact_gate_bit(*binding)?;
            }
        }
        if !artifact_keys.is_subset(&expected_artifacts) {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest artifact matrix contains an unexpected binding",
            ));
        }

        let expected_benchmarks = [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ]
        .into_iter()
        .flat_map(|profile| {
            EXPECTED_POPULATIONS
                .into_iter()
                .map(move |population| (class.raw(), profile.raw(), population))
        })
        .collect::<BTreeSet<_>>();
        let mut benchmark_keys = BTreeSet::new();
        for binding in &row.benchmark_rows {
            binding.validate()?;
            if binding.class_id_raw != class.raw()
                || binding.capacity_digest != row.canonical_capacity_digest
                || binding.protocol_digest != self.benchmark.protocol_digest
                || binding.adapter != self.adapter
                || !benchmark_keys.insert(binding.key())
            {
                return Err(PromotionEvidenceError::Contract(
                    "promotion manifest benchmark row is inconsistent",
                ));
            }
        }
        if benchmark_keys != expected_benchmarks {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest benchmark matrix is incomplete",
            ));
        }
        if complete_passing_benchmark_matrix(&row.benchmark_rows) {
            passed_gate_bits |= GATE_BENCHMARK;
        }
        if row.passed_gate_bits != passed_gate_bits {
            return Err(PromotionEvidenceError::Contract(
                "promotion manifest gate bits are not derived from evidence",
            ));
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(MANIFEST_DOMAIN);
        digest.write_u16(self.schema_version);
        write_oid(&mut digest, self.promotion_commit);
        write_oid(&mut digest, self.source_tree_digest);
        encode_adapter(&mut digest, self.adapter);
        encode_gate(&mut digest, self.gate);
        encode_benchmark_manifest(&mut digest, self.benchmark);
        digest.write_sequence_len(self.rows.len());
        for row in &self.rows {
            write_digest4(&mut digest, row.row_digest);
        }
        digest.write_sequence_len(self.promoted_classes.len());
        for class in &self.promoted_classes {
            digest.write_u16(class.raw());
        }
        digest.finish256()
    }
}

pub fn ingest_promotion_evidence(
    inputs: PromotionEvidenceInputs,
) -> Result<GpuClosedLoopPromotionManifest, PromotionEvidenceError> {
    if inputs.input_digest != inputs.recompute_digest() {
        return Err(PromotionEvidenceError::Contract(
            "promotion input changed after validation",
        ));
    }
    inputs.adapter.validate()?;
    inputs.gate.validate()?;
    inputs.benchmark.validate(&inputs.benchmark_rows)?;
    if inputs.gate.evidence_commit != inputs.promotion_commit
        || inputs.gate.source_tree != inputs.source_tree_digest
        || inputs.gate.adapter != inputs.adapter
        || inputs.benchmark.source_tree != inputs.source_tree_digest
        || inputs.benchmark.adapter != inputs.adapter
        || !trusted_commit(&inputs, inputs.benchmark.evidence_commit)
    {
        return Err(PromotionEvidenceError::Contract(
            "global evidence binding is not trusted for this promotion",
        ));
    }
    let mut ancestor_set = BTreeSet::new();
    if inputs
        .trusted_ancestor_commits
        .iter()
        .any(|commit| !ancestor_set.insert(*commit) || *commit == inputs.promotion_commit)
    {
        return Err(PromotionEvidenceError::Contract(
            "trusted evidence ancestry is duplicated or self-referential",
        ));
    }

    let mut artifact_keys = BTreeSet::new();
    for binding in &inputs.artifact_bindings {
        binding.validate()?;
        if !artifact_keys.insert(binding.key())
            || binding.source_tree != inputs.source_tree_digest
            || binding.adapter != inputs.adapter
            || !trusted_commit(&inputs, binding.evidence_commit)
        {
            return Err(PromotionEvidenceError::Contract(
                "slice artifact binding is duplicated or untrusted",
            ));
        }
    }
    let mut benchmark_keys = BTreeSet::new();
    for row in &inputs.benchmark_rows {
        row.validate()?;
        if !benchmark_keys.insert(row.key())
            || row.adapter != inputs.adapter
            || row.protocol_digest != inputs.benchmark.protocol_digest
        {
            return Err(PromotionEvidenceError::Contract(
                "benchmark row binding is duplicated or globally inconsistent",
            ));
        }
    }

    let mut rows = Vec::with_capacity(3);
    for class_id in production_class_ids() {
        let capacity = production_capacity(class_id.raw())?;
        let mut artifact_bindings = inputs
            .artifact_bindings
            .iter()
            .copied()
            .filter(|binding| binding.class_id_raw == class_id.raw())
            .collect::<Vec<_>>();
        artifact_bindings.sort_unstable_by_key(|binding| binding.key());
        let mut benchmark_rows = inputs
            .benchmark_rows
            .iter()
            .copied()
            .filter(|binding| binding.class_id_raw == class_id.raw())
            .collect::<Vec<_>>();
        benchmark_rows.sort_unstable_by_key(|binding| binding.key());

        let mut passed_gate_bits = GATE_GLOBAL;
        for binding in &artifact_bindings {
            if binding.status_raw == PASSING_STATUS_RAW {
                passed_gate_bits |= artifact_gate_bit(*binding)?;
            }
        }
        if complete_passing_benchmark_matrix(&benchmark_rows) {
            passed_gate_bits |= GATE_BENCHMARK;
        }
        let mut row = PromotionEvidenceRow {
            class_id_raw: class_id.raw(),
            canonical_capacity_digest: capacity.canonical_digest(),
            artifact_bindings,
            benchmark_rows,
            required_gate_bits: REQUIRED_GATE_BITS,
            passed_gate_bits,
            row_digest: [0; 4],
        };
        row.row_digest = row.recompute_digest();
        rows.push(row);
    }
    let promoted_classes = rows
        .iter()
        .filter(|row| row.all_required_gates_pass())
        .map(|row| BrainClassId(row.class_id_raw))
        .collect();
    let mut manifest = GpuClosedLoopPromotionManifest {
        schema_version: PROMOTION_SCHEMA_VERSION,
        promotion_commit: inputs.promotion_commit,
        source_tree_digest: inputs.source_tree_digest,
        adapter: inputs.adapter,
        gate: inputs.gate,
        benchmark: inputs.benchmark,
        rows,
        promoted_classes,
        manifest_digest: [0; 4],
    };
    manifest.manifest_digest = manifest.recompute_digest();
    manifest.validate()?;
    Ok(manifest)
}

pub fn write_gpu_closed_loop_promotion_manifest(
    path: impl AsRef<Path>,
    manifest: &GpuClosedLoopPromotionManifest,
) -> Result<(), PromotionEvidenceError> {
    manifest.validate()?;
    atomic_write_validated_json(path.as_ref(), manifest, |path| {
        load_gpu_closed_loop_promotion_manifest(path).map(|_| ())
    })
}

pub fn load_gpu_closed_loop_promotion_manifest(
    path: impl AsRef<Path>,
) -> Result<GpuClosedLoopPromotionManifest, PromotionEvidenceError> {
    let bytes = read_bounded_json(path.as_ref(), 16 * 1024 * 1024)?;
    let manifest: GpuClosedLoopPromotionManifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn write_gpu_closed_loop_gate_receipt(
    path: impl AsRef<Path>,
    receipt: &GpuClosedLoopGateReceipt,
) -> Result<(), PromotionEvidenceError> {
    receipt.validate()?;
    atomic_write_validated_json(path.as_ref(), receipt, |path| {
        load_gpu_closed_loop_gate_receipt(path).map(|_| ())
    })
}

pub fn write_gpu_closed_loop_gate_receipt_from_capture(
    capture_path: impl AsRef<Path>,
    gate_script_path: impl AsRef<Path>,
    adapter: GpuGateAdapterReceipt,
    output: impl AsRef<Path>,
) -> Result<GpuClosedLoopGateReceipt, PromotionEvidenceError> {
    let capture: GpuGateCaptureManifest =
        serde_json::from_slice(&read_bounded_json(capture_path.as_ref(), 2 * 1024 * 1024)?)?;
    if capture.schema_version != 1 || capture.commands.len() != 12 {
        return Err(PromotionEvidenceError::Contract(
            "gate capture schema or command count is invalid",
        ));
    }
    adapter.validate()?;
    let commands = capture
        .commands
        .into_iter()
        .map(|command| {
            let stdout = read_bounded_stream(&command.stdout_path, 256 * 1024 * 1024)?;
            let stderr = read_bounded_stream(&command.stderr_path, 256 * 1024 * 1024)?;
            GateCommandReceipt::new(
                command.command_id,
                command.argv_utf8,
                command.started_monotonic_ns,
                command.ended_monotonic_ns,
                command.exit_code,
                &stdout,
                &stderr,
            )
        })
        .collect::<Result<Vec<_>, PromotionEvidenceError>>()?;
    let gate_script = read_bounded_stream(gate_script_path.as_ref(), 4 * 1024 * 1024)?;
    let receipt = GpuClosedLoopGateReceipt::new(
        GitObjectId::from_hex(&capture.git_commit)?,
        GitObjectId::from_hex(&capture.source_tree_digest)?,
        adapter,
        &gate_script,
        commands,
    )?;
    write_gpu_closed_loop_gate_receipt(output, &receipt)?;
    Ok(receipt)
}

pub fn load_gpu_closed_loop_gate_receipt(
    path: impl AsRef<Path>,
) -> Result<GpuClosedLoopGateReceipt, PromotionEvidenceError> {
    let bytes = read_bounded_json(path.as_ref(), 16 * 1024 * 1024)?;
    let receipt: GpuClosedLoopGateReceipt = serde_json::from_slice(&bytes)?;
    receipt.validate()?;
    Ok(receipt)
}

pub fn load_benchmark_promotion_bindings(
    path: impl AsRef<Path>,
) -> Result<(BenchmarkManifestBinding, Vec<BenchmarkRowBinding>), PromotionEvidenceError> {
    let bytes = read_bounded_json(path.as_ref(), 256 * 1024 * 1024)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    parse_benchmark_manifest_value(&value)
}

pub fn build_gpu_closed_loop_promotion_from_paths(
    paths: &PromotionArtifactPaths,
) -> Result<GpuClosedLoopPromotionManifest, PromotionEvidenceError> {
    validate_promotion_artifact_paths(paths)?;
    let (promotion_commit, source_tree_digest) = current_clean_git_identity()?;
    let gate_receipt = load_gpu_closed_loop_gate_receipt(&paths.gates)?;
    let gate = gate_receipt.binding()?;
    if gate.evidence_commit != promotion_commit || gate.source_tree != source_tree_digest {
        return Err(PromotionEvidenceError::Contract(
            "gate receipt does not bind the current clean Git identity",
        ));
    }
    let adapter = gate.adapter;
    let (benchmark, benchmark_rows) = load_benchmark_promotion_bindings(&paths.benchmark)?;
    let mut artifact_bindings = Vec::with_capacity(18);
    for path in &paths.slice_a {
        let receipt = crate::load_gpu_slice_a_evidence(path)?;
        artifact_bindings.push(binding_from_header(
            &receipt.header,
            hardware_adapter_binding(&receipt.hardware)?,
        )?);
    }
    for path in &paths.slice_b {
        let receipt = crate::load_gpu_slice_b_evidence(path)?;
        artifact_bindings.push(binding_from_header(
            &receipt.header,
            hardware_adapter_binding(&receipt.hardware)?,
        )?);
    }
    for path in &paths.slice_c {
        let receipt = crate::load_gpu_slice_c_evidence(path)?;
        artifact_bindings.push(binding_from_header(
            &receipt.header.common,
            hardware_adapter_binding(&receipt.hardware)?,
        )?);
    }
    for path in &paths.slice_d {
        let receipt = crate::load_gpu_slice_d_evidence(path)?;
        artifact_bindings.push(binding_from_header(
            &receipt.header.common,
            normalize_provenance_adapter(&receipt.adapter)?,
        )?);
    }

    let candidate_ancestors = artifact_bindings
        .iter()
        .map(|binding| binding.evidence_commit)
        .chain(std::iter::once(benchmark.evidence_commit))
        .filter(|commit| *commit != promotion_commit)
        .collect::<BTreeSet<_>>();
    let mut trusted_ancestor_commits = Vec::with_capacity(candidate_ancestors.len());
    for commit in candidate_ancestors.iter().copied() {
        if !git_commit_is_ancestor(commit, promotion_commit)? {
            return Err(PromotionEvidenceError::Contract(
                "evidence commit is not an ancestor of the promotion commit",
            ));
        }
        trusted_ancestor_commits.push(commit);
    }
    ingest_promotion_evidence(PromotionEvidenceInputs::new(
        promotion_commit,
        source_tree_digest,
        adapter,
        gate,
        benchmark,
        artifact_bindings,
        benchmark_rows,
        trusted_ancestor_commits,
    )?)
}

fn validate_promotion_artifact_paths(
    paths: &PromotionArtifactPaths,
) -> Result<(), PromotionEvidenceError> {
    if paths.slice_a.len() != 3
        || paths.slice_b.len() != 3
        || paths.slice_c.len() != 6
        || paths.slice_d.len() != 6
    {
        return Err(PromotionEvidenceError::Contract(
            "promotion requires exactly three Slice A, three Slice B, six Slice C, and six Slice D artifacts",
        ));
    }
    if paths.benchmark.as_os_str().is_empty() || paths.gates.as_os_str().is_empty() {
        return Err(PromotionEvidenceError::Contract(
            "promotion requires benchmark and gate artifact paths",
        ));
    }
    let mut unique = BTreeSet::new();
    for path in paths
        .slice_a
        .iter()
        .chain(&paths.slice_b)
        .chain(&paths.slice_c)
        .chain(&paths.slice_d)
        .chain([&paths.benchmark, &paths.gates])
    {
        if path.as_os_str().is_empty() || !unique.insert(path.clone()) {
            return Err(PromotionEvidenceError::Contract(
                "promotion artifact paths are empty or duplicated",
            ));
        }
    }
    Ok(())
}

fn binding_from_header(
    header: &crate::GpuSliceEvidenceHeader,
    adapter: EvidenceAdapterBinding,
) -> Result<EvidenceArtifactBinding, PromotionEvidenceError> {
    let binding = EvidenceArtifactBinding {
        slice_raw: header.slice_raw,
        class_id_raw: header.class_id_raw,
        profile_id_raw: header.profile_id_raw,
        profile_schema: header.profile_schema,
        artifact_schema: header.artifact_schema,
        evidence_commit: GitObjectId::from_hex(&header.git_commit)?,
        source_tree: GitObjectId::from_hex(&header.source_tree_digest)?,
        artifact_digest: header.artifact_digest,
        phenotype_hash: header.phenotype_hash,
        phenotype_manifest_digest: header.phenotype_manifest_digest,
        capacity_digest: header.capacity_digest,
        adapter,
        status_raw: header.status_raw,
    };
    binding.validate()?;
    Ok(binding)
}

fn hardware_adapter_binding(
    hardware: &GpuHardwareReceipt,
) -> Result<EvidenceAdapterBinding, PromotionEvidenceError> {
    GpuGateAdapterReceipt::from_hardware(hardware)?.binding()
}

fn current_clean_git_identity() -> Result<(GitObjectId, GitObjectId), PromotionEvidenceError> {
    if !git_output(&["status", "--porcelain=v1"])?.is_empty() {
        return Err(PromotionEvidenceError::Contract(
            "promotion requires a clean committed worktree",
        ));
    }
    Ok((
        GitObjectId::from_hex(git_text(&["rev-parse", "HEAD"])?.as_str())?,
        GitObjectId::from_hex(git_text(&["rev-parse", "HEAD^{tree}"])?.as_str())?,
    ))
}

fn git_commit_is_ancestor(
    ancestor: GitObjectId,
    descendant: GitObjectId,
) -> Result<bool, PromotionEvidenceError> {
    let status = Command::new("git")
        .args([
            "merge-base",
            "--is-ancestor",
            ancestor.to_hex().as_str(),
            descendant.to_hex().as_str(),
        ])
        .status()?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(PromotionEvidenceError::Contract(
            "Git ancestry validation failed",
        )),
    }
}

fn git_text(arguments: &[&str]) -> Result<String, PromotionEvidenceError> {
    let bytes = git_output(arguments)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| PromotionEvidenceError::Contract("Git output is not UTF-8"))?
        .trim();
    if value.is_empty() {
        return Err(PromotionEvidenceError::Contract("Git output is empty"));
    }
    Ok(value.to_string())
}

fn git_output(arguments: &[&str]) -> Result<Vec<u8>, PromotionEvidenceError> {
    let output = Command::new("git").args(arguments).output()?;
    if !output.status.success() {
        return Err(PromotionEvidenceError::Detail(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn parse_benchmark_manifest_value(
    value: &Value,
) -> Result<(BenchmarkManifestBinding, Vec<BenchmarkRowBinding>), PromotionEvidenceError> {
    let object = value.as_object().ok_or(PromotionEvidenceError::Contract(
        "benchmark manifest must be a JSON object",
    ))?;
    require_fields(
        object,
        &[
            "schema_version",
            "git_commit",
            "source_tree_digest",
            "adapter",
            "adapter_identity_digest_or_zero",
            "protocol",
            "rows",
            "manifest_digest",
        ],
    )?;
    let manifest_digest = digest4_field(value, "manifest_digest")?;
    if u16_field(value, "schema_version")? != 1
        || manifest_digest == [0; 4]
        || manifest_digest
            != canonical_json_struct_digest(
                b"alife.gpu.closed-loop-benchmark.manifest.v1",
                value,
                "manifest_digest",
            )?
    {
        return Err(PromotionEvidenceError::Contract(
            "benchmark manifest digest or schema is invalid",
        ));
    }
    let evidence_commit = GitObjectId::from_hex(string_field(value, "git_commit")?)?;
    let source_tree = GitObjectId::from_hex(string_field(value, "source_tree_digest")?)?;
    let adapter: GpuBackendProvenanceSave =
        serde_json::from_value(object.get("adapter").cloned().ok_or(
            PromotionEvidenceError::Contract("benchmark manifest has no adapter"),
        )?)?;
    adapter.validate().map_err(|error| {
        PromotionEvidenceError::Detail(format!("invalid benchmark adapter: {error}"))
    })?;
    let benchmark_identity = benchmark_adapter_identity_digest(&adapter)?;
    if digest4_field(value, "adapter_identity_digest_or_zero")? != benchmark_identity {
        return Err(PromotionEvidenceError::Contract(
            "benchmark adapter identity digest is invalid",
        ));
    }
    let normalized_adapter = normalize_provenance_adapter(&adapter)?;
    let protocol = object
        .get("protocol")
        .ok_or(PromotionEvidenceError::Contract(
            "benchmark protocol is missing",
        ))?;
    let protocol_digest = digest4_field(protocol, "protocol_digest")?;
    if u16_field(protocol, "schema_version")? != 1
        || u16_field(protocol, "protocol_version")? != 1
        || u32_field(protocol, "warmup_ticks")? != 256
        || u32_field(protocol, "measured_ticks")? != 1_024
        || u16_field(protocol, "samples_per_tick")? != 1
        || u16_field(protocol, "nearest_rank_percentile")? != 95
        || u16_field(protocol, "timestamp_scope_raw")? != 2
        || u64_field(protocol, "base_seed")? != 4_404
        || protocol_digest == [0; 4]
        || protocol_digest
            != canonical_json_struct_digest(
                b"alife.gpu.closed-loop-benchmark.protocol.v1",
                protocol,
                "protocol_digest",
            )?
    {
        return Err(PromotionEvidenceError::Contract(
            "benchmark protocol is not canonical v1",
        ));
    }
    let raw_rows =
        object
            .get("rows")
            .and_then(Value::as_array)
            .ok_or(PromotionEvidenceError::Contract(
                "benchmark rows are not an array",
            ))?;
    if raw_rows.len() != 36 {
        return Err(PromotionEvidenceError::Contract(
            "benchmark manifest must contain exactly 36 rows",
        ));
    }
    let mut keys = BTreeSet::new();
    let mut rows = Vec::with_capacity(raw_rows.len());
    for row in raw_rows {
        rows.push(parse_benchmark_row(
            row,
            protocol_digest,
            normalized_adapter,
            &adapter,
            &mut keys,
        )?);
    }
    rows.sort_unstable_by_key(|row| row.key());
    if !complete_global_benchmark_key_matrix(&rows) {
        return Err(PromotionEvidenceError::Contract(
            "benchmark manifest key matrix is incomplete",
        ));
    }
    let binding = BenchmarkManifestBinding {
        evidence_commit,
        source_tree,
        manifest_digest,
        protocol_digest,
        adapter: normalized_adapter,
        row_bindings_digest: BenchmarkManifestBinding::digest_rows(&rows),
    };
    binding.validate(&rows)?;
    Ok((binding, rows))
}

fn parse_benchmark_row(
    value: &Value,
    protocol_digest: [u64; 4],
    adapter: EvidenceAdapterBinding,
    provenance: &GpuBackendProvenanceSave,
    keys: &mut BTreeSet<(u16, u16, u32)>,
) -> Result<BenchmarkRowBinding, PromotionEvidenceError> {
    let object = value.as_object().ok_or(PromotionEvidenceError::Contract(
        "benchmark row must be a JSON object",
    ))?;
    require_fields(
        object,
        &[
            "schema_version",
            "class_id_raw",
            "sensor_profile_id_raw",
            "sensor_profile_schema",
            "sensory_abi_raw",
            "population",
            "fixture_seed",
            "phenotype_hash",
            "phenotype_manifest",
            "phenotype_manifest_digest",
            "capacity_digest",
            "runtime_profile_digest",
            "activity_policy_digest",
            "protocol_digest",
            "target_p95_ns",
            "measured_p95_ns",
            "timestamp_period_ns_q24",
            "raw_inference_timestamp_ticks",
            "raw_plasticity_timestamp_ticks",
            "raw_neural_tick_ns",
            "environment",
            "admission",
            "gpu_selections",
            "executed_actions",
            "sealed_patches",
            "learning_commits",
            "distinct_selected_families",
            "active_synapses",
            "status",
            "row_digest",
        ],
    )?;
    let class_id_raw = u16_field(value, "class_id_raw")?;
    let profile_id_raw = u16_field(value, "sensor_profile_id_raw")?;
    let profile_schema = u16_field(value, "sensor_profile_schema")?;
    let population = u32_field(value, "population")?;
    let capacity = production_capacity(class_id_raw)?;
    SensorProfile::try_from_raw(profile_id_raw)?;
    let row_digest = digest4_field(value, "row_digest")?;
    let phenotype_hash: PhenotypeHash =
        serde_json::from_value(object.get("phenotype_hash").cloned().ok_or(
            PromotionEvidenceError::Contract("benchmark phenotype hash is missing"),
        )?)?;
    let phenotype_manifest =
        object
            .get("phenotype_manifest")
            .ok_or(PromotionEvidenceError::Contract(
                "benchmark phenotype manifest is missing",
            ))?;
    let phenotype_manifest_digest = digest4_field(value, "phenotype_manifest_digest")?;
    let capacity_digest = digest4_field(value, "capacity_digest")?;
    let status_raw = benchmark_status_raw(object.get("status").ok_or(
        PromotionEvidenceError::Contract("benchmark status is missing"),
    )?)?;
    if u16_field(value, "schema_version")? != 1
        || profile_schema != PROFILE_SCHEMA_VERSION
        || u16_field(value, "sensory_abi_raw")? == 0
        || !EXPECTED_POPULATIONS.contains(&population)
        || digest4_field(value, "protocol_digest")? != protocol_digest
        || row_digest == [0; 4]
        || row_digest
            != canonical_json_struct_digest(
                b"alife.gpu.closed-loop-benchmark.row.v1",
                value,
                "row_digest",
            )?
        || phenotype_hash
            != serde_json::from_value(phenotype_manifest.get("phenotype_hash").cloned().ok_or(
                PromotionEvidenceError::Contract("benchmark manifest phenotype hash is missing"),
            )?)?
        || phenotype_manifest_digest != digest4_field(phenotype_manifest, "manifest_digest")?
        || capacity_digest != digest4_field(phenotype_manifest, "capacity_digest")?
        || capacity_digest != capacity.canonical_digest()
        || u16_field(phenotype_manifest, "phenotype_sensor_profile_raw")? != profile_id_raw
    {
        return Err(PromotionEvidenceError::Contract(
            "benchmark row identity or digest is invalid",
        ));
    }
    let key = (class_id_raw, profile_id_raw, population);
    if !keys.insert(key) {
        return Err(PromotionEvidenceError::Contract(
            "benchmark row key is duplicated",
        ));
    }
    validate_benchmark_environment(
        object
            .get("environment")
            .ok_or(PromotionEvidenceError::Contract(
                "benchmark environment is missing",
            ))?,
        provenance,
        status_raw,
    )?;
    let binding = BenchmarkRowBinding {
        class_id_raw,
        profile_id_raw,
        profile_schema,
        population,
        status_raw,
        row_digest,
        phenotype_hash,
        phenotype_manifest_digest,
        capacity_digest,
        protocol_digest,
        adapter,
    };
    binding.validate()?;
    Ok(binding)
}

fn validate_benchmark_environment(
    value: &Value,
    manifest_adapter: &GpuBackendProvenanceSave,
    status_raw: u16,
) -> Result<(), PromotionEvidenceError> {
    let object = value.as_object().ok_or(PromotionEvidenceError::Contract(
        "benchmark environment must be an object",
    ))?;
    require_fields(
        object,
        &[
            "schema_version",
            "availability_reason_code",
            "adapter",
            "adapter_identity_digest_or_zero",
            "environment_digest",
        ],
    )?;
    if u16_field(value, "schema_version")? != 1
        || digest4_field(value, "environment_digest")?
            != canonical_json_struct_digest(
                b"alife.gpu.closed-loop-benchmark.environment.v1",
                value,
                "environment_digest",
            )?
    {
        return Err(PromotionEvidenceError::Contract(
            "benchmark environment digest is invalid",
        ));
    }
    match object.get("adapter") {
        Some(Value::Null) if status_raw == 3 => Ok(()),
        Some(adapter_value) => {
            let adapter: GpuBackendProvenanceSave = serde_json::from_value(adapter_value.clone())?;
            adapter.validate().map_err(|error| {
                PromotionEvidenceError::Detail(format!("invalid row adapter: {error}"))
            })?;
            if adapter != *manifest_adapter
                || digest4_field(value, "adapter_identity_digest_or_zero")?
                    != benchmark_adapter_identity_digest(&adapter)?
            {
                return Err(PromotionEvidenceError::Contract(
                    "benchmark row adapter differs from its manifest",
                ));
            }
            Ok(())
        }
        _ => Err(PromotionEvidenceError::Contract(
            "executed benchmark row has no adapter",
        )),
    }
}

fn trusted_commit(inputs: &PromotionEvidenceInputs, commit: GitObjectId) -> bool {
    commit == inputs.promotion_commit || inputs.trusted_ancestor_commits.contains(&commit)
}

fn artifact_gate_bit(binding: EvidenceArtifactBinding) -> Result<u64, PromotionEvidenceError> {
    match (binding.slice_raw, binding.profile_id_raw) {
        (1, 0) => Ok(GATE_SLICE_A),
        (2, 0) => Ok(GATE_SLICE_B),
        (3, profile) if profile == SensorProfile::PrivilegedAffordanceV1.raw() => {
            Ok(GATE_SLICE_C_PRIVILEGED)
        }
        (3, profile) if profile == SensorProfile::GroundedObjectSlotsV1.raw() => {
            Ok(GATE_SLICE_C_GROUNDED)
        }
        (4, profile) if profile == SensorProfile::PrivilegedAffordanceV1.raw() => {
            Ok(GATE_SLICE_D_PRIVILEGED)
        }
        (4, profile) if profile == SensorProfile::GroundedObjectSlotsV1.raw() => {
            Ok(GATE_SLICE_D_GROUNDED)
        }
        _ => Err(PromotionEvidenceError::Contract(
            "slice artifact does not map to a required gate",
        )),
    }
}

fn complete_passing_benchmark_matrix(rows: &[BenchmarkRowBinding]) -> bool {
    if rows.len() != 12 || rows.iter().any(|row| row.status_raw != PASSING_STATUS_RAW) {
        return false;
    }
    for profile in [
        SensorProfile::PrivilegedAffordanceV1.raw(),
        SensorProfile::GroundedObjectSlotsV1.raw(),
    ] {
        for population in EXPECTED_POPULATIONS {
            if !rows
                .iter()
                .any(|row| row.profile_id_raw == profile && row.population == population)
            {
                return false;
            }
        }
    }
    true
}

fn production_capacity(class_id_raw: u16) -> Result<BrainCapacityClass, PromotionEvidenceError> {
    Ok(BrainCapacityClass::production_for_id(BrainClassId(
        class_id_raw,
    ))?)
}

fn complete_global_benchmark_key_matrix(rows: &[BenchmarkRowBinding]) -> bool {
    if rows.len() != production_class_ids().len() * 2 * EXPECTED_POPULATIONS.len() {
        return false;
    }
    production_class_ids().into_iter().all(|class_id| {
        [
            SensorProfile::PrivilegedAffordanceV1.raw(),
            SensorProfile::GroundedObjectSlotsV1.raw(),
        ]
        .into_iter()
        .all(|profile_id_raw| {
            EXPECTED_POPULATIONS.into_iter().all(|population| {
                rows.iter().any(|row| {
                    row.class_id_raw == class_id.raw()
                        && row.profile_id_raw == profile_id_raw
                        && row.population == population
                })
            })
        })
    })
}

fn benchmark_status_raw(value: &Value) -> Result<u16, PromotionEvidenceError> {
    match value {
        Value::String(status) if status == "completed" => Ok(PASSING_STATUS_RAW),
        Value::String(status) if status == "missed" => Ok(2),
        Value::Object(status) => {
            require_fields(status, &["unavailable"])?;
            let unavailable = status.get("unavailable").and_then(Value::as_object).ok_or(
                PromotionEvidenceError::Contract("benchmark unavailable status is malformed"),
            )?;
            require_fields(unavailable, &["reason_code"])?;
            let reason_code = unavailable
                .get("reason_code")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(PromotionEvidenceError::Contract(
                    "benchmark unavailable reason is invalid",
                ))?;
            if reason_code == 0 {
                return Err(PromotionEvidenceError::Contract(
                    "benchmark unavailable reason is zero",
                ));
            }
            Ok(3)
        }
        _ => Err(PromotionEvidenceError::Contract(
            "benchmark status is invalid",
        )),
    }
}

fn normalize_provenance_adapter(
    adapter: &GpuBackendProvenanceSave,
) -> Result<EvidenceAdapterBinding, PromotionEvidenceError> {
    adapter.validate().map_err(|error| {
        PromotionEvidenceError::Detail(format!("invalid benchmark adapter: {error}"))
    })?;
    EvidenceAdapterBinding::new(
        adapter.vendor_id,
        adapter.device_id,
        adapter.backend_api_raw,
        adapter.driver_digest,
        adapter.available_features_digest,
        adapter.adapter_limits_digest,
    )
}

fn benchmark_adapter_identity_digest(
    adapter: &GpuBackendProvenanceSave,
) -> Result<[u64; 4], PromotionEvidenceError> {
    adapter.validate().map_err(|error| {
        PromotionEvidenceError::Detail(format!("invalid benchmark adapter: {error}"))
    })?;
    let mut digest = CanonicalDigestBuilder::new(b"alife.gpu.closed-loop-benchmark.adapter.v1");
    digest.write_u16(adapter.schema_version);
    digest.write_u16(adapter.backend_api_raw);
    digest.write_u32(adapter.vendor_id);
    digest.write_u32(adapter.device_id);
    digest.write_u16(adapter.backend_version_major);
    digest.write_u16(adapter.backend_version_minor);
    digest.write_u16(adapter.backend_version_patch);
    for words in [
        adapter.driver_digest,
        adapter.available_features_digest,
        adapter.adapter_limits_digest,
    ] {
        write_digest4(&mut digest, words);
    }
    Ok(digest.finish256())
}

fn require_fields(
    object: &serde_json::Map<String, Value>,
    expected: &[&str],
) -> Result<(), PromotionEvidenceError> {
    if object.len() != expected.len() || expected.iter().any(|field| !object.contains_key(*field)) {
        return Err(PromotionEvidenceError::Contract(
            "evidence object fields do not match its schema",
        ));
    }
    Ok(())
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, PromotionEvidenceError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(PromotionEvidenceError::Contract(
            "evidence string field is missing or invalid",
        ))
}

fn u16_field(value: &Value, field: &str) -> Result<u16, PromotionEvidenceError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(PromotionEvidenceError::Contract(
            "evidence u16 field is missing or invalid",
        ))
}

fn u32_field(value: &Value, field: &str) -> Result<u32, PromotionEvidenceError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(PromotionEvidenceError::Contract(
            "evidence u32 field is missing or invalid",
        ))
}

fn u64_field(value: &Value, field: &str) -> Result<u64, PromotionEvidenceError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or(PromotionEvidenceError::Contract(
            "evidence u64 field is missing or invalid",
        ))
}

fn digest4_field(value: &Value, field: &str) -> Result<[u64; 4], PromotionEvidenceError> {
    let words =
        value
            .get(field)
            .and_then(Value::as_array)
            .ok_or(PromotionEvidenceError::Contract(
                "evidence digest field is missing or invalid",
            ))?;
    if words.len() != 4 {
        return Err(PromotionEvidenceError::Contract(
            "evidence digest must contain four words",
        ));
    }
    let mut digest = [0_u64; 4];
    for (index, word) in words.iter().enumerate() {
        digest[index] = word.as_u64().ok_or(PromotionEvidenceError::Contract(
            "evidence digest word is invalid",
        ))?;
    }
    Ok(digest)
}

fn canonical_json_struct_digest(
    domain: &[u8],
    value: &Value,
    excluded_field: &str,
) -> Result<[u64; 4], PromotionEvidenceError> {
    let mut value = value.clone();
    let object = value
        .as_object_mut()
        .ok_or(PromotionEvidenceError::Contract(
            "canonical digest requires an object",
        ))?;
    object
        .remove(excluded_field)
        .ok_or(PromotionEvidenceError::Contract(
            "canonical digest exclusion field is missing",
        ))?;
    let mut digest = CanonicalDigestBuilder::new(domain);
    encode_json_value(&mut digest, &value)?;
    Ok(digest.finish256())
}

fn encode_json_value(
    digest: &mut CanonicalDigestBuilder,
    value: &Value,
) -> Result<(), PromotionEvidenceError> {
    match value {
        Value::Null => digest.write_u8(0),
        Value::Bool(value) => {
            digest.write_u8(1);
            digest.write_bool(*value);
        }
        Value::Number(value) => {
            digest.write_u8(2);
            if let Some(value) = value.as_u64() {
                digest.write_u8(0);
                digest.write_u64(value);
            } else if let Some(value) = value.as_i64() {
                digest.write_u8(1);
                digest.write_u64(value as u64);
            } else {
                return Err(PromotionEvidenceError::Contract(
                    "canonical evidence cannot contain JSON floats",
                ));
            }
        }
        Value::String(value) => {
            digest.write_u8(3);
            digest.write_utf8(value);
        }
        Value::Array(values) => {
            digest.write_u8(4);
            digest.write_sequence_len(values.len());
            for value in values {
                encode_json_value(digest, value)?;
            }
        }
        Value::Object(values) => {
            digest.write_u8(5);
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            digest.write_sequence_len(entries.len());
            for (key, value) in entries {
                digest.write_utf8(key);
                encode_json_value(digest, value)?;
            }
        }
    }
    Ok(())
}

fn production_class_ids() -> [BrainClassId; 3] {
    [
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ]
}

fn hex_nibble(byte: u8) -> Result<u8, PromotionEvidenceError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(PromotionEvidenceError::Contract(
            "Git object ID contains non-hex data",
        )),
    }
}

fn any_zero(digests: impl IntoIterator<Item = [u64; 4]>) -> bool {
    digests.into_iter().any(|digest| digest == [0; 4])
}

fn captured_stream_digest(domain: &[u8], command_id: u16, bytes: &[u8]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_u16(command_id);
    digest.write_bytes(bytes);
    digest.finish256()
}

fn commands_digest(commands: &[GateCommandReceipt]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.gpu.promotion.gate-commands.v1");
    digest.write_sequence_len(commands.len());
    for command in commands {
        digest.write_u16(command.command_id);
        digest.write_bytes(&command.argv_utf8);
        digest.write_u64(command.started_monotonic_ns);
        digest.write_u64(command.ended_monotonic_ns);
        digest.write_u64(command.exit_code as i64 as u64);
        write_digest4(&mut digest, command.stdout_digest);
        write_digest4(&mut digest, command.stderr_digest);
    }
    digest.finish256()
}

fn encode_gate_adapter_receipt(
    digest: &mut CanonicalDigestBuilder,
    adapter: GpuGateAdapterReceipt,
) {
    digest.write_u32(adapter.vendor_id);
    digest.write_u32(adapter.device_id);
    digest.write_u16(adapter.backend_api_raw);
    digest.write_u16(adapter.adapter_name_len);
    digest.write_bytes(&adapter.adapter_name_utf8);
    write_digest4(digest, adapter.driver_digest);
    write_digest4(digest, adapter.feature_digest);
    write_digest4(digest, adapter.limits_digest);
    write_digest4(digest, adapter.identity_digest);
}

fn read_bounded_json(path: &Path, maximum: u64) -> Result<Vec<u8>, PromotionEvidenceError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > maximum {
        return Err(PromotionEvidenceError::Contract(
            "promotion evidence file size is outside its bound",
        ));
    }
    Ok(fs::read(path)?)
}

fn read_bounded_stream(path: &Path, maximum: u64) -> Result<Vec<u8>, PromotionEvidenceError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > maximum {
        return Err(PromotionEvidenceError::Contract(
            "captured gate stream exceeds its byte bound",
        ));
    }
    Ok(fs::read(path)?)
}

fn join_argv(arguments: &[&str]) -> Vec<u8> {
    let capacity = arguments
        .iter()
        .map(|argument| argument.len())
        .sum::<usize>()
        + arguments.len().saturating_sub(1);
    let mut bytes = Vec::with_capacity(capacity);
    for (index, argument) in arguments.iter().enumerate() {
        if index != 0 {
            bytes.push(0);
        }
        bytes.extend_from_slice(argument.as_bytes());
    }
    bytes
}

fn atomic_write_validated_json<T: Serialize>(
    path: &Path,
    value: &T,
    validate_temporary: impl FnOnce(&Path) -> Result<(), PromotionEvidenceError>,
) -> Result<(), PromotionEvidenceError> {
    let parent = path.parent().ok_or(PromotionEvidenceError::Contract(
        "promotion output has no parent directory",
    ))?;
    fs::create_dir_all(parent)?;
    let filename = path.file_name().and_then(|value| value.to_str()).ok_or(
        PromotionEvidenceError::Contract("promotion output filename is not UTF-8"),
    )?;
    let temporary = parent.join(format!(".{filename}.{}.staging", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    let result = (|| -> Result<(), PromotionEvidenceError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        validate_temporary(&temporary)?;
        atomic_replace(&temporary, path)?;
        if fs::read(path)? != bytes {
            return Err(PromotionEvidenceError::Contract(
                "atomic promotion publication changed bytes",
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both buffers are live, NUL-terminated UTF-16 paths for the call.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

mod fixed_bytes_128 {
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &[u8; 128], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 128], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .try_into()
            .map_err(|_| D::Error::custom("expected exactly 128 adapter-name bytes"))
    }
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, value: [u64; 4]) {
    for word in value {
        digest.write_u64(word);
    }
}

fn write_oid(digest: &mut CanonicalDigestBuilder, value: GitObjectId) {
    digest.write_bytes(&value.0);
}

fn encode_adapter(digest: &mut CanonicalDigestBuilder, value: EvidenceAdapterBinding) {
    digest.write_u32(value.vendor_id);
    digest.write_u32(value.device_id);
    digest.write_u16(value.backend_api_raw);
    write_digest4(digest, value.driver_digest);
    write_digest4(digest, value.feature_digest);
    write_digest4(digest, value.limits_digest);
    write_digest4(digest, value.identity_digest);
}

fn encode_artifact(digest: &mut CanonicalDigestBuilder, value: EvidenceArtifactBinding) {
    let mut scoped = CanonicalDigestBuilder::new(ARTIFACT_BINDING_DOMAIN);
    scoped.write_u16(value.slice_raw);
    scoped.write_u16(value.class_id_raw);
    scoped.write_u16(value.profile_id_raw);
    scoped.write_u16(value.profile_schema);
    scoped.write_u16(value.artifact_schema);
    write_oid(&mut scoped, value.evidence_commit);
    write_oid(&mut scoped, value.source_tree);
    write_digest4(&mut scoped, value.artifact_digest);
    write_digest4(&mut scoped, value.phenotype_hash.0);
    write_digest4(&mut scoped, value.phenotype_manifest_digest);
    write_digest4(&mut scoped, value.capacity_digest);
    encode_adapter(&mut scoped, value.adapter);
    scoped.write_u16(value.status_raw);
    write_digest4(digest, scoped.finish256());
}

fn encode_benchmark_row(digest: &mut CanonicalDigestBuilder, value: BenchmarkRowBinding) {
    let mut scoped = CanonicalDigestBuilder::new(BENCHMARK_ROW_DOMAIN);
    scoped.write_u16(value.class_id_raw);
    scoped.write_u16(value.profile_id_raw);
    scoped.write_u16(value.profile_schema);
    scoped.write_u32(value.population);
    scoped.write_u16(value.status_raw);
    write_digest4(&mut scoped, value.row_digest);
    write_digest4(&mut scoped, value.phenotype_hash.0);
    write_digest4(&mut scoped, value.phenotype_manifest_digest);
    write_digest4(&mut scoped, value.capacity_digest);
    write_digest4(&mut scoped, value.protocol_digest);
    encode_adapter(&mut scoped, value.adapter);
    write_digest4(digest, scoped.finish256());
}

fn encode_benchmark_manifest(digest: &mut CanonicalDigestBuilder, value: BenchmarkManifestBinding) {
    write_oid(digest, value.evidence_commit);
    write_oid(digest, value.source_tree);
    write_digest4(digest, value.manifest_digest);
    write_digest4(digest, value.protocol_digest);
    encode_adapter(digest, value.adapter);
    write_digest4(digest, value.row_bindings_digest);
}

fn encode_gate(digest: &mut CanonicalDigestBuilder, value: GateEvidenceBinding) {
    write_oid(digest, value.evidence_commit);
    write_oid(digest, value.source_tree);
    write_digest4(digest, value.receipt_digest);
    write_digest4(digest, value.gate_script_digest);
    write_digest4(digest, value.commands_digest);
    encode_adapter(digest, value.adapter);
}

#[cfg(test)]
mod tests {
    use alife_world::persistence::{
        GpuBackendProvenanceSave, NeuralGpuBackendApi, GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
    };
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn benchmark_manifest_parser_requires_complete_digest_bound_matrix() {
        let mut value = benchmark_fixture();
        let (manifest, rows) = parse_benchmark_manifest_value(&value).unwrap();
        assert_eq!(rows.len(), 36);
        assert_eq!(
            manifest.row_bindings_digest,
            BenchmarkManifestBinding::digest_rows(&rows)
        );

        value["rows"][0]["population"] = json!(2);
        assert!(parse_benchmark_manifest_value(&value).is_err());
    }

    #[test]
    fn persisted_promotion_rejects_resealed_benchmark_row_removal() {
        let (benchmark, benchmark_rows) =
            parse_benchmark_manifest_value(&benchmark_fixture()).unwrap();
        let adapter = benchmark.adapter;
        let promotion_commit = GitObjectId([0x22; 20]);
        let source_tree = GitObjectId([0x33; 20]);
        let gate = GateEvidenceBinding {
            evidence_commit: promotion_commit,
            source_tree,
            receipt_digest: [31, 32, 33, 34],
            gate_script_digest: [35, 36, 37, 38],
            commands_digest: [39, 40, 41, 42],
            adapter,
        };
        let mut artifact_bindings = Vec::new();
        for capacity in [
            BrainCapacityClass::n512(),
            BrainCapacityClass::n1024(),
            BrainCapacityClass::n2048(),
        ] {
            for (slice_raw, profiles) in [
                (1, vec![(0, 0)]),
                (2, vec![(0, 0)]),
                (
                    3,
                    vec![
                        (SensorProfile::PrivilegedAffordanceV1.raw(), 1),
                        (SensorProfile::GroundedObjectSlotsV1.raw(), 1),
                    ],
                ),
                (
                    4,
                    vec![
                        (SensorProfile::PrivilegedAffordanceV1.raw(), 1),
                        (SensorProfile::GroundedObjectSlotsV1.raw(), 1),
                    ],
                ),
            ] {
                for (profile_id_raw, profile_schema) in profiles {
                    artifact_bindings.push(EvidenceArtifactBinding {
                        slice_raw,
                        class_id_raw: capacity.id().raw(),
                        profile_id_raw,
                        profile_schema,
                        artifact_schema: 1,
                        evidence_commit: promotion_commit,
                        source_tree,
                        artifact_digest: [51, 52, 53, 54],
                        phenotype_hash: PhenotypeHash([61, 62, 63, 64]),
                        phenotype_manifest_digest: [71, 72, 73, 74],
                        capacity_digest: capacity.canonical_digest(),
                        adapter,
                        status_raw: PASSING_STATUS_RAW,
                    });
                }
            }
        }
        let inputs = PromotionEvidenceInputs::new(
            promotion_commit,
            source_tree,
            adapter,
            gate,
            benchmark,
            artifact_bindings,
            benchmark_rows,
            Vec::new(),
        )
        .unwrap();
        let mut manifest = ingest_promotion_evidence(inputs).unwrap();

        manifest.rows[0].benchmark_rows.pop();
        manifest.rows[0].row_digest = manifest.rows[0].recompute_digest();
        manifest.manifest_digest = manifest.recompute_digest();

        assert!(manifest.validate().is_err());
    }

    fn benchmark_fixture() -> Value {
        let adapter = test_adapter();
        let adapter_value = serde_json::to_value(&adapter).unwrap();
        let adapter_identity = benchmark_adapter_identity_digest(&adapter).unwrap();
        let mut protocol = json!({
            "schema_version": 1,
            "protocol_version": 1,
            "warmup_ticks": 256,
            "measured_ticks": 1024,
            "samples_per_tick": 1,
            "nearest_rank_percentile": 95,
            "timestamp_scope_raw": 2,
            "base_seed": 4404,
            "protocol_digest": [0, 0, 0, 0],
        });
        let protocol_digest = canonical_json_struct_digest(
            b"alife.gpu.closed-loop-benchmark.protocol.v1",
            &protocol,
            "protocol_digest",
        )
        .unwrap();
        protocol["protocol_digest"] = serde_json::to_value(protocol_digest).unwrap();

        let mut rows = Vec::new();
        for capacity in [
            BrainCapacityClass::n512(),
            BrainCapacityClass::n1024(),
            BrainCapacityClass::n2048(),
        ] {
            for profile in [
                SensorProfile::PrivilegedAffordanceV1,
                SensorProfile::GroundedObjectSlotsV1,
            ] {
                for population in EXPECTED_POPULATIONS {
                    let phenotype_hash = [
                        u64::from(capacity.id().raw()),
                        u64::from(profile.raw()),
                        u64::from(population),
                        1,
                    ];
                    let phenotype_manifest_digest = [7, 8, 9, 10];
                    let capacity_digest = capacity.canonical_digest();
                    let mut environment = json!({
                        "schema_version": 1,
                        "availability_reason_code": 0,
                        "adapter": adapter_value.clone(),
                        "adapter_identity_digest_or_zero": adapter_identity,
                        "environment_digest": [0, 0, 0, 0],
                    });
                    let environment_digest = canonical_json_struct_digest(
                        b"alife.gpu.closed-loop-benchmark.environment.v1",
                        &environment,
                        "environment_digest",
                    )
                    .unwrap();
                    environment["environment_digest"] =
                        serde_json::to_value(environment_digest).unwrap();
                    let mut row = json!({
                        "schema_version": 1,
                        "class_id_raw": capacity.id().raw(),
                        "sensor_profile_id_raw": profile.raw(),
                        "sensor_profile_schema": 1,
                        "sensory_abi_raw": 1,
                        "population": population,
                        "fixture_seed": 4404_u64 ^ (u64::from(capacity.id().raw()) << 48) ^ (u64::from(profile.raw()) << 32) ^ u64::from(population),
                        "phenotype_hash": phenotype_hash,
                        "phenotype_manifest": {
                            "phenotype_hash": phenotype_hash,
                            "manifest_digest": phenotype_manifest_digest,
                            "capacity_digest": capacity_digest,
                            "phenotype_sensor_profile_raw": profile.raw(),
                        },
                        "phenotype_manifest_digest": phenotype_manifest_digest,
                        "capacity_digest": capacity_digest,
                        "runtime_profile_digest": [11, 12, 13, 14],
                        "activity_policy_digest": [15, 16, 17, 18],
                        "protocol_digest": protocol_digest,
                        "target_p95_ns": 1_000_000,
                        "measured_p95_ns": 500_000,
                        "timestamp_period_ns_q24": 1,
                        "raw_inference_timestamp_ticks": [1],
                        "raw_plasticity_timestamp_ticks": [1],
                        "raw_neural_tick_ns": [2],
                        "environment": environment,
                        "admission": null,
                        "gpu_selections": 1,
                        "executed_actions": 1,
                        "sealed_patches": 1,
                        "learning_commits": 1,
                        "distinct_selected_families": 2,
                        "active_synapses": 1,
                        "status": "completed",
                        "row_digest": [0, 0, 0, 0],
                    });
                    let row_digest = canonical_json_struct_digest(
                        b"alife.gpu.closed-loop-benchmark.row.v1",
                        &row,
                        "row_digest",
                    )
                    .unwrap();
                    row["row_digest"] = serde_json::to_value(row_digest).unwrap();
                    rows.push(row);
                }
            }
        }
        rows.sort_by_key(|row| {
            (
                row["class_id_raw"].as_u64().unwrap(),
                row["sensor_profile_id_raw"].as_u64().unwrap(),
                row["population"].as_u64().unwrap(),
            )
        });
        let mut manifest = json!({
            "schema_version": 1,
            "git_commit": "22".repeat(20),
            "source_tree_digest": "33".repeat(20),
            "adapter": adapter_value,
            "adapter_identity_digest_or_zero": adapter_identity,
            "protocol": protocol,
            "rows": rows,
            "manifest_digest": [0, 0, 0, 0],
        });
        let digest = canonical_json_struct_digest(
            b"alife.gpu.closed-loop-benchmark.manifest.v1",
            &manifest,
            "manifest_digest",
        )
        .unwrap();
        manifest["manifest_digest"] = serde_json::to_value(digest).unwrap();
        manifest
    }

    fn test_adapter() -> GpuBackendProvenanceSave {
        let mut adapter = GpuBackendProvenanceSave {
            schema_version: GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
            backend_api_raw: NeuralGpuBackendApi::Vulkan.raw(),
            vendor_id: 0x10de,
            device_id: 0x25a2,
            backend_version_major: 1,
            backend_version_minor: 0,
            backend_version_patch: 0,
            adapter_name_len: 0,
            adapter_name_utf8: [0; 128],
            driver_digest: [10, 11, 12, 13],
            required_features_digest: [14, 15, 16, 17],
            required_limits_digest: [18, 19, 20, 21],
            available_features_digest: [22, 23, 24, 25],
            adapter_limits_digest: [26, 27, 28, 29],
        };
        adapter.set_adapter_name("test-adapter").unwrap();
        adapter
    }
}
