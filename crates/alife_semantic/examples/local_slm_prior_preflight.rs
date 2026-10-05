//! One bounded real provider request; no organism, GPU dispatch or training.
use alife_core::BASIC_VOCABULARY_V1;
use alife_semantic::{LlamaCppSlmPriorConfig, LlamaCppSlmPriorProvider};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("ALIFE_SLM_PRIOR").is_ok_and(|value| value == "off") {
        return Err("rich-information preflight is incompatible with ALIFE_SLM_PRIOR=off".into());
    }
    let model = std::env::var("ALIFE_SLM_PRIOR_MODEL")
        .unwrap_or_else(|_| "alife-qwen3.5-0.8b-prior".into());
    let supplied_sha256 = std::env::var("ALIFE_SLM_PRIOR_MODEL_SHA256").unwrap_or_else(|_| {
        if model == "alife-qwen3.5-0.8b-prior" {
            "37ae482d336108d23516fa35e8e0c4126688d81018b87178a18d752a1357814f".into()
        } else {
            "unverified-model".into()
        }
    });
    let config = LlamaCppSlmPriorConfig {
        model,
        timeout_ms: 30_000,
        association_vocabulary: BASIC_VOCABULARY_V1
            .iter()
            .map(|(word, _)| (*word).into())
            .collect(),
        ..Default::default()
    };
    let mut args = std::env::args().skip(1);
    let context = args.next().unwrap_or_else(||
        "heard words eat food; hunger high; tiredness low; smells chemical levels 0.7,0.1,0.0; feels contact pressure 0.1 grip 0.8; sees object color 0.5,0.2,0.1 shape 0.3,0.3,0.3 chemical 0.7,0.1,0.0;".into());
    if args.next().is_some() {
        return Err("expected at most one quoted bounded context".into());
    }
    let provider = LlamaCppSlmPriorProvider::new(config.clone())?;
    let started = Instant::now();
    let result = provider.generate_prior(&context).and_then(|output| {
        output
            .validate()
            .map_err(|error| format!("invalid output: {error:?}"))?;
        if !output.lexicon_associations.iter().any(|association| {
            association.salience > 0.0
                && BASIC_VOCABULARY_V1
                    .iter()
                    .any(|(word, _)| *word == association.token)
        }) {
            return Err(
                "provider returned no usable positive receiver-vocabulary associations".into(),
            );
        }
        Ok(output)
    });
    let receipt = serde_json::json!({
        "schema": "alife.local_slm_prior_preflight.v1",
        "ready": result.is_ok(),
        "host": config.host, "port": config.port, "model": config.model,
        "timeout_ms": config.timeout_ms, "elapsed_ms": started.elapsed().as_millis(),
        "supplied_model_sha256_claim": supplied_sha256,
        "model_file_hash_verified": false,
        "bounded_context": context,
        "validated_prior": result.as_ref().ok(),
        "error": result.as_ref().err().map(|error| error.chars().take(512).collect::<String>()),
        "scope": "provider response only; neural-port delivery requires a runtime decision receipt"
    });
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    result.map(|_| ()).map_err(Into::into)
}
