//! One bounded asynchronous prior service; no action, target or memory authority.
use alife_core::{
    CompressedSemanticCode, Confidence, ContextFeatureFlags, ExperienceSequenceId,
    NormalizedScalar, PerceptionFrame, PerceptionFrameDraft, SemanticContextRef,
    SemanticPriorPacket, SemanticPriorRequest, BASIC_VOCABULARY_V1,
};
use alife_semantic::{
    BoundedSlmPriorProvider, DeterministicPriorProvider, DevelopmentalPriorController,
    LlamaCppSlmPriorConfig, LocalSlmPriorAsyncQueue, LocalSlmPriorOutput, LocalSlmPriorRequest,
    LunaPriorConfig, LunaPriorProvider, RecordedPriorBank, CA27_UNUSABLE_HINT_FEEDBACK,
    CA27_UNUSABLE_HINT_FEEDBACK_VERSION,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SemanticPriorMetrics {
    pub backend: String,
    pub context_contract: String,
    pub source_provider_identity: Option<String>,
    pub recorded_bank_digest: Option<String>,
    pub model: String,
    pub provider_identity: String,
    pub model_sha256: String,
    pub deliveries: VecDeque<PriorDelivery>,
    pub requests: u64,
    pub delivered_frames: u64,
    pub cache_hits: u64,
    pub stale_replies: u64,
    pub failures: u64,
    pub dropout_frames: u64,
    pub last_error: Option<String>,
    pub prime_wait_ms: u64,
    pub prime_timeouts: u64,
    pub empty_hint_frames: u64,
    pub rich_information_required: bool,
    pub validated_provider_replies: u64,
    pub decision_inputs: VecDeque<PriorDecisionInput>,
    pub decision_frames: u64,
    pub decision_frames_with_prior: u64,
    pub decision_frames_without_prior: u64,
    pub decision_frames_with_heard_language: u64,
    pub provider_failures: VecDeque<PriorProviderFailure>,
    pub unusable_hint_retry_requests: u64,
    pub unusable_hint_retries: VecDeque<PriorUnusableHintRetry>,
    pub last_priming: Option<PriorPrimingStatus>,
}
const MAX_RECEIPTS: usize = 256;
const MAX_UNUSABLE_RETRY_CONTEXTS: usize = 128;
const PROVIDER_FAILURE_COOLDOWN: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorUnusableHintRetry {
    pub organism: u64,
    pub tick: u64,
    pub context: String,
    pub context_digest: String,
    pub previous_output: LocalSlmPriorOutput,
    pub previous_output_digest: String,
    pub feedback: String,
    pub feedback_version: String,
    pub replacement_output: Option<LocalSlmPriorOutput>,
    pub replacement_output_digest: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorPrimingStatus {
    pub organism: u64,
    pub tick: u64,
    pub context: String,
    pub context_digest: String,
    pub status: PriorInputStatus,
    pub developmental_gain: f32,
    pub cache_match: bool,
    pub cache_usable_hints: usize,
    pub cache_output: Option<LocalSlmPriorOutput>,
    pub cache_output_digest: Option<String>,
    pub active_context_match: bool,
    pub pending: bool,
    pub unusable_retry_used: bool,
    pub ready: bool,
    pub exit_reason: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorInputStatus {
    #[default]
    Unavailable,
    Delivered,
    UnsupportedProfile,
    DeliberateDropout,
    Pending,
    ProviderFailure,
    EmptyHints,
    DevelopmentalGainZero,
    NoSemanticEncoder,
}

/// Confirmed inference input, never evidence that the creature used the hint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorDecisionInput {
    pub organism: u64,
    pub sequence: u64,
    pub tick: u64,
    pub dispatch_generation: u64,
    pub frame_digest: alife_core::PerceptionFrameDigest,
    pub sensory_digest: String,
    pub natural_senses_context: String,
    pub heard_tokens: usize,
    pub grounded_object_slots: usize,
    pub semantic_codes: usize,
    pub nonzero_prior_lanes: usize,
    pub semantic_encoder_lanes: usize,
    pub nonzero_encoded_prior_lanes: usize,
    pub gain: f32,
    pub status: PriorInputStatus,
    pub context_digest: Option<String>,
    pub output_digest: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorProviderFailure {
    pub organism: u64,
    pub tick: u64,
    pub context_digest: String,
    pub error: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorDelivery {
    pub organism: u64,
    pub sequence: u64,
    pub issued_tick: u64,
    pub expires_tick: u64,
    pub context_digest: String,
    pub output_digest: String,
    pub gain: f32,
}
struct Pending {
    key: String,
    reply: Receiver<Result<LocalSlmPriorOutput, String>>,
    unusable_hint_feedback: bool,
    submitted_at: Instant,
}
#[derive(Default)]
struct Life {
    controller: DevelopmentalPriorController,
    pending: Option<Pending>,
    active: Option<(String, SemanticPriorPacket, LocalSlmPriorOutput)>,
    last_request: Option<(String, u64)>,
    recently_heard: Option<(u64, Vec<String>)>,
    last_context: String,
    input_status: PriorInputStatus,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct PriorResume {
    schema: u16,
    provider: String,
    controller: DevelopmentalPriorController,
    active: Option<(String, SemanticPriorPacket, LocalSlmPriorOutput)>,
    last_request: Option<(String, u64)>,
    recently_heard: Option<(u64, Vec<String>)>,
    next_request: u64,
    pending_request: bool,
}
pub(super) struct RuntimeSemanticPrior {
    queue: Option<LocalSlmPriorAsyncQueue>,
    retry_not_before: Option<Instant>,
    reported_empty_contexts: BTreeSet<String>,
    frozen_recording: bool,
    deterministic: Option<DeterministicPriorProvider>,
    lives: BTreeMap<u64, Life>,
    // This cache belongs to one fixed provider configuration and prompt version.
    cache: BTreeMap<String, LocalSlmPriorOutput>,
    unusable_retry_contexts: BTreeSet<String>,
    next_request: u64,
    cache_path: std::path::PathBuf,
    dropout_seed: Option<u64>,
    pub metrics: SemanticPriorMetrics,
}
impl RuntimeSemanticPrior {
    pub fn from_environment(
        seed: u64,
        training: bool,
    ) -> Result<Option<Self>, alife_core::ScaffoldContractError> {
        let backend = selected_backend(
            training,
            std::env::var("ALIFE_SLM_PRIOR_BACKEND").ok(),
            std::env::var("ALIFE_SLM_PRIOR").is_ok_and(|s| s == "off"),
        );
        let context_contract = context_contract();
        let mut config = LlamaCppSlmPriorConfig {
            model: std::env::var("ALIFE_SLM_PRIOR_MODEL")
                .unwrap_or_else(|_| "alife-qwen3.5-0.8b-prior".into()),
            timeout_ms: 30_000,
            association_vocabulary: BASIC_VOCABULARY_V1
                .iter()
                .map(|(word, _)| (*word).to_string())
                .collect(),
            ..Default::default()
        };
        let mut model_sha256 = std::env::var("ALIFE_SLM_PRIOR_MODEL_SHA256").unwrap_or_else(|_| {
            if config.model == "alife-qwen3.5-0.8b-prior" {
                "37ae482d336108d23516fa35e8e0c4126688d81018b87178a18d752a1357814f".into()
            } else {
                "unverified-model".into()
            }
        });
        let mut source_provider_identity = None;
        let mut recorded_bank_digest = None;
        let mut frozen_cache = BTreeMap::new();
        let mut setup_error = None;
        let mut deterministic = None;
        let mut backend_config = format!(
            "{}:{}:{}:{}:{}",
            config.host, config.port, config.model, config.num_predict, model_sha256
        );
        let queue = match backend.as_str() {
            "off" => {
                // Keep only the dormant developmental/checkpoint state carrier.
                // Disabling optional delivery never creates a service worker.
                config.model = "disabled".into();
                model_sha256 = "unverified-model".into();
                backend_config = "disabled".into();
                Ok(None)
            }
            "deterministic" => {
                config.model = alife_semantic::DETERMINISTIC_PRIOR_ID.into();
                model_sha256 = "unverified-model".into();
                backend_config = alife_semantic::DETERMINISTIC_PRIOR_ID.into();
                deterministic = Some(DeterministicPriorProvider::default());
                Ok(None)
            }
            "local" => LocalSlmPriorAsyncQueue::new(config.clone())
                .map(Some)
                .map_err(|error| format!("local SLM prior initialization failed: {error:?}")),
            "luna" => {
                let luna = LunaPriorConfig {
                    executable: std::env::var_os("ALIFE_SLM_PRIOR_LUNA_EXECUTABLE")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| "codex".into()),
                    model: std::env::var("ALIFE_SLM_PRIOR_LUNA_MODEL")
                        .unwrap_or_else(|_| "gpt-6-luna".into()),
                    timeout_ms: config.timeout_ms,
                    association_vocabulary: config.association_vocabulary.clone(),
                    ..Default::default()
                };
                config.model = luna.source_identity();
                model_sha256 = "unverified-model".into();
                backend_config = serde_json::to_string(&luna).unwrap_or_default();
                LunaPriorProvider::new(luna).and_then(|provider| {
                    LocalSlmPriorAsyncQueue::with_provider(config.clone(), Box::new(provider))
                        .map(Some)
                        .map_err(|error| {
                            format!("Luna prior worker initialization failed: {error:?}")
                        })
                })
            }
            "recorded" => {
                let path = std::env::var_os("ALIFE_SLM_PRIOR_BANK").map(std::path::PathBuf::from);
                path.ok_or_else(|| "recorded prior requires ALIFE_SLM_PRIOR_BANK".to_string())
                    .and_then(|path| {
                        let bank = RecordedPriorBank::load(&path, &context_contract)?;
                        config.model = bank.origin().model.clone();
                        model_sha256 = bank.origin().model_sha256.clone();
                        source_provider_identity = Some(bank.origin().provider_identity.clone());
                        let digest = blake3::hash(&bank.to_json_vec()?).to_hex().to_string();
                        recorded_bank_digest = Some(digest.clone());
                        backend_config = digest;
                        frozen_cache = bank.entries().clone();
                        Ok(None)
                    })
            }
            _ => Err(format!(
                "unknown semantic prior backend {backend:?}; expected deterministic, local, luna, recorded or off"
            )),
        };
        let queue = match queue {
            Ok(queue) => queue,
            Err(error) => {
                setup_error = Some(error);
                None
            }
        };
        let identity = format!(
            "{backend}:{backend_config}:{context_contract}:{}:{}:{}:{}:{}",
            blake3::hash(include_bytes!(
                "../../../alife_semantic/src/local_slm_prior.rs"
            ))
            .to_hex(),
            blake3::hash(include_bytes!("../../../alife_semantic/src/luna_prior.rs")).to_hex(),
            blake3::hash(include_bytes!(
                "../../../alife_semantic/src/recorded_prior.rs"
            ))
            .to_hex(),
            blake3::hash(include_bytes!(
                "../../../alife_semantic/src/deterministic_prior.rs"
            ))
            .to_hex(),
            blake3::hash(include_bytes!("semantic_prior.rs")).to_hex()
        );
        let provider_identity = blake3::hash(identity.as_bytes()).to_hex().to_string();
        let cache_path = std::path::PathBuf::from("target/slm-prior/cache")
            .join(format!("{provider_identity}.json"));
        let frozen_recording = backend == "recorded";
        let cache = if frozen_recording {
            frozen_cache
        } else if deterministic.is_some() || backend == "off" {
            BTreeMap::new()
        } else {
            load_live_cache(&cache_path, &config.model, &mut setup_error)
        };
        let mut metrics = SemanticPriorMetrics {
            backend: backend.clone(),
            context_contract,
            source_provider_identity,
            recorded_bank_digest,
            model: config.model,
            provider_identity,
            model_sha256,
            ..Default::default()
        };
        if let Some(error) = setup_error {
            record_provider_failure(&mut metrics, 0, 0, "provider initialization", error);
        }
        Ok(Some(Self {
            queue,
            retry_not_before: None,
            reported_empty_contexts: BTreeSet::new(),
            frozen_recording,
            deterministic,
            lives: BTreeMap::new(),
            cache,
            unusable_retry_contexts: BTreeSet::new(),
            cache_path,
            next_request: 1,
            dropout_seed: (training && backend != "off").then_some(seed),
            metrics,
        }))
    }

    #[cfg(feature = "foundation-training")]
    pub fn require_rich_information(&mut self) {
        self.dropout_seed = None;
        self.metrics.rich_information_required = true;
    }

    #[cfg(feature = "foundation-training")]
    pub fn ready(&self, id: u64) -> bool {
        self.lives.get(&id).is_some_and(|life| {
            life.controller.developmental_gain() > 0.0
                && (self
                    .cache
                    .get(&life.last_context)
                    .is_some_and(has_usable_hints)
                    || life.active.as_ref().is_some_and(|(key, packet, output)| {
                        key == &life.last_context
                            && packet.plasticity_modulation > 0.0
                            && has_usable_hints(output)
                    }))
        })
    }

    #[cfg(feature = "foundation-training")]
    pub fn record_priming_status(&mut self, id: u64, tick: u64, exit_reason: &str) {
        let Some(life) = self.lives.get(&id) else {
            return;
        };
        let output = self.cache.get(&life.last_context);
        let usable = output.map_or(0, |output| {
            BASIC_VOCABULARY_V1
                .iter()
                .filter(|(_, code)| lexicon_slot_salience(output, *code) > 0.0)
                .count()
        });
        self.metrics.last_priming = Some(PriorPrimingStatus {
            organism: id,
            tick,
            context: life.last_context.clone(),
            context_digest: blake3::hash(life.last_context.as_bytes())
                .to_hex()
                .to_string(),
            status: life.input_status,
            developmental_gain: life.controller.developmental_gain(),
            cache_match: output.is_some(),
            cache_usable_hints: usable,
            cache_output: output.cloned(),
            cache_output_digest: output.map(digest_json),
            active_context_match: life
                .active
                .as_ref()
                .is_some_and(|(key, _, _)| key == &life.last_context),
            pending: life.pending.is_some(),
            unusable_retry_used: self.unusable_retry_contexts.contains(&life.last_context),
            ready: self.ready(id),
            exit_reason: exit_reason.into(),
        });
    }

    pub fn record_decision_input(
        &mut self,
        frame: &PerceptionFrame,
        sequence: u64,
        dispatch_generation: u64,
        semantic_encoder_lanes: usize,
        nonzero_encoded_prior_lanes: usize,
    ) {
        let life = self.lives.get(&frame.organism_id().raw());
        let context = frame.sensory().semantic_context.as_ref();
        let gain = context.map_or(0.0, |prior| prior.confidence.raw());
        let nonzero_prior_lanes = frame.sensory().language_prior_neural_lanes()[128..]
            .iter()
            .filter(|value| **value != 0.0)
            .count();
        let status = if nonzero_prior_lanes > 0 {
            if nonzero_encoded_prior_lanes > 0 {
                PriorInputStatus::Delivered
            } else {
                PriorInputStatus::NoSemanticEncoder
            }
        } else if context.is_some() && gain == 0.0 {
            PriorInputStatus::DevelopmentalGainZero
        } else {
            match life.map_or(PriorInputStatus::Unavailable, |life| life.input_status) {
                // Preparation can mark a cached hint ready. Only actual frame
                // lanes and encoder contributions establish delivery above.
                PriorInputStatus::Delivered => PriorInputStatus::EmptyHints,
                status => status,
            }
        };
        let active = life.and_then(|life| life.active.as_ref());
        let receipt = PriorDecisionInput {
            organism: frame.organism_id().raw(),
            sequence,
            tick: frame.tick().raw(),
            dispatch_generation,
            frame_digest: frame.frame_digest(),
            sensory_digest: digest_json(frame.sensory()),
            natural_senses_context: life.map_or_else(String::new, |life| life.last_context.clone()),
            heard_tokens: frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .count(),
            grounded_object_slots: frame.grounded_object_slots().len(),
            semantic_codes: context.map_or(0, |prior| prior.compressed_codes.len()),
            nonzero_prior_lanes,
            semantic_encoder_lanes,
            nonzero_encoded_prior_lanes,
            gain,
            status,
            context_digest: active
                .map(|(key, _, _)| blake3::hash(key.as_bytes()).to_hex().to_string()),
            output_digest: active.map(|(_, _, output)| digest_json(output)),
        };
        self.metrics.decision_frames += 1;
        if status == PriorInputStatus::Delivered {
            self.metrics.decision_frames_with_prior += 1;
        } else {
            self.metrics.decision_frames_without_prior += 1;
        }
        if receipt.heard_tokens > 0 {
            self.metrics.decision_frames_with_heard_language += 1;
        }
        push_bounded(&mut self.metrics.decision_inputs, receipt);
    }

    pub fn prepare(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
    ) -> Result<PerceptionFrameDraft, alife_core::ScaffoldContractError> {
        self.prepare_inner(draft, sequence, true)
    }
    #[cfg(feature = "foundation-training")]
    pub fn prime(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
    ) -> Result<(), alife_core::ScaffoldContractError> {
        self.prepare_inner(draft, sequence, false).map(|_| ())
    }
    #[cfg(feature = "foundation-training")]
    pub fn pending(&self, id: u64) -> bool {
        self.lives
            .get(&id)
            .is_some_and(|life| life.pending.is_some())
    }
    fn prepare_inner(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
        consume: bool,
    ) -> Result<PerceptionFrameDraft, alife_core::ScaffoldContractError> {
        let id = draft.organism_id();
        let tick = draft.tick();
        // Organism/sequence corruption is not an optional provider failure.
        SemanticPriorRequest::new(id, sequence)?;
        if draft.sensor_profile() != alife_core::SensorProfile::GroundedTerrainVisionV1 {
            self.lives.entry(id.raw()).or_default().input_status =
                PriorInputStatus::UnsupportedProfile;
            return Ok(draft);
        }
        if self
            .dropout_seed
            .is_some_and(|seed| (seed ^ id.raw()).is_multiple_of(5))
        {
            if consume {
                self.metrics.dropout_frames += 1;
            }
            self.lives.entry(id.raw()).or_default().input_status =
                PriorInputStatus::DeliberateDropout;
            return Ok(draft);
        }
        let life = self.lives.entry(id.raw()).or_default();
        let heard = heard_words(&draft);
        if !heard.is_empty() {
            life.recently_heard = Some((tick.raw(), heard));
        }
        if life
            .recently_heard
            .as_ref()
            .is_some_and(|(at, _)| tick.raw().saturating_sub(*at) >= 32)
        {
            life.recently_heard = None;
        }
        // A bounded private observation history. This never fabricates current
        // hearing inputs or writes the learner's memory; changed words invalidate
        // an old reply immediately, and old words expire in simulation ticks.
        let key = bounded_context(
            &draft,
            life.recently_heard
                .as_ref()
                .map_or(&[], |(_, words)| words.as_slice()),
        );
        if life.last_context != key {
            life.input_status = PriorInputStatus::Unavailable;
        }
        life.last_context = key.clone();
        if let Some(provider) = self.deterministic.as_ref() {
            if !self.cache.contains_key(&key) {
                match provider.generate_prior(&key, false) {
                    Ok(output) if output.validate().is_ok() => {
                        if self.cache.len() >= 128 {
                            self.cache.pop_first();
                        }
                        self.cache.insert(key.clone(), output);
                        self.metrics.requests += 1;
                        self.metrics.validated_provider_replies += 1;
                    }
                    result => {
                        let error = match result {
                            Err(error) => error,
                            Ok(_) => "deterministic prior failed output validation".into(),
                        };
                        record_provider_failure(
                            &mut self.metrics,
                            id.raw(),
                            tick.raw(),
                            &key,
                            error,
                        );
                        life.active = None;
                        life.input_status = PriorInputStatus::ProviderFailure;
                        if consume {
                            life.controller.record_relevant_exposure(false);
                        }
                        return draft.with_semantic_context(None);
                    }
                }
            }
        }
        if self.queue.is_none() && !self.frozen_recording && self.deterministic.is_none() {
            life.active = None;
            life.input_status = PriorInputStatus::Unavailable;
            if consume {
                life.controller.record_relevant_exposure(false);
            }
            return draft.with_semantic_context(None);
        }
        if let Some(pending) = life.pending.take() {
            match pending.reply.try_recv() {
                Ok(Ok(output)) => {
                    if let Err(error) = output.validate() {
                        record_provider_failure(
                            &mut self.metrics,
                            id.raw(),
                            tick.raw(),
                            &pending.key,
                            format!("invalid provider output: {error:?}"),
                        );
                        life.input_status = PriorInputStatus::ProviderFailure;
                        life.active = None;
                        self.cache.remove(&pending.key);
                        self.retry_not_before = Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                        if consume {
                            life.controller.record_relevant_exposure(false);
                        }
                        return draft.with_semantic_context(None);
                    }
                    self.metrics.validated_provider_replies += 1;
                    self.retry_not_before = None;
                    if pending.unusable_hint_feedback {
                        if let Some(retry) = self
                            .metrics
                            .unusable_hint_retries
                            .iter_mut()
                            .rev()
                            .find(|retry| {
                                retry.organism == id.raw()
                                    && retry.context == pending.key
                                    && retry.replacement_output.is_none()
                            })
                        {
                            retry.replacement_output_digest = Some(digest_json(&output));
                            retry.replacement_output = Some(output.clone());
                        }
                    }
                    if pending.key != key {
                        self.metrics.stale_replies += 1;
                    }
                    if self.cache.len() >= 128 {
                        self.cache.pop_first();
                    }
                    self.cache.insert(pending.key, output);
                    if let Some(parent) = self.cache_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(bytes) = serde_json::to_vec(&self.cache) {
                        let _ = std::fs::write(&self.cache_path, bytes);
                    }
                }
                Ok(Err(error)) => {
                    record_provider_failure(
                        &mut self.metrics,
                        id.raw(),
                        tick.raw(),
                        &pending.key,
                        error,
                    );
                    life.input_status = PriorInputStatus::ProviderFailure;
                    life.active = None;
                    self.cache.remove(&pending.key);
                    self.retry_not_before = Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                }
                Err(TryRecvError::Empty) => {
                    if pending.submitted_at.elapsed()
                        >= Duration::from_millis(
                            self.queue
                                .as_ref()
                                .map_or(30_000, |queue| queue.timeout_ms()),
                        )
                    {
                        self.metrics.prime_timeouts += 1;
                        record_provider_failure(
                            &mut self.metrics,
                            id.raw(),
                            tick.raw(),
                            &pending.key,
                            "local SLM prior asynchronous request timed out".into(),
                        );
                        life.active = None;
                        self.cache.remove(&pending.key);
                        life.input_status = PriorInputStatus::ProviderFailure;
                        self.retry_not_before = Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                    } else {
                        life.pending = Some(pending);
                        life.input_status = PriorInputStatus::Pending;
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    record_provider_failure(
                        &mut self.metrics,
                        id.raw(),
                        tick.raw(),
                        &pending.key,
                        "local SLM prior worker disconnected".into(),
                    );
                    life.input_status = PriorInputStatus::ProviderFailure;
                    life.active = None;
                    self.cache.remove(&pending.key);
                    self.retry_not_before = Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                }
            }
        }
        if life.pending.is_none() && life.input_status == PriorInputStatus::Pending {
            life.input_status = PriorInputStatus::Unavailable;
        }
        if life.active.as_ref().is_some_and(|(context, packet, _)| {
            context != &key || tick.raw() >= packet.expires_at_tick.raw()
        }) {
            life.active = None;
        }
        if life.active.is_none() {
            if let Some(output) = self.cache.get(&key) {
                if !has_usable_hints(output)
                    && self.reported_empty_contexts.len() < MAX_UNUSABLE_RETRY_CONTEXTS
                    && self.reported_empty_contexts.insert(key.clone())
                {
                    record_provider_failure(
                        &mut self.metrics,
                        id.raw(),
                        tick.raw(),
                        &key,
                        "schema-valid semantic prior supplied no usable hints".into(),
                    );
                }
                if !consume {
                    life.input_status = if has_usable_hints(output) {
                        PriorInputStatus::Delivered
                    } else {
                        PriorInputStatus::EmptyHints
                    };
                    // Schema-valid zero hints remain evidence, but do not become
                    // a permanent admission failure. One same-context feedback
                    // request is allowed for a rich lesson; a second zero still
                    // remains unaided. No world/neural state advances here.
                    if self.queue.is_some()
                        && !self.frozen_recording
                        && self.metrics.rich_information_required
                        && !has_usable_hints(output)
                        && life.controller.developmental_gain() > 0.0
                        && life.pending.is_none()
                        && self.retry_not_before.is_none_or(|at| Instant::now() >= at)
                        && self.unusable_retry_contexts.len() < MAX_UNUSABLE_RETRY_CONTEXTS
                        && self.unusable_retry_contexts.insert(key.clone())
                    {
                        push_bounded(
                            &mut self.metrics.unusable_hint_retries,
                            PriorUnusableHintRetry {
                                organism: id.raw(),
                                tick: tick.raw(),
                                context: key.clone(),
                                context_digest: blake3::hash(key.as_bytes()).to_hex().to_string(),
                                previous_output: output.clone(),
                                previous_output_digest: digest_json(output),
                                feedback: CA27_UNUSABLE_HINT_FEEDBACK.into(),
                                feedback_version: CA27_UNUSABLE_HINT_FEEDBACK_VERSION.into(),
                                replacement_output: None,
                                replacement_output_digest: None,
                            },
                        );
                        let request = LocalSlmPriorRequest {
                            request_id: self.next_request,
                            prompt: key.clone(),
                        };
                        match self
                            .queue
                            .as_ref()
                            .expect("live prior queue checked")
                            .submit_unusable_hint_retry(request)
                        {
                            Ok(reply) => {
                                self.next_request = self
                                    .next_request
                                    .checked_add(1)
                                    .ok_or(alife_core::ScaffoldContractError::InvalidId)?;
                                life.pending = Some(Pending {
                                    key: key.clone(),
                                    reply,
                                    unusable_hint_feedback: true,
                                    submitted_at: Instant::now(),
                                });
                                life.last_request = Some((key.clone(), tick.raw()));
                                self.metrics.requests += 1;
                                self.metrics.unusable_hint_retry_requests += 1;
                                life.input_status = PriorInputStatus::Pending;
                            }
                            Err(error) => {
                                record_provider_failure(&mut self.metrics, id.raw(), tick.raw(), &key,
                                    format!("local SLM prior unusable-hint retry submit failed: {error:?}"));
                                life.input_status = PriorInputStatus::ProviderFailure;
                                self.retry_not_before =
                                    Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                            }
                        }
                    }
                    return Ok(draft);
                }
                let slots: Vec<u16> = BASIC_VOCABULARY_V1
                    .iter()
                    .filter_map(|(_, code)| {
                        (lexicon_slot_salience(output, *code) > 0.0).then_some(*code)
                    })
                    .collect();
                if !slots.is_empty() {
                    let packet = life.controller.issue_packet(
                        SemanticPriorRequest::new(id, sequence)?,
                        tick,
                        slots,
                        false,
                    )?;
                    if self.metrics.deliveries.len() >= 256 {
                        self.metrics.deliveries.pop_front();
                    }
                    self.metrics.deliveries.push_back(PriorDelivery {
                        organism: id.raw(),
                        sequence: sequence.raw(),
                        issued_tick: tick.raw(),
                        expires_tick: packet.expires_at_tick.raw(),
                        context_digest: blake3::hash(key.as_bytes()).to_hex().to_string(),
                        output_digest: blake3::hash(
                            &serde_json::to_vec(output)
                                .map_err(|_| alife_core::ScaffoldContractError::InvalidId)?,
                        )
                        .to_hex()
                        .to_string(),
                        gain: packet.plasticity_modulation,
                    });
                    life.active = Some((key.clone(), packet, output.clone()));
                    self.metrics.cache_hits += 1;
                } else if consume {
                    self.metrics.empty_hint_frames += 1;
                    life.input_status = PriorInputStatus::EmptyHints;
                }
            } else if self.queue.is_some()
                && !self.frozen_recording
                && life.pending.is_none()
                && self.retry_not_before.is_none_or(|at| Instant::now() >= at)
                && life.last_request.as_ref().is_none_or(|(context, at)| {
                    context != &key || tick.raw().saturating_sub(*at) >= 128
                })
            {
                let request = LocalSlmPriorRequest {
                    request_id: self.next_request,
                    prompt: key.clone(),
                };
                match self
                    .queue
                    .as_ref()
                    .expect("live prior queue checked")
                    .submit(request)
                {
                    Ok(reply) => {
                        self.next_request = self
                            .next_request
                            .checked_add(1)
                            .ok_or(alife_core::ScaffoldContractError::InvalidId)?;
                        life.pending = Some(Pending {
                            key: key.clone(),
                            reply,
                            unusable_hint_feedback: false,
                            submitted_at: Instant::now(),
                        });
                        life.last_request = Some((key.clone(), tick.raw()));
                        self.metrics.requests += 1;
                        life.input_status = PriorInputStatus::Pending;
                    }
                    Err(error) => {
                        record_provider_failure(
                            &mut self.metrics,
                            id.raw(),
                            tick.raw(),
                            &key,
                            format!("local SLM prior submit failed: {error:?}"),
                        );
                        life.input_status = PriorInputStatus::ProviderFailure;
                        self.retry_not_before = Some(Instant::now() + PROVIDER_FAILURE_COOLDOWN);
                    }
                }
            }
        }
        if self.frozen_recording
            && life.active.is_none()
            && !self.cache.contains_key(&key)
            && self.reported_empty_contexts.len() < MAX_UNUSABLE_RETRY_CONTEXTS
            && self.reported_empty_contexts.insert(key.clone())
        {
            record_provider_failure(
                &mut self.metrics,
                id.raw(),
                tick.raw(),
                &key,
                "recorded prior has no exact current-context entry".into(),
            );
            life.input_status = PriorInputStatus::Unavailable;
        }
        let context = life.active.as_ref().map(|(_, packet, output)| {
            if consume {
                self.metrics.delivered_frames += 1;
            }
            life.input_status = if packet.plasticity_modulation > 0.0 {
                PriorInputStatus::Delivered
            } else {
                PriorInputStatus::DevelopmentalGainZero
            };
            SemanticContextRef {
                feature_flags: ContextFeatureFlags::NONE,
                confidence: Confidence(packet.plasticity_modulation),
                compressed_codes: packet
                    .lexicon_bias_slots
                    .iter()
                    .map(|slot| CompressedSemanticCode {
                        codebook_id: 1,
                        code: u32::from(*slot),
                        salience: NormalizedScalar(lexicon_slot_salience(output, *slot)),
                    })
                    .collect(),
                salience: Vec::new(),
            }
        });
        if consume {
            life.controller.record_relevant_exposure(context.is_some());
        }
        draft.with_semantic_context(context)
    }
    #[cfg(all(test, feature = "gpu-tests"))]
    pub(super) fn seed_resume_check(&mut self, id: u64, tick: u64) {
        let life = self.lives.entry(id).or_default();
        for _ in 0..256 {
            life.controller.record_relevant_exposure(false);
        }
        for _ in 0..3 {
            life.controller.record_unaided_probe(1.0).unwrap();
        }
        assert_eq!(life.controller.developmental_gain(), 0.0);
        life.recently_heard = Some((tick, vec!["play".into(), "ball".into()]));
        let packet = life
            .controller
            .issue_packet(
                SemanticPriorRequest::new(alife_core::OrganismId(id), ExperienceSequenceId(1))
                    .unwrap(),
                alife_core::Tick(tick),
                vec![13, 14],
                false,
            )
            .unwrap();
        let output = LocalSlmPriorOutput {
            schema: "alife.ca27.local_slm_prior_output.v1".into(),
            schema_version: 1,
            model: self.metrics.model.clone(),
            salience_labels: vec!["toy".into()],
            context_summary: "play ball".into(),
            lexicon_associations: vec![alife_semantic::SlmLexiconAssociation {
                token: "play".into(),
                salience: 0.5,
            }],
            perception_tags: vec!["near".into()],
            can_issue_actions: false,
            can_rewrite_weights: false,
            can_bypass_arbitration: false,
            hidden_vector_injection: false,
            bounded_context_only: true,
        };
        output.validate().unwrap();
        life.active = Some(("play ball".into(), packet, output));
    }
    pub fn snapshot(&self, id: u64) -> Result<Option<Vec<u8>>, alife_core::ScaffoldContractError> {
        let Some(life) = self.lives.get(&id) else {
            return Ok(None);
        };
        let state = PriorResume {
            schema: 1,
            provider: self.metrics.provider_identity.clone(),
            controller: life.controller.clone(),
            active: life.active.clone(),
            last_request: life.last_request.clone(),
            recently_heard: life.recently_heard.clone(),
            next_request: self.next_request,
            pending_request: life.pending.is_some(),
        };
        let bytes = serde_json::to_vec(&state)
            .map_err(|_| alife_core::ScaffoldContractError::InvalidSparseProjectionSchema)?;
        if bytes.len() > 16_384 {
            return Err(alife_core::ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Some(bytes))
    }
    pub fn restore_life(
        &mut self,
        id: u64,
        tick: u64,
        bytes: &[u8],
    ) -> Result<(), alife_core::ScaffoldContractError> {
        use alife_core::Validate;
        let invalid = || alife_core::ScaffoldContractError::InvalidSparseProjectionSchema;
        if bytes.len() > 16_384 {
            return Err(invalid());
        }
        let state: PriorResume = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        if state.schema != 1
            || state.provider.is_empty()
            || state.provider.len() > 256
            || state.next_request == 0
            || state.controller.consecutive_passing_probes()
                > alife_semantic::PASSING_PROBES_TO_ZERO
            || state
                .last_request
                .as_ref()
                .is_some_and(|(k, t)| k.len() > 768 || *t > tick)
            || state
                .recently_heard
                .as_ref()
                .is_some_and(|(t, w)| *t > tick || w.len() > 6 || w.iter().any(|w| w.len() > 24))
        {
            return Err(invalid());
        }
        state.controller.validate_resume(alife_core::Tick(tick))?;
        if let Some((key, packet, output)) = &state.active {
            packet.validate_contract()?;
            output.validate()?;
            if key.len() > 768
                || packet.issued_at_tick.raw() > tick
                || packet.request.organism_id.raw() != id
            {
                return Err(invalid());
            }
        }
        let provider_changed = state.provider != self.metrics.provider_identity;
        if provider_changed {
            // Swapping an optional service drops only its validated old hints,
            // never the organism/optimizer/checkpoint state or developmental gain.
            record_provider_failure(
                &mut self.metrics,
                id,
                tick,
                "provider swap",
                "semantic prior provider changed; discarded previous provider hints".into(),
            );
        }
        self.next_request = self.next_request.max(state.next_request);
        // In-flight network work is resubmitted; compatible active hints and fade state survive.
        self.lives.insert(
            id,
            Life {
                controller: state.controller,
                pending: None,
                active: if provider_changed { None } else { state.active },
                last_request: if state.pending_request || provider_changed {
                    None
                } else {
                    state.last_request
                },
                recently_heard: state.recently_heard,
                last_context: String::new(),
                input_status: PriorInputStatus::Unavailable,
            },
        );
        Ok(())
    }
    pub fn retain_lives<T>(&mut self, live: &std::collections::BTreeMap<u64, T>) {
        self.lives.retain(|id, _| live.contains_key(id));
    }
}

fn selected_backend(training: bool, explicit: Option<String>, legacy_off: bool) -> String {
    if legacy_off {
        "off".into()
    } else {
        explicit.unwrap_or_else(|| {
            if training {
                "deterministic".into()
            } else {
                "local".into()
            }
        })
    }
}

fn context_builder_source(source: &str) -> &str {
    let start = source
        .rfind("\nfn bounded_context(")
        .expect("context builder exists");
    let end = source[start..]
        .find("\nfn lexicon_slot_salience(")
        .expect("context builder end exists")
        + start;
    &source[start..end]
}

pub(super) fn context_contract() -> String {
    // Fingerprint the actual bounded observation builder plus vocabulary. Queue,
    // provider and failure-policy changes do not redefine request semantics.
    format!(
        "grounded-terrain-language-prior-v1:{}:{}",
        blake3::hash(context_builder_source(include_str!("semantic_prior.rs")).as_bytes()).to_hex(),
        digest_json(&BASIC_VOCABULARY_V1)
    )
}

fn load_live_cache(
    path: &std::path::Path,
    expected_model: &str,
    error: &mut Option<String>,
) -> BTreeMap<String, LocalSlmPriorOutput> {
    let loaded = (|| {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(e) => return Err(format!("prior cache unavailable: {e}")),
        };
        if metadata.len() > 256 * 1024 {
            return Err("prior cache exceeds byte bound".into());
        }
        let bytes = std::fs::read(path).map_err(|e| format!("prior cache read failed: {e}"))?;
        let cache: BTreeMap<String, LocalSlmPriorOutput> =
            serde_json::from_slice(&bytes).map_err(|e| format!("prior cache parse failed: {e}"))?;
        if cache.len() > 128
            || cache.iter().any(|(key, output)| {
                LocalSlmPriorRequest {
                    request_id: 1,
                    prompt: key.clone(),
                }
                .validate(768)
                .is_err()
                    || output.validate().is_err()
                    || output.model != expected_model
            })
        {
            return Err("prior cache failed bounded output validation".into());
        }
        Ok(cache)
    })();
    match loaded {
        Ok(cache) => cache,
        Err(e) => {
            *error = Some(e);
            BTreeMap::new()
        }
    }
}

fn heard_words(draft: &PerceptionFrameDraft) -> Vec<String> {
    draft
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .take(6)
        .map(|h| {
            BASIC_VOCABULARY_V1
                .iter()
                .find(|(_, code)| u32::from(*code) == h.token_id)
                .map_or_else(
                    || format!("symbol{}", h.token_id),
                    |(word, _)| word.to_string(),
                )
        })
        .collect()
}

fn bounded_context(draft: &PerceptionFrameDraft, words: &[String]) -> String {
    // Coarse grounded features only; no entity IDs, GPS, legal choices, weights,
    // hidden food locations or teacher goal are available to the provider.
    let mut context = format!(
        "heard words {}; hunger {}; tiredness {}; ",
        words.join(" "),
        if draft.homeostasis().drives.hunger > 0.5 {
            "high"
        } else {
            "low"
        },
        if draft.homeostasis().drives.fatigue > 0.3 {
            "high"
        } else {
            "low"
        }
    );
    context.push_str(&format!(
        "smells chemical levels {:.1},{:.1},{:.1}; feels contact pressure {:.1} grip {:.1}; ",
        draft.sensory().channels.smell_chemistry[0],
        draft.sensory().channels.smell_chemistry[1],
        draft.sensory().channels.smell_chemistry[2],
        draft.sensory().channels.tactile_contact[0],
        draft.sensory().channels.tactile_contact[2]
    ));
    for slot in draft.grounded_object_slots().iter().take(2) {
        context.push_str(&format!("sees object color {:.1},{:.1},{:.1} shape {:.1},{:.1},{:.1} chemical {:.1},{:.1},{:.1}; ",
            slot.color[0], slot.color[1], slot.color[2], slot.shape[0], slot.shape[1], slot.shape[2],
            slot.chemical[0], slot.chemical[1], slot.chemical[2]));
    }
    if draft
        .sensory()
        .channels
        .visual_affordance
        .iter()
        .any(|range| *range < 0.15)
    {
        context.push_str("sees nearby terrain surface");
    }
    context
}

fn lexicon_slot_salience(output: &LocalSlmPriorOutput, slot: u16) -> f32 {
    // Readiness and conversion use the same order-independent bounded reduction.
    // Duplicate rows cannot let a zero association mask a usable hint.
    BASIC_VOCABULARY_V1
        .iter()
        .find(|(_, code)| *code == slot)
        .map_or(0.0, |(word, _)| {
            output
                .lexicon_associations
                .iter()
                .filter(|association| association.token == *word)
                .map(|association| association.salience)
                .fold(0.0, f32::max)
        })
}

fn has_usable_hints(output: &LocalSlmPriorOutput) -> bool {
    BASIC_VOCABULARY_V1
        .iter()
        .any(|(_, code)| lexicon_slot_salience(output, *code) > 0.0)
}

fn digest_json(value: &impl serde::Serialize) -> String {
    // Only validated typed runtime records are serialized here.
    blake3::hash(&serde_json::to_vec(value).expect("typed prior receipt serializes"))
        .to_hex()
        .to_string()
}

fn push_bounded<T>(receipts: &mut VecDeque<T>, receipt: T) {
    if receipts.len() >= MAX_RECEIPTS {
        receipts.pop_front();
    }
    receipts.push_back(receipt);
}

fn record_provider_failure(
    metrics: &mut SemanticPriorMetrics,
    organism: u64,
    tick: u64,
    key: &str,
    error: String,
) {
    let error: String = error.chars().take(512).collect();
    eprintln!("warning: semantic prior unavailable; continuing unaided: {error}");
    metrics.failures += 1;
    metrics.last_error = Some(error.clone());
    push_bounded(
        &mut metrics.provider_failures,
        PriorProviderFailure {
            organism,
            tick,
            context_digest: blake3::hash(key.as_bytes()).to_hex().to_string(),
            error,
        },
    );
}

#[cfg(all(test, feature = "foundation-training"))]
mod tests {
    use super::*;
    use alife_core::{
        HomeostaticSnapshot, OrganismId, PerceptionContextBlock, SensorProfile, Tick, Vec3f,
    };

    fn draft() -> PerceptionFrameDraft {
        let mut world = alife_world::HeadlessScenarioBuilder::new(77)
            .agent("learner", OrganismId(5), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        world
            .perception_frame_draft(
                OrganismId(5),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap()
    }
    fn output() -> LocalSlmPriorOutput {
        LocalSlmPriorOutput {
            schema: "alife.ca27.local_slm_prior_output.v1".into(),
            schema_version: 1,
            model: "test-prior".into(),
            salience_labels: vec!["near".into()],
            context_summary: "nearby observation".into(),
            lexicon_associations: vec![alife_semantic::SlmLexiconAssociation {
                token: "eat".into(),
                salience: 0.5,
            }],
            perception_tags: vec!["near".into()],
            can_issue_actions: false,
            can_rewrite_weights: false,
            can_bypass_arbitration: false,
            hidden_vector_injection: false,
            bounded_context_only: true,
        }
    }
    struct FixtureProvider;
    impl alife_semantic::BoundedSlmPriorProvider for FixtureProvider {
        fn generate_prior(&self, _: &str, _: bool) -> Result<LocalSlmPriorOutput, String> {
            Err("CPU fixture provider unavailable".into())
        }
    }
    fn prior() -> RuntimeSemanticPrior {
        RuntimeSemanticPrior {
            queue: Some(
                LocalSlmPriorAsyncQueue::with_provider(
                    LlamaCppSlmPriorConfig::default(),
                    Box::new(FixtureProvider),
                )
                .unwrap(),
            ),
            retry_not_before: None,
            reported_empty_contexts: BTreeSet::new(),
            frozen_recording: false,
            deterministic: None,
            lives: BTreeMap::new(),
            cache: BTreeMap::new(),
            unusable_retry_contexts: BTreeSet::new(),
            next_request: 1,
            cache_path: std::env::temp_dir().join("alife-semantic-receipt-unused-cache.json"),
            dropout_seed: Some(5),
            metrics: SemanticPriorMetrics {
                provider_identity: "synthetic-cpu-provider".into(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn rich_lesson_disables_dropout_and_only_dispatch_creates_input_receipt() {
        let draft = draft();
        let mut prior = prior();
        let key = bounded_context(&draft, &[]);
        prior.cache.insert(key, output());
        assert!(prior
            .prepare(draft.clone(), ExperienceSequenceId(1))
            .unwrap()
            .sensory()
            .semantic_context
            .is_none());
        assert_eq!(prior.metrics.dropout_frames, 1);
        prior.require_rich_information();
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        assert!(prior.ready(5));
        assert_eq!(prior.metrics.delivered_frames, 0);
        assert_eq!(prior.metrics.decision_frames, 0);
        let prepared = prior.prepare(draft, ExperienceSequenceId(1)).unwrap();
        assert!(prepared.sensory().semantic_context.is_some());
        assert_eq!(prior.metrics.delivered_frames, 1);
        assert_eq!(prior.metrics.decision_frames, 0);
        let frame = prepared.finalize(PerceptionContextBlock::empty()).unwrap();
        prior.record_decision_input(&frame, 1, 7, 128, 8);
        let receipt = prior.metrics.decision_inputs.back().unwrap();
        assert_eq!(receipt.status, PriorInputStatus::Delivered);
        assert_eq!((receipt.sequence, receipt.dispatch_generation), (1, 7));
        assert!(receipt.nonzero_prior_lanes > 0);
        assert!(receipt.natural_senses_context.contains("smells chemical"));
        assert!(receipt.natural_senses_context.contains("feels contact"));
        assert!(receipt.context_digest.is_some());
        assert_eq!(prior.metrics.decision_frames_with_prior, 1);
        prior.record_decision_input(&frame, 2, 8, 0, 0);
        assert_eq!(
            prior.metrics.decision_inputs.back().unwrap().status,
            PriorInputStatus::NoSemanticEncoder
        );
        assert_eq!(prior.metrics.decision_frames_without_prior, 1);
    }

    #[test]
    fn duplicate_lexicon_rows_keep_ready_hints_nonzero_and_delivered() {
        let capacity = alife_core::BrainCapacityClass::n2048();
        let mut genome = alife_core::BrainGenome::scaffold(77_112, capacity.id());
        // The minimal scaffold has no Hearing gene. Native private language/prior
        // ports are compiled only for hearing-enabled organisms, as in a founder.
        genome
            .sensor_layout
            .channels
            .push(alife_core::SensorChannelGene {
                kind: alife_core::SensorChannelKind::Hearing,
                receptor_count: 32,
                target_lobe: alife_core::LobeKind::PerceptualIntegration,
                enabled_at_maturation: 0,
            });
        let development =
            alife_core::DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(1.0));
        let phenotype = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )
        .unwrap();
        for saliences in [[0.0, 0.5], [0.25, 0.5], [0.5, 0.25], [0.5, 0.0]] {
            let draft = draft();
            let mut prior = prior();
            prior.require_rich_information();
            let mut output = output();
            output.lexicon_associations = saliences
                .into_iter()
                .map(|salience| alife_semantic::SlmLexiconAssociation {
                    token: "eat".into(),
                    salience,
                })
                .collect();
            output.validate().unwrap();
            prior.cache.insert(bounded_context(&draft, &[]), output);
            prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
            assert!(prior.ready(5));
            let prepared = prior.prepare(draft, ExperienceSequenceId(1)).unwrap();
            let context = prepared.sensory().semantic_context.as_ref().unwrap();
            assert_eq!(context.compressed_codes.len(), 1);
            assert_eq!(context.compressed_codes[0].salience.raw(), 0.5);
            let frame = prepared.finalize(PerceptionContextBlock::empty()).unwrap();
            let lanes = frame.sensory().language_prior_neural_lanes();
            assert!(lanes[128..].iter().any(|lane| *lane != 0.0));
            let assignments =
                phenotype
                    .sensor_encoder()
                    .assignments()
                    .iter()
                    .filter(|assignment| {
                        assignment.source_group()
                            == alife_core::SensorEncoderSourceGroup::SemanticPrior
                    });
            let semantic_encoder_lanes = assignments.clone().count();
            assert_eq!(semantic_encoder_lanes, 128);
            let nonzero_encoded_prior_lanes = assignments
                .filter(|assignment| {
                    let source = lanes[128 + usize::from(assignment.source_index())];
                    let (low, high) = assignment.clamp_range();
                    (source * assignment.scale() + assignment.bias()).clamp(low, high)
                        != assignment.bias().clamp(low, high)
                })
                .count();
            assert!(nonzero_encoded_prior_lanes > 0);
            prior.record_decision_input(
                &frame,
                1,
                7,
                semantic_encoder_lanes,
                nonzero_encoded_prior_lanes,
            );
            let receipt = prior.metrics.decision_inputs.back().unwrap();
            assert_eq!(receipt.status, PriorInputStatus::Delivered);
            assert!(receipt.nonzero_encoded_prior_lanes > 0);
            assert_eq!(prior.metrics.decision_frames_with_prior, 1);
            assert_eq!(prior.metrics.decision_frames_without_prior, 0);
        }
    }

    #[test]
    fn ready_cached_hint_cannot_count_as_delivery_on_an_empty_frame() {
        let draft = draft();
        let mut prior = prior();
        prior.require_rich_information();
        prior.cache.insert(bounded_context(&draft, &[]), output());
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        assert!(prior.ready(5));
        let frame = draft.finalize(PerceptionContextBlock::empty()).unwrap();
        prior.record_decision_input(&frame, 1, 7, 128, 0);
        assert_eq!(prior.metrics.decision_frames_with_prior, 0);
        assert_eq!(prior.metrics.decision_frames_without_prior, 1);
        assert_eq!(
            prior.metrics.decision_inputs.back().unwrap().status,
            PriorInputStatus::EmptyHints
        );
    }

    #[test]
    fn current_active_hint_survives_resume_without_cache_or_prime_delivery() {
        let draft = draft();
        let mut prior = prior();
        prior.require_rich_information();
        prior.cache.insert(bounded_context(&draft, &[]), output());
        let prepared = prior
            .prepare(draft.clone(), ExperienceSequenceId(1))
            .unwrap();
        let gain = prepared
            .sensory()
            .semantic_context
            .as_ref()
            .unwrap()
            .confidence;
        let bytes = prior.snapshot(5).unwrap().unwrap();
        let mut restored = self::prior();
        restored.require_rich_information();
        restored.restore_life(5, 0, &bytes).unwrap();
        assert!(restored.cache.is_empty());
        restored
            .prime(draft.clone(), ExperienceSequenceId(2))
            .unwrap();
        assert!(restored.ready(5));
        assert_eq!(restored.metrics.delivered_frames, 0);
        assert_eq!(restored.metrics.decision_frames, 0);
        let resumed = restored.prepare(draft, ExperienceSequenceId(2)).unwrap();
        assert_eq!(
            resumed
                .sensory()
                .semantic_context
                .as_ref()
                .unwrap()
                .confidence,
            gain
        );
    }

    #[test]
    fn provider_failure_and_disconnection_remain_visible() {
        for disconnected in [false, true] {
            let draft = draft();
            let key = bounded_context(&draft, &[]);
            let mut prior = prior();
            prior.require_rich_information();
            let (tx, rx) = std::sync::mpsc::channel();
            if !disconnected {
                tx.send(Err("provider unavailable".into())).unwrap();
            }
            drop(tx);
            let life = prior.lives.entry(5).or_default();
            life.pending = Some(Pending {
                key: key.clone(),
                reply: rx,
                unusable_hint_feedback: false,
                submitted_at: Instant::now(),
            });
            life.last_request = Some((key, 0));
            let prepared = prior.prepare(draft, ExperienceSequenceId(1)).unwrap();
            assert!(!prior.ready(5));
            let frame = prepared.finalize(PerceptionContextBlock::empty()).unwrap();
            prior.record_decision_input(&frame, 1, 7, 128, 0);
            assert_eq!(prior.metrics.failures, 1);
            assert!(prior.metrics.last_error.is_some());
            assert_eq!(prior.metrics.provider_failures.len(), 1);
            assert_eq!(
                prior.metrics.decision_inputs.back().unwrap().status,
                PriorInputStatus::ProviderFailure
            );
            let retry = prior
                .prepare(self::draft(), ExperienceSequenceId(2))
                .unwrap();
            let retry_frame = retry.finalize(PerceptionContextBlock::empty()).unwrap();
            prior.record_decision_input(&retry_frame, 2, 8, 128, 0);
            assert_eq!(
                prior.metrics.decision_inputs.back().unwrap().status,
                PriorInputStatus::ProviderFailure
            );
            assert_eq!(prior.metrics.failures, 1);
        }
    }

    #[test]
    fn empty_hints_are_not_readiness_and_receipts_are_bounded() {
        let draft = draft();
        let mut prior = prior();
        prior.require_rich_information();
        let mut empty = output();
        empty.lexicon_associations[0].salience = 0.0;
        empty.validate().unwrap();
        prior.cache.insert(bounded_context(&draft, &[]), empty);
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        assert!(!prior.ready(5));
        let frame = prior
            .prepare(draft, ExperienceSequenceId(1))
            .unwrap()
            .finalize(PerceptionContextBlock::empty())
            .unwrap();
        for sequence in 1..=300 {
            prior.record_decision_input(&frame, sequence, sequence, 128, 0);
        }
        assert_eq!(prior.metrics.decision_frames_without_prior, 300);
        assert_eq!(prior.metrics.decision_inputs.len(), MAX_RECEIPTS);
        assert_eq!(prior.metrics.decision_inputs.front().unwrap().sequence, 45);
        assert_eq!(
            prior.metrics.decision_inputs.back().unwrap().status,
            PriorInputStatus::EmptyHints
        );
    }

    fn complete_pending(prior: &mut RuntimeSemanticPrior, result: LocalSlmPriorOutput) {
        let life = prior.lives.get_mut(&5).unwrap();
        let pending = life.pending.take().expect("one retry was queued");
        let (send, receive) = std::sync::mpsc::channel();
        send.send(Ok(result)).unwrap();
        life.pending = Some(Pending {
            key: pending.key,
            reply: receive,
            unusable_hint_feedback: pending.unusable_hint_feedback,
            submitted_at: Instant::now(),
        });
    }

    #[test]
    fn rich_zero_hint_cache_retries_same_context_once_and_accepts_only_usable_reply() {
        let draft = draft();
        let key = bounded_context(&draft, &[]);
        let mut prior = prior();
        prior.require_rich_information();
        let mut zero = output();
        zero.lexicon_associations[0].salience = 0.0;
        prior.cache.insert(key.clone(), zero.clone());
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        assert!(prior.pending(5));
        assert!(!prior.ready(5));
        assert_eq!(prior.metrics.unusable_hint_retry_requests, 1);
        assert_eq!(prior.metrics.unusable_hint_retries[0].context, key);
        assert_eq!(prior.metrics.unusable_hint_retries[0].previous_output, zero);
        complete_pending(&mut prior, output());
        prior.prime(draft, ExperienceSequenceId(1)).unwrap();
        assert!(!prior.pending(5));
        assert!(prior.ready(5));
        prior.record_priming_status(5, 0, "no_pending");
        let status = prior.metrics.last_priming.as_ref().unwrap();
        assert_eq!(status.context, key);
        assert!(status.ready && status.cache_match && status.unusable_retry_used);
        assert_eq!(status.cache_usable_hints, 1);
        assert!(status.developmental_gain > 0.0);
    }

    #[test]
    fn honest_second_zero_stays_unready_without_repeated_requests() {
        let draft = draft();
        let mut prior = prior();
        prior.require_rich_information();
        let mut zero = output();
        zero.lexicon_associations[0].salience = 0.0;
        prior
            .cache
            .insert(bounded_context(&draft, &[]), zero.clone());
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        complete_pending(&mut prior, zero.clone());
        for _ in 0..3 {
            prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        }
        assert!(!prior.pending(5));
        assert!(!prior.ready(5));
        assert_eq!(prior.metrics.unusable_hint_retry_requests, 1);
        prior.record_priming_status(5, 0, "no_pending");
        let status = prior.metrics.last_priming.as_ref().unwrap();
        assert_eq!(status.status, PriorInputStatus::EmptyHints);
        assert_eq!(status.cache_output.as_ref().unwrap(), &zero);
        assert_eq!(status.cache_usable_hints, 0);
        assert!(!status.ready);
    }

    #[test]
    fn ordinary_zero_hint_cache_remains_valid_without_training_retry() {
        let draft = draft();
        let mut prior = prior();
        prior.dropout_seed = None;
        let mut zero = output();
        zero.lexicon_associations[0].salience = 0.0;
        prior.cache.insert(bounded_context(&draft, &[]), zero);
        prior.prime(draft, ExperienceSequenceId(1)).unwrap();
        assert!(!prior.pending(5));
        assert!(!prior.ready(5));
        assert_eq!(prior.metrics.unusable_hint_retry_requests, 0);
    }
    #[test]
    fn ordinary_teacher_hearing_is_independent_of_private_lexicon_associations() {
        let learner = OrganismId(5);
        let mut world = alife_world::HeadlessScenarioBuilder::new(77)
            .agent("learner", learner, Vec3f::ZERO)
            .social_agent("teacher", OrganismId(6), Vec3f::new(-2.0, 0.0, 0.0), 0.75)
            .food("food", Vec3f::new(3.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        world
            .grounded_teacher_actor(world.entity_id("teacher").unwrap())
            .unwrap()
            .speak(
                &mut world,
                Some(learner),
                vec![alife_core::LanguageTokenId::new(1).unwrap()],
                alife_core::TeacherPerceptionChannel::Hearing,
            )
            .unwrap();
        let draft = world
            .perception_frame_draft(
                learner,
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        let baseline_lanes = draft.sensory().language_prior_neural_lanes();
        assert!(baseline_lanes[..128].iter().any(|lane| *lane != 0.0));
        assert!(baseline_lanes[128..].iter().all(|lane| *lane == 0.0));
        assert!(!draft.grounded_object_slots().is_empty());
        let heard = draft
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].token_id, 1);
        assert_eq!(
            heard[0].source_kind,
            alife_core::UtteranceSourceKind::Teacher
        );
        assert_eq!(
            heard[0].teacher_channel,
            Some(alife_core::TeacherPerceptionChannel::Hearing)
        );

        let positive = output();
        // An empty list is invalid in this schema. The valid zero-hint control
        // retains the same vocabulary row with zero salience.
        let mut invalid_empty = positive.clone();
        invalid_empty.lexicon_associations.clear();
        assert!(invalid_empty.validate().is_err());
        let mut zero = positive.clone();
        zero.lexicon_associations[0].salience = 0.0;
        zero.validate().unwrap();
        positive.validate().unwrap();
        assert_eq!(zero.salience_labels, positive.salience_labels);
        assert_eq!(zero.context_summary, positive.context_summary);
        assert_eq!(zero.perception_tags, positive.perception_tags);
        assert!(!has_usable_hints(&zero));
        assert!(has_usable_hints(&positive));

        for (case, output, expected_ready, expected_codes) in [
            ("zero_salience_with_tags_summary", zero, false, 0),
            ("positive_receiver_association", positive, true, 1),
        ] {
            let mut prior = prior();
            // Exercise conversion and readiness without priming, a provider
            // request, a training loop, or synthetic encoder-delivery counts.
            prior.dropout_seed = None;
            prior.cache.insert(
                bounded_context(&draft, &heard_words(&draft)),
                output.clone(),
            );
            let prepared = prior
                .prepare(draft.clone(), ExperienceSequenceId(1))
                .unwrap();
            assert_eq!(prior.ready(learner.raw()), expected_ready);
            assert_eq!(prior.metrics.requests, 0);
            assert_eq!(prior.metrics.unusable_hint_retry_requests, 0);
            assert_eq!(prior.metrics.decision_frames, 0);
            assert!(prior.metrics.decision_inputs.is_empty());
            assert_eq!(
                prepared.grounded_object_slots(),
                draft.grounded_object_slots()
            );
            assert_eq!(
                prepared.sensory().language_context,
                draft.sensory().language_context
            );
            assert_eq!(prepared.sensory().channels, draft.sensory().channels);
            let frame = prepared.finalize(PerceptionContextBlock::empty()).unwrap();
            let lanes = frame.sensory().language_prior_neural_lanes();
            assert_eq!(&lanes[..128], &baseline_lanes[..128]);
            let nonzero_private_lanes = lanes[128..].iter().filter(|lane| **lane != 0.0).count();
            assert_eq!(nonzero_private_lanes, expected_codes * 8);
            assert_eq!(
                frame
                    .sensory()
                    .semantic_context
                    .as_ref()
                    .map_or(0, |context| { context.compressed_codes.len() }),
                expected_codes
            );
            if expected_codes == 1 {
                let context = frame.sensory().semantic_context.as_ref().unwrap();
                assert!(context.confidence.raw() > 0.0);
                assert_eq!(context.compressed_codes[0].salience.raw(), 0.5);
            } else {
                assert!(frame.sensory().semantic_context.is_none());
            }
            println!(
                "{}",
                serde_json::json!({
                    "case": case,
                    "schema_valid": true,
                    "association_count": output.lexicon_associations.len(),
                    "association_salience": output.lexicon_associations[0].salience,
                    "teacher_heard_token": heard[0].token_id,
                    "nonzero_hearing_lanes": lanes[..128].iter().filter(|lane| **lane != 0.0).count(),
                    "hearing_bank_identical": true,
                    "grounded_object_slots_identical": true,
                    "grounded_object_slots": frame.grounded_object_slots().len(),
                    "nonzero_private_prior_lanes": nonzero_private_lanes,
                    "ready": prior.ready(learner.raw()),
                    "provider_requests": prior.metrics.requests,
                    "gpu_dispatches": 0,
                    "encoder_delivery_tested": false,
                    "comprehension_tested": false
                })
            );
        }
    }

    fn teacher_draft() -> PerceptionFrameDraft {
        let learner = OrganismId(5);
        let mut world = alife_world::HeadlessScenarioBuilder::new(77)
            .agent("learner", learner, Vec3f::ZERO)
            .social_agent("teacher", OrganismId(6), Vec3f::new(-2.0, 0.0, 0.0), 0.75)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        world
            .grounded_teacher_actor(world.entity_id("teacher").unwrap())
            .unwrap()
            .speak(
                &mut world,
                Some(learner),
                vec![alife_core::LanguageTokenId::new(1).unwrap()],
                alife_core::TeacherPerceptionChannel::Hearing,
            )
            .unwrap();
        world
            .perception_frame_draft(
                learner,
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap()
    }

    fn pending_result(
        prior: &mut RuntimeSemanticPrior,
        key: &str,
        result: Option<Result<LocalSlmPriorOutput, String>>,
        timeout: bool,
    ) -> Option<std::sync::mpsc::Sender<Result<LocalSlmPriorOutput, String>>> {
        let (send, receive) = std::sync::mpsc::channel();
        if let Some(result) = result {
            send.send(result).unwrap();
        }
        let life = prior.lives.entry(5).or_default();
        life.pending = Some(Pending {
            key: key.into(),
            reply: receive,
            unusable_hint_feedback: false,
            submitted_at: Instant::now()
                - if timeout {
                    Duration::from_secs(300)
                } else {
                    Duration::ZERO
                },
        });
        life.last_request = Some((key.into(), 0));
        Some(send)
    }

    #[test]
    fn every_provider_failure_preserves_ordinary_hearing_and_can_recover() {
        for case in [
            "provider",
            "parse",
            "timeout",
            "disconnect",
            "invalid_schema",
            "nonfinite_hint",
            "forbidden_action",
        ] {
            let draft = teacher_draft();
            let key = bounded_context(&draft, &heard_words(&draft));
            let mut prior = prior();
            prior.require_rich_information();
            let result = match case {
                "provider" => Some(Err("provider unavailable".into())),
                "parse" => Some(Err("invalid provider JSON".into())),
                "timeout" | "disconnect" => None,
                _ => {
                    let mut invalid = output();
                    match case {
                        "invalid_schema" => invalid.schema_version = 0,
                        "nonfinite_hint" => invalid.lexicon_associations[0].salience = f32::NAN,
                        "forbidden_action" => invalid.can_issue_actions = true,
                        _ => unreachable!(),
                    }
                    Some(Ok(invalid))
                }
            };
            let sender = pending_result(&mut prior, &key, result, case == "timeout");
            if case == "disconnect" {
                drop(sender);
            }
            let prepared = prior
                .prepare(draft.clone(), ExperienceSequenceId(1))
                .unwrap();
            assert_eq!(
                prepared.sensory().language_context,
                draft.sensory().language_context,
                "{case}"
            );
            assert_eq!(
                prepared.sensory().channels,
                draft.sensory().channels,
                "{case}"
            );
            assert_eq!(
                prepared.grounded_object_slots(),
                draft.grounded_object_slots(),
                "{case}"
            );
            assert!(prepared.sensory().semantic_context.is_none(), "{case}");
            assert_eq!(prior.metrics.failures, 1, "{case}");
            assert_eq!(prior.metrics.provider_failures.len(), 1);
            let frame = prepared.finalize(PerceptionContextBlock::empty()).unwrap();
            prior.record_decision_input(&frame, 1, 1, 128, 0);
            assert_eq!(prior.metrics.decision_frames_without_prior, 1);
            assert_eq!(prior.metrics.decision_frames_with_heard_language, 1);
            // The same context can recover later without admitting stale hints.
            pending_result(&mut prior, &key, Some(Ok(output())), false);
            let recovered = prior.prepare(draft, ExperienceSequenceId(2)).unwrap();
            assert!(recovered.sensory().semantic_context.is_some(), "{case}");
        }
    }

    #[test]
    fn changed_context_failure_does_not_reuse_previous_active_prior() {
        let mut prior = prior();
        prior.require_rich_information();
        let original = draft();
        prior
            .cache
            .insert(bounded_context(&original, &[]), output());
        assert!(prior
            .prepare(original, ExperienceSequenceId(1))
            .unwrap()
            .sensory()
            .semantic_context
            .is_some());
        let changed = teacher_draft();
        let key = bounded_context(&changed, &heard_words(&changed));
        pending_result(
            &mut prior,
            &key,
            Some(Err("changed context provider failed".into())),
            false,
        );
        let unaided = prior.prepare(changed, ExperienceSequenceId(2)).unwrap();
        assert!(unaided.sensory().semantic_context.is_none());
        assert!(prior.lives[&5].active.is_none());
    }

    #[test]
    fn unavailable_provider_and_pending_priming_are_nonblocking() {
        let draft = teacher_draft();
        let mut prior = prior();
        prior.require_rich_information();
        prior.queue = None;
        let prepared = prior
            .prepare(draft.clone(), ExperienceSequenceId(1))
            .unwrap();
        assert!(prepared.sensory().semantic_context.is_none());
        assert_eq!(
            prepared.sensory().language_context,
            draft.sensory().language_context
        );
        let mut prior = self::prior();
        prior.require_rich_information();
        let key = bounded_context(&draft, &heard_words(&draft));
        let _sender = pending_result(&mut prior, &key, None, false);
        let started = Instant::now();
        for _ in 0..100 {
            prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        }
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(prior.pending(5));
        assert_eq!(prior.metrics.requests, 0);
    }

    #[test]
    fn provider_cooldown_bounds_changed_context_retries() {
        let mut prior = prior();
        prior.require_rich_information();
        let original = draft();
        let key = bounded_context(&original, &[]);
        pending_result(&mut prior, &key, Some(Err("failed provider".into())), false);
        prior.prepare(original, ExperienceSequenceId(1)).unwrap();
        for _ in 0..100 {
            prior
                .prime(teacher_draft(), ExperienceSequenceId(2))
                .unwrap();
        }
        assert_eq!(prior.metrics.requests, 0);
        assert_eq!(prior.metrics.failures, 1);
        prior.retry_not_before = Some(Instant::now() - Duration::from_secs(1));
        prior
            .prime(teacher_draft(), ExperienceSequenceId(2))
            .unwrap();
        assert_eq!(prior.metrics.requests, 1);
    }

    #[test]
    fn corrupt_nonprior_sequence_and_checkpoint_still_fail() {
        let mut prior = prior();
        prior.require_rich_information();
        assert!(prior.prepare(draft(), ExperienceSequenceId(0)).is_err());
        assert!(prior.restore_life(5, 0, b"invalid checkpoint").is_err());
    }
    #[test]
    fn frozen_recordings_use_exact_contexts_without_live_inference_or_retry() {
        let mut prior = prior();
        prior.require_rich_information();
        prior.queue = None;
        prior.frozen_recording = true;
        let original = teacher_draft();
        let key = bounded_context(&original, &heard_words(&original));
        prior.cache.insert(key.clone(), output());
        prior
            .prime(original.clone(), ExperienceSequenceId(1))
            .unwrap();
        assert!(prior.ready(5));
        let prepared = prior
            .prepare(original.clone(), ExperienceSequenceId(1))
            .unwrap();
        assert!(prepared.sensory().semantic_context.is_some());
        assert_eq!(
            prepared.sensory().language_context,
            original.sensory().language_context
        );
        assert_eq!(prior.metrics.requests, 0);
        let changed = draft();
        // A different current context cannot borrow the recorded teacher cue.
        prior.lives.get_mut(&5).unwrap().recently_heard = None;
        let missed = prior.prepare(changed, ExperienceSequenceId(2)).unwrap();
        assert!(missed.sensory().semantic_context.is_none());
        assert_eq!(prior.metrics.requests, 0);
        assert_eq!(prior.metrics.unusable_hint_retry_requests, 0);
        let mut zero = output();
        zero.lexicon_associations[0].salience = 0.0;
        prior.cache.insert(key, zero);
        prior
            .prime(original.clone(), ExperienceSequenceId(3))
            .unwrap();
        assert!(!prior.pending(5));
        assert!(!prior.ready(5));
        assert!(prior
            .prepare(original, ExperienceSequenceId(3))
            .unwrap()
            .sensory()
            .semantic_context
            .is_none());
        assert_eq!(prior.metrics.requests, 0);
    }

    #[test]
    fn context_contract_is_bounded_and_identifies_prompt_and_vocabulary() {
        let contract = context_contract();
        assert!(contract.starts_with("grounded-terrain-language-prior-v1:"));
        assert!(contract.len() <= 256);
        assert_eq!(contract.split(':').count(), 3);
        let source = include_str!("semantic_prior.rs");
        let span = context_builder_source(source);
        assert!(span.starts_with("\nfn bounded_context("));
        assert!(span.contains("smells chemical levels"));
        assert!(span.contains("grounded_object_slots"));
        let changed = source.replace("smells chemical levels", "changed sensory clause");
        assert_ne!(
            blake3::hash(span.as_bytes()),
            blake3::hash(context_builder_source(&changed).as_bytes())
        );
    }
    #[test]
    fn provider_swap_discards_old_hints_but_preserves_validated_developmental_state() {
        let original = draft();
        let mut source = prior();
        source.require_rich_information();
        source.metrics.provider_identity = "old-provider".into();
        source
            .cache
            .insert(bounded_context(&original, &[]), output());
        source.prepare(original, ExperienceSequenceId(1)).unwrap();
        source
            .lives
            .get_mut(&5)
            .unwrap()
            .controller
            .record_relevant_exposure(false);
        let exposures = source.lives[&5].controller.unaided_exposures();
        let bytes = source.snapshot(5).unwrap().unwrap();
        let mut replacement = prior();
        replacement.metrics.provider_identity = "new-provider".into();
        replacement.restore_life(5, 0, &bytes).unwrap();
        assert!(replacement.lives[&5].active.is_none());
        assert!(replacement.lives[&5].last_request.is_none());
        assert_eq!(
            replacement.lives[&5].controller.unaided_exposures(),
            exposures
        );
        assert_eq!(replacement.metrics.failures, 1);
        assert!(replacement
            .restore_life(5, 0, b"corrupt checkpoint")
            .is_err());
    }
    #[test]
    fn deterministic_current_context_delivers_without_worker_or_wait() {
        let mut prior = prior();
        prior.require_rich_information();
        prior.queue = None;
        prior.deterministic = Some(DeterministicPriorProvider::default());
        let draft = teacher_draft();
        prior.prime(draft.clone(), ExperienceSequenceId(1)).unwrap();
        assert!(prior.ready(5));
        assert!(!prior.pending(5));
        let prepared = prior
            .prepare(draft.clone(), ExperienceSequenceId(1))
            .unwrap();
        assert!(prepared.sensory().semantic_context.is_some());
        assert_eq!(
            prepared.sensory().language_context,
            draft.sensory().language_context
        );
        assert_eq!(prepared.sensory().channels, draft.sensory().channels);
        assert_eq!(
            prepared.grounded_object_slots(),
            draft.grounded_object_slots()
        );
        assert_eq!(prior.metrics.requests, 1);
        assert_eq!(prior.metrics.delivered_frames, 1);
        assert_eq!(prior.metrics.unusable_hint_retry_requests, 0);
        assert!(prior.queue.is_none());
    }
    #[test]
    fn training_and_gameplay_defaults_and_explicit_off_are_separate() {
        assert_eq!(selected_backend(true, None, false), "deterministic");
        assert_eq!(selected_backend(false, None, false), "local");
        for backend in ["deterministic", "local", "luna", "recorded", "off"] {
            assert_eq!(selected_backend(true, Some(backend.into()), false), backend);
            assert_eq!(selected_backend(true, Some(backend.into()), true), "off");
        }
    }

    #[test]
    fn on_off_save_on_preserves_prior_developmental_state_without_delivering_hints() {
        let original = draft();
        let mut enabled = prior();
        enabled.require_rich_information();
        enabled
            .cache
            .insert(bounded_context(&original, &[]), output());
        enabled
            .prepare(original.clone(), ExperienceSequenceId(1))
            .unwrap();
        for _ in 0..130 {
            enabled
                .lives
                .get_mut(&5)
                .unwrap()
                .controller
                .record_relevant_exposure(false);
        }
        enabled
            .lives
            .get_mut(&5)
            .unwrap()
            .controller
            .record_unaided_probe(1.0)
            .unwrap();
        let before = enabled.lives[&5].controller.clone();
        let bytes = enabled.snapshot(5).unwrap().unwrap();
        let mut off = prior();
        off.queue = None;
        off.dropout_seed = None;
        off.metrics.backend = "off".into();
        off.metrics.provider_identity = "off-state-carrier".into();
        off.restore_life(5, 0, &bytes).unwrap();
        assert_eq!(off.lives[&5].controller, before);
        let unaided = off.prepare(original, ExperienceSequenceId(2)).unwrap();
        assert!(unaided.sensory().semantic_context.is_none());
        assert!(off.queue.is_none());
        let preserved = off.lives[&5].controller.clone();
        let disabled_bytes = off.snapshot(5).unwrap().unwrap();
        let mut reenabled = prior();
        reenabled.restore_life(5, 0, &disabled_bytes).unwrap();
        assert_eq!(reenabled.lives[&5].controller, preserved);
        assert!(reenabled.lives[&5].active.is_none());
        assert_eq!(
            reenabled.lives[&5].controller.developmental_gain(),
            before.developmental_gain()
        );
    }
}
