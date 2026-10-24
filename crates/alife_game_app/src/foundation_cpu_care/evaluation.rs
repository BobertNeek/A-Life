//! Teacher/prior-removed assessment of an explicitly selected native CPU source.
//! Ordinary private learning and biology continue; offline founder/critic updates,
//! learner shards, accepted-source selection and promotion are never performed.
//! The caller exclusively owns its output; concurrent assessment dispatch is not
//! supported. An interrupted private life is retained and never auto-restarted.
use super::*;
use crate::foundation_training_cycle::admit_native_cpu_care_assessment_seed;
use alife_training::{resolve_isolated_training_output, FoundationTrainingHandoffV1};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

pub const CPU_CARE_EVALUATION_FILE: &str = "care-evaluation.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCareEvaluationRequest {
    pub previous: PathBuf,
    pub output: PathBuf,
    pub seed_history: PathBuf,
    pub seed: u64,
    pub decisions: usize,
    pub temperature: f32,
    pub lesson: FoundationTeacherLesson,
    #[serde(default)]
    pub known_exclusions: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationReceipt {
    schema_version: u16,
    request: CpuCareEvaluationRequest,
    input_sha256: BTreeMap<PathBuf, String>,
    artifacts: BTreeMap<String, String>,
    result: Value,
    payload_sha256: [u8; 32],
}
impl EvaluationReceipt {
    fn digest(&self) -> Result<[u8; 32]> {
        let mut hash = Sha256::new();
        hash.update(b"ALIFE-CPU-CARE-EVALUATION-V1");
        serde_json::to_writer(
            &mut hash,
            &(
                self.schema_version,
                &self.request,
                &self.input_sha256,
                &self.artifacts,
                &self.result,
            ),
        )?;
        Ok(hash.finalize().into())
    }
}

pub(super) fn ensure_no_teacher(world: &alife_world::HeadlessWorld) -> Result<()> {
    if world.entity_id("nursery-teacher").is_some()
        || world
            .audible_utterances()
            .iter()
            .any(|utterance| utterance.source_kind == alife_core::UtteranceSourceKind::Teacher)
    {
        return Err("assessment contains a teacher actor or utterance".into());
    }
    Ok(())
}
pub(super) fn teacher_heard(frame: &alife_core::PerceptionFrame) -> bool {
    frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|token| token.source_kind == alife_core::UtteranceSourceKind::Teacher)
}
fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_value(value: &impl Serialize) -> Result<String> {
    let mut hash = Sha256::new();
    serde_json::to_writer(&mut hash, value)?;
    Ok(format!("{:x}", hash.finalize()))
}
fn file_hash(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("assessment artifacts must be regular nonsymlink files".into());
    }
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    if io::copy(&mut file, &mut hash)? != metadata.len() {
        return Err("assessment artifact changed while reading".into());
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    if fs::symlink_metadata(path)?.len() > 64 * 1024 * 1024 {
        return Err("assessment JSON exceeds64 MiB".into());
    }
    file_hash(path)?;
    Ok(serde_json::from_reader(fs::File::open(path)?)?)
}
fn inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(
        root: &Path,
        dir: &Path,
        result: &mut BTreeMap<String, String>,
        depth: usize,
    ) -> Result<()> {
        if depth > 128 || result.len() > 65_536 {
            return Err("assessment archive exceeds its bounded inventory".into());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err("assessment archives cannot contain symlinks".into());
            }
            if kind.is_dir() {
                visit(root, &path, result, depth + 1)?;
            } else if kind.is_file() {
                let key = path
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or("non-UTF8 assessment artifact")?
                    .replace('\\', "/");
                if key != CPU_CARE_EVALUATION_FILE {
                    result.insert(key, file_hash(&path)?);
                }
            } else {
                return Err("nonregular assessment artifact".into());
            }
        }
        Ok(())
    }
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err("assessment output cannot be a symlink".into());
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result, 0)?;
    Ok(result)
}
fn input_pins(
    request: &CpuCareEvaluationRequest,
    files: &BTreeMap<String, Vec<u8>>,
    provenance: &Value,
) -> Result<BTreeMap<PathBuf, String>> {
    let mut pins = files
        .iter()
        .map(|(name, bytes)| (request.previous.join(name), hash_bytes(bytes)))
        .collect::<BTreeMap<_, _>>();
    pins.insert(
        request.seed_history.clone(),
        file_hash(&request.seed_history)?,
    );
    for evidence in provenance["evidence"]
        .as_array()
        .ok_or("assessment lacks pinned seed evidence")?
    {
        let path = PathBuf::from(
            evidence["path"]
                .as_str()
                .ok_or("assessment evidence lacks path")?,
        );
        let expected = evidence["sha256"]
            .as_str()
            .ok_or("assessment evidence lacks digest")?;
        if file_hash(&path)? != expected {
            return Err("assessment seed evidence changed".into());
        }
        pins.insert(path, expected.into());
    }
    Ok(pins)
}
fn verify_pins(pins: &BTreeMap<PathBuf, String>) -> Result<()> {
    for (path, expected) in pins {
        if file_hash(path)? != *expected {
            return Err("assessment source/history changed".into());
        }
    }
    Ok(())
}
fn metrics(
    request: &CpuCareEvaluationRequest,
    source: &FoundationTrainingHandoffV1,
    pins: &BTreeMap<PathBuf, String>,
    episode: &Value,
    actor_after_hash: &str,
    value_after_hash: &str,
) -> Result<Value> {
    let actor_hash = hash_value(&source.actor)?;
    let value_hash = hash_value(&source.value)?;
    let source_pins = pins
        .iter()
        .filter_map(|(path, hash)| {
            path.strip_prefix(&request.previous)
                .ok()
                .map(|name| (name.to_string_lossy().replace('\\', "/"), hash.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    Ok(
        json!({"mode":"native-cpu-care-assessment","backend":"native-cpu",
        "result":request.output.join(CPU_CARE_EVALUATION_FILE),"lesson":request.lesson,"seed":request.seed,
        "decisions":request.decisions,"evaluated_decisions":request.decisions,"bootstrap_decisions":1,
        "source_policy_version":source.policy_version,"policy_version":source.policy_version,
        "actor_optimizer_step":source.actor.optimizer_step,"value_optimizer_step":source.value.optimizer_step,
        "source_sha256":source_pins,"input_sha256":pins,"lesson_completed":episode["lesson_completed"],
        "evaluated_nourished_meals":episode["evaluated_nourished_meals"],
        "ordinary_acquisition_events":episode["evaluated_ordinary_acquisitions"],
        "consumption_events":episode["evaluated_consumptions"],
        "navigation_completed":if request.lesson == FoundationTeacherLesson::ObstacleNavigation {episode["lesson_completed"].clone()} else {Value::Null},
        "teacher_present":false,"teacher_cue_frames":0,"semantic_prior_enabled":false,
        "actor_checkpoint_unchanged":true,"value_checkpoint_unchanged":true,
        "actor_checkpoint_before_sha256":actor_hash,"actor_checkpoint_after_sha256":actor_after_hash,
        "value_checkpoint_before_sha256":value_hash,"value_checkpoint_after_sha256":value_after_hash,
        "offline_actor_optimizer_updates":0,"offline_value_optimizer_updates":0,
        "offline_learning_disabled":true,"private_waking_learning":"ordinary","promotion":"none",
        "trace":request.output.join("care-trace.json"),"private_checkpoint":request.output.join("collector-checkpoint.json")}),
    )
}
fn verify_recovery(
    request: &CpuCareEvaluationRequest,
    source: &FoundationTrainingHandoffV1,
) -> Result<(Value, String, String)> {
    let actor: alife_training::FoundationTrainerCheckpoint =
        read_json(&request.output.join("collection-actor-checkpoint.json"))?;
    let value: alife_training::PpoValueHeadCheckpoint =
        read_json(&request.output.join("initial-value-checkpoint.json"))?;
    let actor_hash = hash_value(&actor)?;
    let value_hash = hash_value(&value)?;
    if actor_hash != hash_value(&source.actor)? || value_hash != hash_value(&source.value)? {
        return Err("assessment changed full actor/critic state".into());
    }
    let components = source.clone().admit()?;
    let founder =
        CpuFoundationTrainer::from_checkpoint(components.phenotype, components.foundation, actor)?;
    let checkpoint: CpuCanonicalCheckpoint =
        read_json(&request.output.join("collector-checkpoint.json"))?;
    let host = CpuCanonicalHost::from_checkpoint(checkpoint, &founder)?;
    ensure_no_teacher(host.world())?;
    let prior: Value = read_json(&request.output.join("care-prior-lives.json"))?;
    if !prior["provider_identity"].is_null()
        || prior["lives"]
            .as_object()
            .is_none_or(|lives| !lives.is_empty())
        || prior["world_tick"] != json!(host.world().tick())
    {
        return Err(
            "assessment retained teacher/prior state or mismatched private recovery".into(),
        );
    }
    let episode: Value = read_json(&request.output.join("care-episode.json"))?;
    let trace: Value = read_json(&request.output.join("care-trace.json"))?;
    let rows = trace["rows"]
        .as_array()
        .ok_or("assessment lacks real outcome trace")?;
    if rows.len() != request.decisions + 1
        || episode["semantic_prior_enabled"] != false
        || episode["teacher_cue_frames"] != 0
        || episode["evaluated_decisions"] != request.decisions
        || rows.iter().enumerate().any(|(i, row)| {
            row["loss_row"] != false
                || row["teacher_heard"] != false
                || row["evaluation_row"] != (i < request.decisions)
        })
    {
        return Err("assessment dose/teacher/prior/loss boundary is invalid".into());
    }
    for name in [
        "collector-shard.json",
        "collector-commit.json",
        "cpu-cycle.json",
        "trained.alife-foundation",
        "training-handoff.json",
    ] {
        if request.output.join(name).exists() {
            return Err("assessment cannot be a reusable learner/trained source".into());
        }
    }
    Ok((episode, actor_hash, value_hash))
}

/// Evaluate one explicitly admitted holdout using a fresh private body/prior
/// lifetime namespace. A completed exact result is read-only reusable; any other
/// existing output is preserved and refused, never resumed or overwritten.
pub fn evaluate_cpu_care(mut request: CpuCareEvaluationRequest) -> Result<Value> {
    request.previous = request.previous.canonicalize()?;
    request.seed_history = request.seed_history.canonicalize()?;
    let source = read_cpu_founder_source(&request.previous)?;
    request.output = resolve_isolated_training_output(
        &request.output,
        [&request.previous, &request.seed_history].map(PathBuf::as_path),
    )?;
    let (mut config, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: Some(request.previous.clone()),
        output: request.output.clone(),
        seed: request.seed,
        decisions: request.decisions,
        temperature: request.temperature,
        lesson: request.lesson,
        seed_history: Some(request.seed_history.clone()),
        known_exclusions: request.known_exclusions,
        record_diagnostic_replay: false,
        fixture: false,
    })?;
    let provenance = admit_native_cpu_care_assessment_seed(
        &request.previous,
        &request.seed_history,
        &request.output,
        request.seed,
        request.known_exclusions,
    )?;
    let pins = input_pins(&request, &source.files, &provenance)?;
    request.output = resolve_isolated_training_output(
        &request.output,
        std::iter::once(request.previous.as_path()).chain(pins.keys().map(PathBuf::as_path)),
    )?;
    config.output = request.output.clone();
    if request.output.exists() {
        let receipt: EvaluationReceipt = read_json(&request.output.join(CPU_CARE_EVALUATION_FILE))
            .map_err(|error| format!("existing assessment is unsealed or damaged; preserve its private life: {error}"))?;
        if receipt.schema_version != 1
            || receipt.request != request
            || receipt.input_sha256 != pins
            || receipt.payload_sha256 != receipt.digest()?
            || receipt.artifacts != inventory(&request.output)?
        {
            return Err(
                "assessment request/input/archive changed; existing life is not restarted".into(),
            );
        }
        let (episode, actor_hash, value_hash) = verify_recovery(&request, &source.handoff)?;
        if receipt.result
            != metrics(
                &request,
                &source.handoff,
                &pins,
                &episode,
                &actor_hash,
                &value_hash,
            )?
        {
            return Err("assessment result differs from its exact native recovery evidence".into());
        }
        verify_pins(&pins)?;
        return Ok(receipt.result);
    }
    episode.assessment = true;
    let collected = collect_cpu_episode(config, &mut episode)?;
    let actor_after_hash = hash_value(&collected.actor_checkpoint())?;
    let value_after_hash = hash_value(&collected.value_checkpoint())?;
    if actor_after_hash != hash_value(&source.handoff.actor)?
        || value_after_hash != hash_value(&source.handoff.value)?
    {
        return Err("assessment mutated full offline actor/critic checkpoints".into());
    }
    let (episode, recovery_actor_hash, recovery_value_hash) =
        verify_recovery(&request, &source.handoff)?;
    if recovery_actor_hash != actor_after_hash || recovery_value_hash != value_after_hash {
        return Err("assessment private recovery differs from full frozen live checkpoints".into());
    }
    let result = metrics(
        &request,
        &source.handoff,
        &pins,
        &episode,
        &actor_after_hash,
        &value_after_hash,
    )?;
    verify_pins(&pins)?;
    let final_provenance = admit_native_cpu_care_assessment_seed(
        &request.previous,
        &request.seed_history,
        &request.output,
        request.seed,
        request.known_exclusions,
    )?;
    if provenance != final_provenance {
        return Err("assessment admission changed during collection".into());
    }
    let mut receipt = EvaluationReceipt {
        schema_version: 1,
        request: request.clone(),
        input_sha256: pins,
        artifacts: inventory(&request.output)?,
        result: result.clone(),
        payload_sha256: [0; 32],
    };
    receipt.payload_sha256 = receipt.digest()?;
    admission::write_atomic(
        &request.output.join(CPU_CARE_EVALUATION_FILE),
        &serde_json::to_value(&receipt)?,
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assessment_removes_teacher_and_prior_but_preserves_real_private_outcomes_and_full_optimizers(
    ) {
        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let fixture = Fixture(std::env::temp_dir().join(
            format!(
            "alife-care-assessment-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
        ));
        fs::create_dir(&fixture.0).unwrap();
        let source = fixture.0.join("explicit-synthetic-source");
        let (config, mut episode) = prepare_care_episode(CpuCareRequest {
            previous: None,
            output: source.clone(),
            seed: 0x93_51,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            seed_history: None,
            known_exclusions: false,
            record_diagnostic_replay: false,
            fixture: true,
        })
        .unwrap();
        episode.explicit_deterministic_prior = true;
        run_cpu_continuation(config, &mut episode).unwrap();
        let founder_source = read_cpu_founder_source(&source).unwrap();
        assert!(founder_source.handoff.actor.optimizer_step > 0);
        assert!(founder_source
            .handoff
            .actor
            .first_moment
            .iter()
            .any(|moment| *moment != 0.));
        let source_before = inventory(&source).unwrap();
        let history = fixture.0.join("assessment-history.json");
        let source_pins = [
            "cpu-cycle.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .into_iter()
        .map(|name| (name, file_hash(&source.join(name)).unwrap()))
        .collect::<BTreeMap<_, _>>();
        fs::write(&history, serde_json::to_vec(&json!({
            "schema":1,"kind":"mixed-care-seed-history","complete":true,
            "evidence":"Explicit disposable CPU fixture only; no retained lineage or historical freshness is qualified",
            "source_sha256":source_pins,"consumed_seed_ranges":[[0x93_51,0x93_51]],
            "reserved_seed_ranges":[],"assessment_seeds":[0x93_52],"imports":[]
        })).unwrap()).unwrap();
        let history_before = fs::read(&history).unwrap();
        let request = CpuCareEvaluationRequest {
            previous: source.clone(),
            output: fixture.0.join("assessment"),
            seed_history: history.clone(),
            seed: 0x93_52,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            known_exclusions: false,
        };
        let result = evaluate_cpu_care(request.clone()).unwrap();
        assert_eq!(result["teacher_present"], false);
        assert_eq!(result["teacher_cue_frames"], 0);
        assert_eq!(result["semantic_prior_enabled"], false);
        assert_eq!(result["offline_actor_optimizer_updates"], 0);
        assert_eq!(result["offline_value_optimizer_updates"], 0);
        assert_eq!(
            result["policy_version"],
            founder_source.handoff.policy_version
        );
        let actor: alife_training::FoundationTrainerCheckpoint =
            read_json(&request.output.join("collection-actor-checkpoint.json")).unwrap();
        let value: alife_training::PpoValueHeadCheckpoint =
            read_json(&request.output.join("initial-value-checkpoint.json")).unwrap();
        assert_eq!(actor, founder_source.handoff.actor);
        assert_eq!(value, founder_source.handoff.value);
        let components = founder_source.handoff.clone().admit().unwrap();
        let founder = CpuFoundationTrainer::from_checkpoint(
            components.phenotype,
            components.foundation,
            actor,
        )
        .unwrap();
        let checkpoint: CpuCanonicalCheckpoint =
            read_json(&request.output.join("collector-checkpoint.json")).unwrap();
        assert!(checkpoint.residents.iter().any(|resident| resident
            .neural
            .eligibility
            .iter()
            .any(|trace| *trace != 0.)));
        let host = CpuCanonicalHost::from_checkpoint(checkpoint, &founder).unwrap();
        ensure_no_teacher(host.world()).unwrap();
        assert!(host.world().entity_id("nursery-teacher").is_none());
        let prior: Value = read_json(&request.output.join("care-prior-lives.json")).unwrap();
        assert!(prior["provider_identity"].is_null());
        assert_eq!(prior["lives"], json!({}));
        let traces: Value = read_json(&request.output.join("care-trace.json")).unwrap();
        let rows = traces["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row["evaluation_row"], i < 2);
            assert_eq!(row["loss_row"], false);
            assert_eq!(row["teacher_heard"], false);
            let outcome: alife_core::PostActionOutcome =
                serde_json::from_value(row["outcome"].clone()).unwrap();
            assert!(outcome.measured_physiology.is_some());
            assert!(row["lesson_target_ownership_before_decision"].is_object());
            assert!(row["lesson_target_ownership_after_outcome"].is_object());
        }
        assert_eq!(
            result["evaluated_nourished_meals"],
            rows[..2]
                .iter()
                .filter(|row| row["nourished_meal"] == true)
                .count()
        );
        let completed = inventory(&request.output).unwrap();
        let receipt = fs::read(request.output.join(CPU_CARE_EVALUATION_FILE)).unwrap();
        assert_eq!(evaluate_cpu_care(request.clone()).unwrap(), result);
        assert_eq!(inventory(&request.output).unwrap(), completed);
        assert_eq!(
            fs::read(request.output.join(CPU_CARE_EVALUATION_FILE)).unwrap(),
            receipt
        );
        assert_eq!(inventory(&source).unwrap(), source_before);
        assert_eq!(fs::read(&history).unwrap(), history_before);
        assert!(read_cpu_collector_output(&request.output).is_err());
        fs::rename(
            request.output.join(CPU_CARE_EVALUATION_FILE),
            fixture.0.join("preserved-completed-evaluation.json"),
        )
        .unwrap();
        assert!(evaluate_cpu_care(request.clone()).is_err());
        assert_eq!(inventory(&request.output).unwrap(), completed);
        assert_eq!(
            fs::read(fixture.0.join("preserved-completed-evaluation.json")).unwrap(),
            receipt
        );
    }
}
