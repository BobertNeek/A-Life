//! Frozen replay of bounded semantic-prior outputs captured at the real provider boundary.
//!
//! A bank indexes only the exact request context. There is no scenario, time,
//! expected outcome, inference worker, retry, or nearest-context fallback here.
//! Provenance is a compatibility declaration, not proof that a model produced
//! the entries; synthetic fixtures must remain identified as such in evidence.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use alife_core::BASIC_VOCABULARY_V1;
use serde::{de::MapAccess, de::Visitor, Deserialize, Deserializer, Serialize};

use crate::{LocalSlmPriorOutput, LocalSlmPriorRequest, SlmLexiconAssociation};

pub const RECORDED_PRIOR_BANK_SCHEMA: &str = "alife.recorded_semantic_prior_bank.v1";
pub const RECORDED_PRIOR_BANK_SCHEMA_VERSION: u16 = 1;
pub const MAX_RECORDED_PRIOR_BANK_BYTES: usize = 256 * 1024;
pub const MAX_RECORDED_PRIOR_BANK_ENTRIES: usize = 128;
const MAX_PROVENANCE_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedPriorOrigin {
    /// Identity of the source provider configuration, not the recorded backend.
    pub provider_identity: String,
    pub model: String,
    /// Exact model SHA256, or an explicit declaration that it was not verified.
    pub model_sha256: String,
    /// Current bounded-context builder, request schema/prompt, and vocabulary
    /// fingerprint. The runtime must supply its own expected contract.
    pub context_contract: String,
}

impl RecordedPriorOrigin {
    /// Whether provenance declares a well-formed digest. This does not verify
    /// model bytes or recording authenticity.
    pub fn has_model_sha256(&self) -> bool {
        is_sha256(&self.model_sha256)
    }

    fn validate(&self, expected_context_contract: &str) -> Result<(), String> {
        if [&self.provider_identity, &self.model, &self.context_contract]
            .iter()
            .any(|value| {
                value.trim().is_empty()
                    || value.len() > MAX_PROVENANCE_BYTES
                    || value.chars().any(char::is_control)
            })
        {
            return Err("recorded prior origin has missing or unbounded provenance".into());
        }
        if !self.has_model_sha256() && self.model_sha256 != "unverified-model" {
            return Err("recorded prior origin requires a model SHA256 or unverified-model".into());
        }
        if self.context_contract != expected_context_contract {
            return Err("recorded prior context contract does not match this runtime".into());
        }
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Immutable, validated recording bank. Loading never instantiates a provider.
/// Misses are `None`, including when all entries are valid zero-salience hints.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedPriorBank {
    schema: String,
    schema_version: u16,
    origin: RecordedPriorOrigin,
    entries: BTreeMap<String, LocalSlmPriorOutput>,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct CacheDocument(
    #[serde(deserialize_with = "deserialize_outputs")] BTreeMap<String, LocalSlmPriorOutput>,
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BankDocument {
    schema: String,
    schema_version: u16,
    origin: RecordedPriorOrigin,
    #[serde(deserialize_with = "deserialize_outputs")]
    entries: BTreeMap<String, LocalSlmPriorOutput>,
}

// The live output type is shared with older callers. The bank's stricter wire
// type rejects unknown/duplicate fields instead of silently dropping commands
// or hidden scenario information during deserialization.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedOutput {
    schema: String,
    schema_version: u16,
    model: String,
    salience_labels: Vec<String>,
    context_summary: String,
    lexicon_associations: Vec<RecordedAssociation>,
    perception_tags: Vec<String>,
    can_issue_actions: bool,
    can_rewrite_weights: bool,
    can_bypass_arbitration: bool,
    hidden_vector_injection: bool,
    bounded_context_only: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedAssociation {
    token: String,
    salience: f32,
}

impl From<RecordedOutput> for LocalSlmPriorOutput {
    fn from(output: RecordedOutput) -> Self {
        Self {
            schema: output.schema,
            schema_version: output.schema_version,
            model: output.model,
            salience_labels: output.salience_labels,
            context_summary: output.context_summary,
            lexicon_associations: output
                .lexicon_associations
                .into_iter()
                .map(|association| SlmLexiconAssociation {
                    token: association.token,
                    salience: association.salience,
                })
                .collect(),
            perception_tags: output.perception_tags,
            can_issue_actions: output.can_issue_actions,
            can_rewrite_weights: output.can_rewrite_weights,
            can_bypass_arbitration: output.can_bypass_arbitration,
            hidden_vector_injection: output.hidden_vector_injection,
            bounded_context_only: output.bounded_context_only,
        }
    }
}

fn deserialize_outputs<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, LocalSlmPriorOutput>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OutputsVisitor;
    impl<'de> Visitor<'de> for OutputsVisitor {
        type Value = BTreeMap<String, LocalSlmPriorOutput>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("at most 128 unique exact-context recorded outputs")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut entries = BTreeMap::new();
            while let Some(context) = map.next_key::<String>()? {
                if entries.len() >= MAX_RECORDED_PRIOR_BANK_ENTRIES {
                    return Err(serde::de::Error::custom("too many recorded prior entries"));
                }
                if entries.contains_key(&context) {
                    return Err(serde::de::Error::custom("duplicate recorded prior context"));
                }
                let output = map.next_value::<RecordedOutput>()?;
                entries.insert(context, output.into());
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(OutputsVisitor)
}

impl RecordedPriorBank {
    pub fn load(path: impl AsRef<Path>, expected_context_contract: &str) -> Result<Self, String> {
        let file = File::open(path.as_ref())
            .map_err(|error| format!("cannot open recorded prior bank: {error}"))?;
        let mut bytes = Vec::new();
        file.take((MAX_RECORDED_PRIOR_BANK_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read recorded prior bank: {error}"))?;
        Self::from_json_slice(&bytes, expected_context_contract)
    }

    pub fn from_json_slice(bytes: &[u8], expected_context_contract: &str) -> Result<Self, String> {
        if bytes.len() > MAX_RECORDED_PRIOR_BANK_BYTES {
            return Err("recorded prior bank exceeds 256 KiB".into());
        }
        let document: BankDocument = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid recorded prior bank JSON: {error}"))?;
        if document.schema != RECORDED_PRIOR_BANK_SCHEMA
            || document.schema_version != RECORDED_PRIOR_BANK_SCHEMA_VERSION
        {
            return Err("unsupported recorded prior bank schema/version".into());
        }
        Self::from_cache(document.origin, document.entries, expected_context_contract)
    }

    /// Parse a legacy exact-context cache with the same strict output and
    /// duplicate-key checks as a recording bank. Its source provenance must be
    /// supplied from the original capture receipt, never inferred or fabricated.
    pub fn from_cache_json_slice(
        origin: RecordedPriorOrigin,
        bytes: &[u8],
        expected_context_contract: &str,
    ) -> Result<Self, String> {
        if bytes.len() > MAX_RECORDED_PRIOR_BANK_BYTES {
            return Err("recorded prior source cache exceeds 256 KiB".into());
        }
        let cache: CacheDocument = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid recorded prior source cache JSON: {error}"))?;
        Self::from_cache(origin, cache.0, expected_context_contract)
    }

    /// Export preparation only: wrap outputs already captured from the provider
    /// boundary. This function does not synthesize or obtain model responses.
    pub fn from_cache(
        origin: RecordedPriorOrigin,
        entries: BTreeMap<String, LocalSlmPriorOutput>,
        expected_context_contract: &str,
    ) -> Result<Self, String> {
        origin.validate(expected_context_contract)?;
        if entries.len() > MAX_RECORDED_PRIOR_BANK_ENTRIES {
            return Err("recorded prior bank exceeds 128 entries".into());
        }
        for (context, output) in &entries {
            LocalSlmPriorRequest {
                request_id: 1,
                prompt: context.clone(),
            }
            .validate(crate::local_slm_prior::CA27_MAX_PROMPT_CHARS)
            .map_err(|error| format!("invalid recorded prior context: {error:?}"))?;
            output
                .validate()
                .map_err(|error| format!("invalid recorded prior output: {error:?}"))?;
            if output.model != origin.model {
                return Err("recorded prior output model differs from bank origin".into());
            }
            if output.lexicon_associations.iter().any(|association| {
                association.salience > 0.0
                    && !BASIC_VOCABULARY_V1
                        .iter()
                        .any(|(word, _)| *word == association.token)
            }) {
                return Err(
                    "recorded prior positive association outside receiver vocabulary".into(),
                );
            }
        }
        let bank = Self {
            schema: RECORDED_PRIOR_BANK_SCHEMA.into(),
            schema_version: RECORDED_PRIOR_BANK_SCHEMA_VERSION,
            origin,
            entries,
        };
        bank.to_json_vec()?;
        Ok(bank)
    }

    pub fn origin(&self) -> &RecordedPriorOrigin {
        &self.origin
    }

    pub fn entries(&self) -> &BTreeMap<String, LocalSlmPriorOutput> {
        &self.entries
    }

    pub fn lookup(&self, exact_context: &str) -> Option<&LocalSlmPriorOutput> {
        self.entries.get(exact_context)
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| format!("cannot serialize recorded prior bank: {error}"))?;
        if bytes.len() > MAX_RECORDED_PRIOR_BANK_BYTES {
            return Err("recorded prior bank exceeds 256 KiB".into());
        }
        Ok(bytes)
    }

    /// Explicit export only. Refuses to replace an existing file, including the
    /// frozen bank used by a running creature. Runtime replay calls only `load`.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let bytes = self.to_json_vec()?;
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(path.as_ref())
            .map_err(|error| format!("cannot create recorded prior export: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("cannot write recorded prior export: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CA27_SLM_PRIOR_OUTPUT_SCHEMA, CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION};
    use std::sync::atomic::{AtomicU64, Ordering};

    const CONTEXT_CONTRACT: &str =
        "synthetic-request-v1:synthetic-context-v1:synthetic-vocabulary-v1";
    const CONTEXT: &str =
        "heard words food; hunger high; tiredness low; smells chemical levels 0.0,0.0,0.0";

    fn origin() -> RecordedPriorOrigin {
        RecordedPriorOrigin {
            provider_identity: "synthetic-cpu-fixture-provider".into(),
            model: "synthetic-fixture-model".into(),
            model_sha256: "unverified-model".into(),
            context_contract: CONTEXT_CONTRACT.into(),
        }
    }

    fn output() -> LocalSlmPriorOutput {
        LocalSlmPriorOutput {
            schema: CA27_SLM_PRIOR_OUTPUT_SCHEMA.into(),
            schema_version: CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
            model: origin().model,
            salience_labels: vec!["food".into()],
            context_summary: "heard food".into(),
            lexicon_associations: vec![SlmLexiconAssociation {
                token: "food".into(),
                salience: 0.5,
            }],
            perception_tags: vec!["heard".into()],
            can_issue_actions: false,
            can_rewrite_weights: false,
            can_bypass_arbitration: false,
            hidden_vector_injection: false,
            bounded_context_only: true,
        }
    }

    fn bank_with(output: LocalSlmPriorOutput) -> RecordedPriorBank {
        RecordedPriorBank::from_cache(
            origin(),
            BTreeMap::from([(CONTEXT.into(), output)]),
            CONTEXT_CONTRACT,
        )
        .unwrap()
    }

    fn temporary_path() -> std::path::PathBuf {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "alife-recorded-prior-{}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn recorded_bank_roundtrips_source_provenance_and_exact_output() {
        let bank = bank_with(output());
        let path = temporary_path();
        bank.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let loaded = RecordedPriorBank::load(&path, CONTEXT_CONTRACT).unwrap();
        assert_eq!(loaded.origin(), bank.origin());
        assert!(!loaded.origin().has_model_sha256());
        assert_eq!(loaded.lookup(CONTEXT), Some(&output()));
        assert_eq!(loaded.entries(), bank.entries());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(loaded.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn exact_context_misses_never_select_or_mutate_another_recording() {
        let bank = bank_with(output());
        let before = bank.to_json_vec().unwrap();
        for miss in [
            "",
            "food",
            "scenario_one",
            "tick 123",
            &CONTEXT.to_uppercase(),
            &format!("{CONTEXT} "),
        ] {
            assert!(bank.lookup(miss).is_none());
        }
        assert_eq!(bank.lookup(CONTEXT), Some(&output()));
        assert_eq!(bank.to_json_vec().unwrap(), before);
    }

    #[test]
    fn zero_salience_and_empty_banks_remain_honest_without_inference() {
        let mut zero = output();
        zero.lexicon_associations[0].salience = 0.0;
        zero.lexicon_associations[0].token = "novelword".into();
        let bank = bank_with(zero.clone());
        let loaded =
            RecordedPriorBank::from_json_slice(&bank.to_json_vec().unwrap(), CONTEXT_CONTRACT)
                .unwrap();
        for _ in 0..4 {
            assert_eq!(loaded.lookup(CONTEXT), Some(&zero));
            assert!(loaded.lookup("new sensed context").is_none());
        }
        let empty =
            RecordedPriorBank::from_cache(origin(), BTreeMap::new(), CONTEXT_CONTRACT).unwrap();
        assert!(empty.entries().is_empty());
        assert!(empty.lookup(CONTEXT).is_none());
    }

    #[test]
    fn incompatible_contract_model_or_schema_is_rejected() {
        let bank = bank_with(output());
        let bytes = bank.to_json_vec().unwrap();
        assert!(RecordedPriorBank::from_json_slice(&bytes, "changed-contract").is_err());
        for (field, value) in [
            ("schema", serde_json::json!("different-bank-schema")),
            ("schema_version", serde_json::json!(2)),
        ] {
            let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            document[field] = value;
            assert!(RecordedPriorBank::from_json_slice(
                &serde_json::to_vec(&document).unwrap(),
                CONTEXT_CONTRACT
            )
            .is_err());
        }
        let mut changed = output();
        changed.model = "other-provider-model".into();
        assert!(RecordedPriorBank::from_cache(
            origin(),
            BTreeMap::from([(CONTEXT.into(), changed)]),
            CONTEXT_CONTRACT
        )
        .is_err());
        let mut origin = origin();
        origin.model_sha256 = "not-a-digest".into();
        assert!(RecordedPriorBank::from_cache(origin, BTreeMap::new(), CONTEXT_CONTRACT).is_err());
    }

    #[test]
    fn bounded_bank_rejects_oversize_context_entries_and_file() {
        assert!(RecordedPriorBank::from_json_slice(
            &vec![b' '; MAX_RECORDED_PRIOR_BANK_BYTES + 1],
            CONTEXT_CONTRACT
        )
        .is_err());
        assert!(RecordedPriorBank::from_cache(
            origin(),
            BTreeMap::from([("a".repeat(769), output())]),
            CONTEXT_CONTRACT
        )
        .is_err());
        let entries = (0..129)
            .map(|index| {
                (
                    format!("heard words food; feels contact pressure {index}"),
                    output(),
                )
            })
            .collect();
        assert!(RecordedPriorBank::from_cache(origin(), entries, CONTEXT_CONTRACT).is_err());
        let path = temporary_path();
        std::fs::write(&path, vec![b' '; MAX_RECORDED_PRIOR_BANK_BYTES + 1]).unwrap();
        assert!(RecordedPriorBank::load(&path, CONTEXT_CONTRACT).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn bank_rejects_authority_unknown_positive_tokens_and_hidden_fields() {
        let mut authoritative = output();
        authoritative.can_issue_actions = true;
        assert!(RecordedPriorBank::from_cache(
            origin(),
            BTreeMap::from([(CONTEXT.into(), authoritative)]),
            CONTEXT_CONTRACT
        )
        .is_err());
        let mut unknown = output();
        unknown.lexicon_associations[0].token = "not-in-receiver-vocabulary".into();
        assert!(RecordedPriorBank::from_cache(
            origin(),
            BTreeMap::from([(CONTEXT.into(), unknown)]),
            CONTEXT_CONTRACT
        )
        .is_err());
        let bytes = bank_with(output()).to_json_vec().unwrap();
        for (level, field) in [
            ("bank", "scenario"),
            ("origin", "expected_outcome"),
            ("output", "action"),
            ("association", "reward"),
        ] {
            let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let target = match level {
                "bank" => &mut document,
                "origin" => &mut document["origin"],
                "output" => &mut document["entries"][CONTEXT],
                _ => &mut document["entries"][CONTEXT]["lexicon_associations"][0],
            };
            target[field] = serde_json::json!("scripted hidden data");
            assert!(RecordedPriorBank::from_json_slice(
                &serde_json::to_vec(&document).unwrap(),
                CONTEXT_CONTRACT
            )
            .is_err());
        }
    }

    #[test]
    fn duplicate_context_or_output_fields_are_rejected() {
        let bytes = bank_with(output()).to_json_vec().unwrap();
        let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        document["entries"] = serde_json::json!({});
        let prefix = serde_json::to_string(&document).unwrap();
        let key = serde_json::to_string(CONTEXT).unwrap();
        let output = serde_json::to_string(&output()).unwrap();
        let duplicates = format!("{key}:{output},{key}:{output}");
        let document = prefix.replace("\"entries\":{}", &format!("\"entries\":{{{duplicates}}}"));
        assert!(RecordedPriorBank::from_json_slice(document.as_bytes(), CONTEXT_CONTRACT).is_err());
        let document = String::from_utf8(bytes).unwrap().replace(
            "\"can_issue_actions\":false",
            "\"can_issue_actions\":true,\"can_issue_actions\":false",
        );
        assert!(RecordedPriorBank::from_json_slice(document.as_bytes(), CONTEXT_CONTRACT).is_err());
    }

    #[test]
    fn declared_sha256_is_retained_without_claiming_recording_authenticity() {
        let mut origin = origin();
        origin.model_sha256 = "a".repeat(64);
        let bank = RecordedPriorBank::from_cache(origin.clone(), BTreeMap::new(), CONTEXT_CONTRACT)
            .unwrap();
        assert!(bank.origin().has_model_sha256());
        assert_eq!(bank.origin(), &origin);
    }
}
