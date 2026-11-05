//! Native load-only admission. No experiment CLI, world preparation, replay,
//! critic creation, policy rebind, or input/output checkpoint writer is exposed.
use alife_core::{
    load_only, BrainCapacityClass, FoundationWeightAsset, PhenotypeCompiler,
    PhenotypeCompilerInputs,
};
use alife_training::{
    cpu::{CpuCanonicalCheckpoint, CpuCanonicalHost, CpuFoundationTrainer},
    FoundationTrainerCheckpoint,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const MAX_JSON: u64 = 64 * 1024 * 1024;

#[derive(Debug, PartialEq)]
struct Request {
    manifest: PathBuf,
    expected_sha256: String,
    profile: String,
    require_joint: bool,
}
fn parse(args: impl Iterator<Item = String>) -> Result<Request> {
    let mut values = BTreeMap::new();
    let mut joint = false;
    let mut args = args;
    while let Some(flag) = args.next() {
        match flag.as_str(){
            "--require-joint-critic"|"--require-ppo"=>joint=true,
            "--manifest"|"--manifest-sha256"|"--profile"=>{
                let value=args.next().ok_or("missing option value")?;
                if values.insert(flag,value).is_some(){return Err("duplicate option".into());}
            }
            _=>return Err("only --manifest, --manifest-sha256, --profile and explicit joint/PPO refusal flags are supported".into()),
        }
    }
    let expected = values
        .remove("--manifest-sha256")
        .ok_or("--manifest-sha256 is required to pin the owner manifest")?;
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("expected manifest SHA256 must be64 lowercase hex digits".into());
    }
    let profile = values
        .remove("--profile")
        .unwrap_or_else(|| "actor_history".into());
    if !["original_snapshot", "four_arm_snapshots", "actor_history"].contains(&profile.as_str()) {
        return Err("only actor/private recovery profiles are supported; evidence scoring is a separate metadata tool".into());
    }
    Ok(Request {
        manifest: values
            .remove("--manifest")
            .ok_or("--manifest is required")?
            .into(),
        expected_sha256: expected,
        profile,
        require_joint: joint,
    })
}
#[derive(Clone, Deserialize)]
struct FileSpec {
    root: String,
    path: PathBuf,
    bytes: u64,
    sha256: String,
    requires: Vec<String>,
    native_typed_identity_sha256: Option<String>,
}
#[derive(Deserialize)]
struct Profile {
    files: Vec<String>,
}
#[derive(Deserialize)]
struct Original {
    recovery_file: String,
    checksum_file: String,
    native_actor_identity_sha256: String,
    native_private_identity_sha256: String,
}
#[derive(Deserialize)]
struct Update {
    before: String,
    after: String,
}
#[derive(Deserialize)]
struct ActorSpec {
    actor_file: String,
    updates: Vec<Update>,
}
#[derive(Deserialize)]
struct Critic {
    present: bool,
}
#[derive(Deserialize)]
struct Manifest {
    schema: u16,
    roots: BTreeMap<String, PathBuf>,
    files: BTreeMap<String, FileSpec>,
    profiles: BTreeMap<String, Profile>,
    original: Original,
    actors: BTreeMap<String, ActorSpec>,
    latest_private_end_files: BTreeMap<String, String>,
    known_positive_private_start: String,
    critic: Critic,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recovery {
    kind: String,
    inputs: PhenotypeCompilerInputs,
    foundation: Vec<u8>,
    actor: FoundationTrainerCheckpoint,
    host: CpuCanonicalCheckpoint,
    next_gradient: serde_json::Value,
    assessment: Option<serde_json::Value>,
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity(v: &impl Serialize) -> Result<String> {
    Ok(sha(&serde_json::to_vec(v)?))
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut de = serde_json::Deserializer::from_slice(bytes);
    de.disable_recursion_limit();
    let value = T::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}
fn selected<'a>(m: &'a Manifest, profile: &str) -> Result<BTreeSet<&'a str>> {
    if m.schema != 1 || m.critic.present {
        return Err("unsupported manifest: matching critic/version state unavailable; actor-only schema cannot certify joint/PPO continuation".into());
    }
    let ids = m
        .profiles
        .get(profile)
        .ok_or("missing profile")?
        .files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for id in &ids {
        let f = m.files.get(*id).ok_or("unknown selected file")?;
        if f.requires.iter().any(|d| !ids.contains(d.as_str())) {
            return Err("incomplete selected dependency closure".into());
        }
    }
    if !ids.contains(m.original.recovery_file.as_str())
        || !ids.contains(m.original.checksum_file.as_str())
    {
        return Err("missing original compiler/asset or checksum dependency".into());
    }
    Ok(ids)
}
fn resolve(m: &Manifest, s: &FileSpec) -> Result<PathBuf> {
    if s.path.as_os_str().is_empty()
        || s.path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("artifact path must be a nonempty relative path without traversal".into());
    }
    let root = m.roots.get(&s.root).ok_or("unknown root")?;
    let canonical_root = root.canonicalize()?;
    let p = root.join(&s.path).canonicalize()?;
    if !p.starts_with(&canonical_root) || !p.is_file() {
        return Err("artifact escaped its declared root or is not a file".into());
    }
    Ok(p)
}
fn verify_bytes(spec: &FileSpec, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != spec.bytes || sha(bytes) != spec.sha256 {
        return Err("input size/SHA256 mismatch".into());
    }
    Ok(())
}
fn json_bytes(spec: &FileSpec, bytes: &[u8]) -> Result<Vec<u8>> {
    if spec.path.extension().is_some_and(|e| e == "zst") {
        let mut raw = vec![];
        zstd::stream::read::Decoder::new(bytes)?
            .take(MAX_JSON + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > MAX_JSON {
            return Err("checkpoint JSON exceeds bounded decode size".into());
        }
        Ok(raw)
    } else {
        Ok(bytes.to_vec())
    }
}
fn checkpoint<T: serde::de::DeserializeOwned + Serialize>(
    m: &Manifest,
    id: &str,
    cache: &BTreeMap<String, Vec<u8>>,
) -> Result<T> {
    let f = m.files.get(id).ok_or("missing file descriptor")?;
    let bytes = cache.get(id).ok_or("checkpoint not selected")?;
    let raw = json_bytes(f, bytes)?;
    let value: T = decode(&raw)?;
    let expected = f
        .native_typed_identity_sha256
        .as_ref()
        .ok_or("missing recorded native checkpoint identity")?;
    if sha(&raw) != *expected || identity(&value)? != *expected {
        return Err("native checkpoint identity/canonical serialization mismatch".into());
    }
    Ok(value)
}

#[derive(Debug, Serialize)]
struct ActorReceipt {
    file: String,
    identity_sha256: String,
    optimizer_step: u32,
    weights: usize,
    complete_weights_moments_integer_ages_mask_config_exact: bool,
}
#[derive(Debug, Serialize)]
struct PrivateReceipt {
    file: String,
    actor: String,
    identity_sha256: String,
    world_tick: u64,
    sampling_counter: u32,
    complete_world_private_rng_policy_provenance_exact: bool,
}
#[derive(Debug, Serialize)]
struct Receipt {
    mode: &'static str,
    manifest_sha256: String,
    profile: String,
    actor_admissions: Vec<ActorReceipt>,
    private_admissions: Vec<PrivateReceipt>,
    input_files_hashed_before_after: usize,
    input_hashes_unchanged: bool,
    forbidden_operation_attempts: BTreeMap<&'static str, u64>,
    matching_critic_version: &'static str,
    joint_actor_critic_certified: bool,
    ppo_continuation_certified: bool,
    checkpoint_schema_or_admission_bypassed: bool,
}

fn admit_actor(
    p: &alife_core::BrainPhenotype,
    asset: &FoundationWeightAsset,
    cp: FoundationTrainerCheckpoint,
) -> Result<CpuFoundationTrainer> {
    let before = serde_json::to_vec(&cp)?;
    let actor = CpuFoundationTrainer::from_checkpoint(p.clone(), asset.clone(), cp)?;
    if serde_json::to_vec(&actor.checkpoint())? != before {
        return Err("actor constructor changed retained weights/Adam/configuration".into());
    }
    Ok(actor)
}
fn admit_private(
    cp: CpuCanonicalCheckpoint,
    actor: &CpuFoundationTrainer,
) -> Result<PrivateReceipt> {
    let before = serde_json::to_vec(&cp)?;
    let tick = cp.world.world.tick.raw();
    let counter = cp.sampling_counter;
    let host = CpuCanonicalHost::from_checkpoint(cp, actor)?;
    if serde_json::to_vec(&host.checkpoint(actor)?)? != before {
        return Err(
            "private constructor changed retained world/private/RNG/provenance state".into(),
        );
    }
    if host.world().tick().raw() != tick {
        return Err("private constructor advanced world tick".into());
    }
    Ok(PrivateReceipt {
        file: String::new(),
        actor: String::new(),
        identity_sha256: sha(&before),
        world_tick: tick,
        sampling_counter: counter,
        complete_world_private_rng_policy_provenance_exact: true,
    })
}
fn inspect_loaded(
    m: &Manifest,
    profile: &str,
    cache: &BTreeMap<String, Vec<u8>>,
    scope: &load_only::Scope,
) -> Result<(Vec<ActorReceipt>, Vec<PrivateReceipt>)> {
    let ids = selected(m, profile)?;
    for id in &ids {
        verify_bytes(
            &m.files[*id],
            cache.get(*id).ok_or("missing selected input")?,
        )?;
    }
    let base = &cache[&m.original.recovery_file];
    if cache[&m.original.checksum_file] != sha(base).as_bytes() {
        return Err("original recovery checksum sidecar mismatch".into());
    }
    let r: Recovery = decode(base)?;
    if r.kind != "disposable-retained-language-pilot-v1" {
        return Err("unsupported actor-only recovery kind".into());
    }
    // The recorded gradient and assessment are data only. Never dispatch them.
    let _ = (&r.next_gradient, &r.assessment);
    if identity(&r.actor)? != m.original.native_actor_identity_sha256
        || identity(&r.host)? != m.original.native_private_identity_sha256
    {
        return Err("original actor/private identity mismatch".into());
    }
    let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
    let asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
    let original = admit_actor(&p, &asset, r.actor)?;
    let mut ar = vec![ActorReceipt {
        file: format!("{}#actor", m.original.recovery_file),
        identity_sha256: identity(&original.checkpoint())?,
        optimizer_step: original.optimizer_step(),
        weights: original.weights().len(),
        complete_weights_moments_integer_ages_mask_config_exact: true,
    }];
    let mut private = admit_private(r.host, &original)?;
    private.file = format!("{}#host", m.original.recovery_file);
    private.actor = ar[0].file.clone();
    let mut pr = vec![private];
    let mut actor_ids = BTreeSet::new();
    for a in m.actors.values() {
        actor_ids.insert(a.actor_file.as_str());
        for u in &a.updates {
            actor_ids.insert(u.before.as_str());
            actor_ids.insert(u.after.as_str());
        }
    }
    let mut admitted = BTreeMap::new();
    for id in actor_ids.iter().filter(|id| ids.contains(**id)) {
        let cp: FoundationTrainerCheckpoint = checkpoint(m, id, cache)?;
        let a = admit_actor(&p, &asset, cp)?;
        ar.push(ActorReceipt {
            file: (*id).into(),
            identity_sha256: identity(&a.checkpoint())?,
            optimizer_step: a.optimizer_step(),
            weights: a.weights().len(),
            complete_weights_moments_integer_ages_mask_config_exact: true,
        });
        admitted.insert(*id, a);
    }
    let mut bindings = m
        .latest_private_end_files
        .iter()
        .map(|(arm, id)| {
            Ok((
                id.as_str(),
                m.actors
                    .get(arm)
                    .ok_or("private arm lacks actor")?
                    .actor_file
                    .as_str(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    bindings.push((
        m.known_positive_private_start.as_str(),
        m.actors
            .get("Untouched")
            .ok_or("missing original Untouched binding")?
            .actor_file
            .as_str(),
    ));
    for (id, aid) in bindings.into_iter().filter(|(id, _)| ids.contains(*id)) {
        let dependencies = &m.files[id].requires;
        let actor_deps = dependencies
            .iter()
            .filter(|d| actor_ids.contains(d.as_str()))
            .map(String::as_str)
            .collect::<Vec<_>>();
        if actor_deps != [aid]
            || !dependencies.contains(&m.original.recovery_file)
            || !dependencies.contains(&m.original.checksum_file)
        {
            return Err("private checkpoint has mismatched actor/asset dependencies".into());
        }
        let a = admitted.get(aid).ok_or("private actor not admitted")?;
        let cp: CpuCanonicalCheckpoint = checkpoint(m, id, cache)?;
        let mut receipt = admit_private(cp, a)?;
        receipt.file = id.into();
        receipt.actor = aid.into();
        pr.push(receipt);
    }
    if scope.attempts() != [0; 7] {
        return Err(format!(
            "native constructors attempted forbidden work: {:?}",
            scope.attempts()
        )
        .into());
    }
    Ok((ar, pr))
}
fn inspect_files(request: Request) -> Result<Receipt> {
    inspect_files_with_after_admission(request, || Ok(()))
}
// Private test seam: the production entry point supplies only the no-op above.
// A fixture can change its own input here to exercise the actual second read.
fn inspect_files_with_after_admission(
    request: Request,
    after_admission: impl FnOnce() -> Result<()>,
) -> Result<Receipt> {
    if request.require_joint {
        return Err("matching critic/version state is unavailable; refusing joint actor/critic or PPO continuation certification; no critic initialized".into());
    }
    let scope = load_only::Scope::enter()?;
    let bytes = fs::read(&request.manifest)?;
    if sha(&bytes) != request.expected_sha256 {
        return Err("owner-pinned manifest SHA256 mismatch".into());
    }
    let m: Manifest = decode(&bytes)?;
    let ids = selected(&m, &request.profile)?;
    let mut cache = BTreeMap::new();
    let mut paths = BTreeMap::new();
    for id in ids {
        let spec = &m.files[id];
        let path = resolve(&m, spec)?;
        if fs::metadata(&path)?.len() != spec.bytes {
            return Err("input metadata size mismatch".into());
        }
        let bytes = fs::read(&path)?;
        verify_bytes(spec, &bytes)?;
        cache.insert(id.to_owned(), bytes);
        paths.insert(id.to_owned(), path);
    }
    let result = inspect_loaded(&m, &request.profile, &cache, &scope);
    after_admission()?;
    // Also checked after a failed semantic admission: no input can be repaired,
    // resealed, rewritten or replaced by a default to make validation succeed.
    for (id, path) in &paths {
        verify_bytes(&m.files[id], &fs::read(path)?)?;
    }
    if fs::read(&request.manifest)? != bytes {
        return Err("manifest changed during admission".into());
    }
    let attempts = scope.attempts();
    if attempts != [0; 7] {
        return Err(format!(
            "load-only violation; constructor attempted {:?}; admission stopped",
            attempts
        )
        .into());
    }
    let (actor_admissions, private_admissions) = result?;
    Ok(Receipt {
        mode: "native-load-only-actor-private",
        manifest_sha256: request.expected_sha256,
        profile: request.profile,
        actor_admissions,
        private_admissions,
        input_files_hashed_before_after: paths.len() + 1,
        input_hashes_unchanged: true,
        forbidden_operation_attempts: load_only::Operation::NAMES
            .into_iter()
            .zip(attempts)
            .collect(),
        matching_critic_version: "unavailable_actor_only_lineage",
        joint_actor_critic_certified: false,
        ppo_continuation_certified: false,
        checkpoint_schema_or_admission_bypassed: false,
    })
}
fn main() -> Result<()> {
    let request = parse(std::env::args().skip(1))?;
    let value = std::thread::Builder::new()
        .name("native-load-only".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || inspect_files(request).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "load-only admission thread panicked")?
        .map_err(|s| -> Box<dyn std::error::Error + Send + Sync> { s.into() })?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::*;
    use alife_training::{AdamWConfig, StageTrainableMask};
    use alife_world::*;

    #[test]
    fn cli_has_no_simulation_or_rewrite_route_and_requires_owner_digest() {
        let digest = "a".repeat(64);
        let p = |a: Vec<&str>| parse(a.into_iter().map(str::to_owned));
        assert!(p(vec!["--manifest", "m"]).is_err());
        assert!(p(vec![
            "--manifest",
            "m",
            "--manifest-sha256",
            &digest,
            "--experimental-reload-child",
            "x"
        ])
        .is_err());
        assert!(p(vec![
            "--manifest",
            "m",
            "--manifest-sha256",
            &digest,
            "--profile",
            "full_recorded_evidence"
        ])
        .is_err());
        for flag in ["--require-joint-critic", "--require-ppo"] {
            let r = p(vec![
                "--manifest",
                "nonexistent",
                "--manifest-sha256",
                &digest,
                flag,
            ])
            .unwrap();
            assert!(r.require_joint);
            assert!(inspect_files(r)
                .unwrap_err()
                .to_string()
                .contains("critic/version"));
        }
    }
    #[test]
    fn byte_corruption_and_traversal_fail_without_admission() {
        let b = b"unchanged fixture";
        let s = FileSpec {
            root: "r".into(),
            path: "x".into(),
            bytes: b.len() as u64,
            sha256: sha(b),
            requires: vec![],
            native_typed_identity_sha256: None,
        };
        verify_bytes(&s, b).unwrap();
        assert!(verify_bytes(&s, b"corrupt").is_err());
        let mut bad = s;
        bad.path = "../x".into();
        let m:Manifest=serde_json::from_value(serde_json::json!({"schema":1,"roots":{},"files":{},"profiles":{},"original":{"recovery_file":"a","checksum_file":"b","native_actor_identity_sha256":"","native_private_identity_sha256":""},"actors":{},"latest_private_end_files":{},"known_positive_private_start":"","critic":{"present":false}})).unwrap();
        assert!(resolve(&m, &bad).is_err());
    }
    #[test]
    fn missing_dependency_closure_is_rejected_before_native_admission() {
        let file = serde_json::json!({"root":"r","path":"x","bytes":0,"sha256":"","requires":[]});
        let mut m: Manifest = serde_json::from_value(serde_json::json!({
            "schema":1,"roots":{},"files":{"a":file,"b":file},
            "profiles":{"actor_history":{"files":["a","b"]}},
            "original":{"recovery_file":"a","checksum_file":"b","native_actor_identity_sha256":"","native_private_identity_sha256":""},
            "actors":{},"latest_private_end_files":{},"known_positive_private_start":"","critic":{"present":false}
        })).unwrap();
        assert!(selected(&m, "actor_history").is_ok());
        m.files
            .get_mut("a")
            .unwrap()
            .requires
            .push("missing".into());
        assert!(selected(&m, "actor_history").is_err());
        m.files.get_mut("a").unwrap().requires.clear();
        m.profiles.get_mut("actor_history").unwrap().files.pop();
        assert!(selected(&m, "actor_history").is_err());
    }
    pub(super) fn fixture() -> Result<(
        alife_core::BrainPhenotype,
        FoundationWeightAsset,
        FoundationTrainerCheckpoint,
        CpuCanonicalCheckpoint,
        PhenotypeCompilerInputs,
    )> {
        let mut c = CanonicalNewGameConfig::phase3(117, 1)?;
        c.brain_class = BrainScaleTier::Standard2048;
        let builtin = FoundationWeightAsset::builtin_n2048_v1(c.sensor_profile)?;
        let first = create_canonical_new_game_with_n2048_candidate(&c, &builtin)?;
        let record = first
            .world
            .organism_registry()
            .iter()
            .next()
            .ok_or("no fixture founder")?;
        let genome = record.phenotype().brain_genome.clone();
        let capacity = BrainCapacityClass::n2048();
        let development =
            alife_runtime::canonical_preparation::foundation_construction_development(
                &genome,
                &capacity,
                &record.phenotype().development_state_at(Tick::ZERO)?,
            )?;
        let p = PhenotypeCompiler::compile(&genome, &capacity, &development, c.sensor_profile)?;
        let asset = FoundationWeightAsset::from_trained_weights(
            &p,
            p.synapses().iter().map(|s| s.genetic_weight()).collect(),
            TrainingStageManifest::new(1, 1, 1),
        )?;
        let game = create_canonical_new_game_with_n2048_candidate(&c, &asset)?;
        let (p, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?;
        let mut cp = CpuFoundationTrainer::new(
            p.clone(),
            asset.clone(),
            StageTrainableMask::recurrent_only(&p)?,
            AdamWConfig::default(),
        )?
        .checkpoint();
        // In-memory serialization sentinels, not training or fabricated outcomes.
        cp.optimizer_step = (1 << 24) + 17;
        cp.update_ages[0] = cp.optimizer_step;
        cp.first_moment[0] = -0.125;
        cp.second_moment[0] = 0.25;
        let a = CpuFoundationTrainer::from_checkpoint(p.clone(), asset.clone(), cp.clone())?;
        let host = CpuCanonicalHost::new_cohort(game, &a, 117)?;
        assert_eq!(host.world().tick(), Tick::ZERO);
        Ok((p, asset, cp, host.checkpoint(&a)?, inputs))
    }
    #[test]
    #[ignore = "process-wide fail-closed guard: run this source fixture in isolation with --test-threads=1; zero simulation"]
    fn exact_native_admission_rejects_mismatch_and_forbids_all_work() -> Result<()> {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| -> std::result::Result<(), String> {
                let run = || -> Result<()> {
                    let scope = load_only::Scope::enter()?;
                    let (p, asset, cp, host, _) = fixture()?;
                    let original_actor = serde_json::to_vec(&cp)?;
                    let original_host = serde_json::to_vec(&host)?;
                    assert!(load_only::Scope::enter().is_err());
                    let mut a = admit_actor(&p, &asset, cp.clone())?;
                    let receipt = admit_private(host.clone(), &a)?;
                    assert_eq!(receipt.world_tick, 0);
                    assert_eq!(serde_json::to_vec(&a.checkpoint())?, original_actor);
                    let mut wrong = cp.clone();
                    wrong.first_moment.pop();
                    assert!(admit_actor(&p, &asset, wrong).is_err());
                    let mut wrong = cp.clone();
                    wrong.update_ages[0] = wrong.optimizer_step + 1;
                    assert!(admit_actor(&p, &asset, wrong).is_err());
                    let mut wrong = cp.clone();
                    wrong.second_moment[0] = -0.25;
                    assert!(admit_actor(&p, &asset, wrong).is_err());
                    let mut wrong = cp.clone();
                    wrong.weights[0] += 0.25;
                    let mismatched = admit_actor(&p, &asset, wrong)?;
                    assert!(admit_private(host.clone(), &mismatched).is_err());
                    let mut wrong = host.clone();
                    wrong.sampling_counter += 1;
                    assert!(admit_private(wrong, &a).is_err());
                    assert_eq!(scope.attempts(), [0; 7]);
                    let mut h = CpuCanonicalHost::from_checkpoint(host.clone(), &a)?;
                    assert!(h.step(&a, 1.).is_err());
                    assert!(h.world_mut().try_advance_tick().is_err());
                    assert!(a.apply_gradients(&vec![0.; a.weights().len()]).is_err());
                    let q = alife_training::TrainingSequence {
                        phenotype_hash: p.phenotype_hash(),
                        initial: host.residents[0].neural.neural.clone(),
                        ticks: vec![],
                        burn_in_ticks: 0,
                        memory_candidate_gain: 0.,
                    };
                    assert!(a.evaluate_replay(&q).is_err());
                    assert!(!std::thread::spawn(|| load_only::permitted(
                        load_only::Operation::NeuralForward
                    ))
                    .join()
                    .unwrap());
                    assert_eq!(scope.attempts(), [1, 2, 1, 0, 0, 1, 0]);
                    // Cover the other barrier categories directly without
                    // manufacturing a credit patch, action or a missing critic.
                    for kind in [
                        load_only::Operation::WorldAction,
                        load_only::Operation::Credit,
                        load_only::Operation::CriticAdam,
                    ] {
                        assert!(!load_only::permitted(kind));
                    }
                    assert_eq!(scope.attempts(), [1, 2, 1, 1, 1, 1, 1]);
                    assert_eq!(serde_json::to_vec(&a.checkpoint())?, original_actor);
                    assert_eq!(serde_json::to_vec(&h.checkpoint(&a)?)?, original_host);
                    drop(scope);
                    assert!(load_only::permitted(load_only::Operation::Decision));
                    Ok(())
                };
                run().map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "fixture thread panicked")?
            .map_err(|s| -> Box<dyn std::error::Error + Send + Sync> { s.into() })
    }
}

#[cfg(test)]
#[path = "verify_actor_private_recovery/qualification_tests.rs"]
mod qualification_tests;
