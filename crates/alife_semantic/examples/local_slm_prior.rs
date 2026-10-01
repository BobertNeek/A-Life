//! One real request through the bounded production prior provider; no organism or training.

use std::time::Instant;

use alife_semantic::{LlamaCppSlmPriorConfig, LlamaCppSlmPriorProvider};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model = args
        .next()
        .ok_or("usage: local_slm_prior MODEL_ALIAS BOUNDED_CONTEXT")?;
    let context = args
        .next()
        .ok_or("usage: local_slm_prior MODEL_ALIAS BOUNDED_CONTEXT")?;
    if args.next().is_some() {
        return Err("expected one quoted bounded context".into());
    }
    let provider = LlamaCppSlmPriorProvider::new(LlamaCppSlmPriorConfig {
        model,
        timeout_ms: 30_000,
        ..LlamaCppSlmPriorConfig::default()
    })?;
    let started = Instant::now();
    let output = provider.generate_prior(&context)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "elapsed_ms": started.elapsed().as_millis(),
            "bounded_context": context,
            "validated_prior": output,
        }))?
    );
    Ok(())
}
