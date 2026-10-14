//! One bounded asynchronous prior service; no action, target or memory authority.
use alife_core::{
    CompressedSemanticCode, Confidence, ContextFeatureFlags, ExperienceSequenceId,
    NormalizedScalar, PerceptionFrame, PerceptionFrameDraft, SemanticContextRef,
    SemanticPriorPacket, SemanticPriorRequest, BASIC_VOCABULARY_V1,
};
use alife_semantic::{
    DevelopmentalPriorController, LlamaCppSlmPriorConfig, LocalSlmPriorAsyncQueue,
    LocalSlmPriorOutput, LocalSlmPriorRequest,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::mpsc::{Receiver, TryRecvError},
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SemanticPriorMetrics {
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
}
const MAX_RECEIPTS: usize = 256;

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
    queue: LocalSlmPriorAsyncQueue,
    lives: BTreeMap<u64, Life>,
    // This cache belongs to one fixed provider configuration and prompt version.
    cache: BTreeMap<String, LocalSlmPriorOutput>,
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
        if std::env::var("ALIFE_SLM_PRIOR").is_ok_and(|s| s == "off") {
            return Ok(None);
        }
        let config = LlamaCppSlmPriorConfig {
            model: std::env::var("ALIFE_SLM_PRIOR_MODEL")
                .unwrap_or_else(|_| "alife-qwen3.5-0.8b-prior".into()),
            timeout_ms: 30_000,
            association_vocabulary: BASIC_VOCABULARY_V1
                .iter()
                .map(|(word, _)| (*word).to_string())
                .collect(),
            ..Default::default()
        };
        let model_sha256 = std::env::var("ALIFE_SLM_PRIOR_MODEL_SHA256").unwrap_or_else(|_| {
            if config.model == "alife-qwen3.5-0.8b-prior" {
                "37ae482d336108d23516fa35e8e0c4126688d81018b87178a18d752a1357814f".into()
            } else {
                "unverified-model".into()
            }
        });
        let identity = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}",
            config.host,
            config.port,
            config.model,
            config.num_predict,
            model_sha256,
            blake3::hash(include_bytes!(
                "../../../alife_semantic/src/local_slm_prior.rs"
            ))
            .to_hex(),
            blake3::hash(include_bytes!("semantic_prior.rs")).to_hex(),
            blake3::hash(include_bytes!("../../../alife_core/src/language.rs")).to_hex(),
            "grounded-terrain-language-prior-v1"
        );
        let provider_identity = blake3::hash(identity.as_bytes()).to_hex().to_string();
        let cache_path = std::path::PathBuf::from("target/slm-prior/cache")
            .join(format!("{provider_identity}.json"));
        let cache: BTreeMap<String, LocalSlmPriorOutput> = std::fs::metadata(&cache_path)
            .ok()
            .filter(|m| m.len() <= 256 * 1024)
            .and_then(|_| std::fs::read(&cache_path).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let cache = cache
            .into_iter()
            .filter(|(key, output)| key.len() <= 768 && output.validate().is_ok())
            .take(128)
            .collect();
        let metrics = SemanticPriorMetrics {
            model: config.model.clone(),
            provider_identity,
            model_sha256,
            ..Default::default()
        };
        Ok(Some(Self {
            queue: LocalSlmPriorAsyncQueue::new(config)?,
            lives: BTreeMap::new(),
            cache,
            cache_path,
            next_request: 1,
            dropout_seed: training.then_some(seed),
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
                        return Err(error);
                    }
                    self.metrics.validated_provider_replies += 1;
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
                }
                Err(TryRecvError::Empty) => {
                    life.pending = Some(pending);
                    life.input_status = PriorInputStatus::Pending;
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
                if !consume {
                    life.input_status = if has_usable_hints(output) {
                        PriorInputStatus::Delivered
                    } else {
                        PriorInputStatus::EmptyHints
                    };
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
            } else if life.pending.is_none()
                && life.last_request.as_ref().is_none_or(|(context, at)| {
                    context != &key || tick.raw().saturating_sub(*at) >= 128
                })
            {
                let request = LocalSlmPriorRequest {
                    request_id: self.next_request,
                    prompt: key.clone(),
                };
                match self.queue.submit(request) {
                    Ok(reply) => {
                        self.next_request = self
                            .next_request
                            .checked_add(1)
                            .ok_or(alife_core::ScaffoldContractError::InvalidId)?;
                        life.pending = Some(Pending {
                            key: key.clone(),
                            reply,
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
                    }
                }
            }
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
            || state.provider != self.metrics.provider_identity
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
        self.next_request = self.next_request.max(state.next_request);
        // In-flight network work is resubmitted; sealed active hints and fade state survive.
        self.lives.insert(
            id,
            Life {
                controller: state.controller,
                pending: None,
                active: state.active,
                last_request: if state.pending_request {
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
    fn prior() -> RuntimeSemanticPrior {
        RuntimeSemanticPrior {
            queue: LocalSlmPriorAsyncQueue::new(LlamaCppSlmPriorConfig::default()).unwrap(),
            lives: BTreeMap::new(),
            cache: BTreeMap::new(),
            next_request: 1,
            cache_path: std::env::temp_dir().join("alife-semantic-receipt-unused-cache.json"),
            dropout_seed: Some(5),
            metrics: SemanticPriorMetrics::default(),
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
}
