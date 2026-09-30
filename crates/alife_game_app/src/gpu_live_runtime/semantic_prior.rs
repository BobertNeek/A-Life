//! One bounded asynchronous prior service; no action, target or memory authority.
use alife_core::{
    CompressedSemanticCode, Confidence, ContextFeatureFlags, ExperienceSequenceId,
    NormalizedScalar, PerceptionFrameDraft, SemanticContextRef, SemanticPriorPacket,
    SemanticPriorRequest, BASIC_VOCABULARY_V1,
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

    pub fn prepare(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
    ) -> Result<PerceptionFrameDraft, alife_core::ScaffoldContractError> {
        self.prepare_inner(draft, sequence, true)
    }
    pub fn prime(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
    ) -> Result<(), alife_core::ScaffoldContractError> {
        self.prepare_inner(draft, sequence, false).map(|_| ())
    }
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
            return Ok(draft);
        }
        if self
            .dropout_seed
            .is_some_and(|seed| (seed ^ id.raw()).is_multiple_of(5))
        {
            if consume {
                self.metrics.dropout_frames += 1;
            }
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
        if let Some(pending) = life.pending.take() {
            match pending.reply.try_recv() {
                Ok(Ok(output)) => {
                    output.validate()?;
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
                    self.metrics.failures += 1;
                    self.metrics.last_error = Some(error);
                }
                Err(TryRecvError::Empty) => life.pending = Some(pending),
                Err(TryRecvError::Disconnected) => self.metrics.failures += 1,
            }
        }
        if life.active.as_ref().is_some_and(|(context, packet, _)| {
            context != &key || tick.raw() >= packet.expires_at_tick.raw()
        }) {
            life.active = None;
        }
        if life.active.is_none() {
            if let Some(output) = self.cache.get(&key) {
                if !consume {
                    return Ok(draft);
                }
                let slots: Vec<u16> = output
                    .lexicon_associations
                    .iter()
                    .filter(|a| a.salience > 0.0)
                    .filter_map(|a| {
                        BASIC_VOCABULARY_V1
                            .iter()
                            .find(|(word, _)| *word == a.token)
                            .map(|(_, code)| *code)
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
                if let Ok(reply) = self.queue.submit(request) {
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
                }
            }
        }
        let context = life.active.as_ref().map(|(_, packet, output)| {
            self.metrics.delivered_frames += 1;
            SemanticContextRef {
                feature_flags: ContextFeatureFlags::NONE,
                confidence: Confidence(packet.plasticity_modulation),
                compressed_codes: packet
                    .lexicon_bias_slots
                    .iter()
                    .map(|slot| CompressedSemanticCode {
                        codebook_id: 1,
                        code: u32::from(*slot),
                        salience: NormalizedScalar(
                            output
                                .lexicon_associations
                                .iter()
                                .find(|a| {
                                    BASIC_VOCABULARY_V1
                                        .iter()
                                        .any(|(word, code)| code == slot && *word == a.token)
                                })
                                .map_or(0.0, |a| a.salience),
                        ),
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
