//! Explicit real-provider delivery probe on a disposable public simulation.
use super::*;

#[test]
#[ignore = "requires existing authorized official Codex login and explicit fresh probe directory"]
fn capture_real_luna_provider_to_native_encoded_lanes() {
    use crate::gpu_live_runtime::semantic_prior::RuntimeSemanticPrior;
    use std::time::Duration;
    assert_eq!(
        std::env::var("ALIFE_SLM_PRIOR_BACKEND").as_deref(),
        Ok("luna")
    );
    // This adapter clears these variables for a ChatGPT-only subprocess. Do
    // not invoke it when that would remove managed authentication/routing.
    for name in ["OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL"] {
        assert!(
            std::env::var_os(name).is_none(),
            "provider must preserve managed auth/routing"
        );
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("ALIFE_LANGUAGE_PROVIDER_PROBE_DIR")
            .expect("explicit new probe directory"),
    );
    fs::create_dir(&directory).unwrap();
    let seed = 847_503; // Does not select the native one-in-five training dropout.
    let asset = initial_n2048_care_asset(seed).unwrap();
    let actor = new_actor_profile(seed, &asset, false).unwrap();
    let (mut host, scene) = host(seed, seed, &asset, &actor, 0, false).unwrap();
    let id = host.world().organism_entity_ids()[0].0;
    let pos = speech_position(host.world()).unwrap();
    host.world_mut()
        .emit_player_tokens(
            Some(id),
            pos,
            vec![
                LanguageTokenId::new(8).unwrap(),
                LanguageTokenId::new(1).unwrap(),
            ],
        )
        .unwrap();
    // The empty disposable working directory gives this probe its own cache.
    let previous_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&directory).unwrap();
    let mut prior = RuntimeSemanticPrior::from_environment(seed, true)
        .unwrap()
        .unwrap();
    let before = identity(&actor.checkpoint()).unwrap();
    let mut observation = Value::Null;
    let started = Instant::now();
    let step=host.step_with_preparation(&actor,1.,|draft,sequence| {
        observation=json!({"organism":draft.organism_id(),"tick":draft.tick(),"body":draft.body(),"homeostasis":draft.homeostasis(),"heard":draft.sensory().language_context,"grounded_object_slots":draft.grounded_object_slots()});
        prior.prime(draft.clone(),sequence)?;
        while prior.pending(id.raw())&&started.elapsed()<Duration::from_secs(35) {
            std::thread::sleep(Duration::from_millis(40));
            prior.prime(draft.clone(),sequence)?;
        }
        prior.prepare(draft,sequence)
    }).unwrap();
    let row = step.decisions.first().expect("one actual native decision");
    let frame = row.patch.pre_action().perception();
    let evidence = serde_json::to_value(row.patch.decision()).unwrap();
    let generation = evidence["evidence"]["NeuralClosedLoopCpu"]["dispatch_generation"]
        .as_u64()
        .unwrap();
    prior.record_encoded_decision_input(
        frame,
        row.patch.header().sequence_id.raw(),
        generation,
        actor.phenotype(),
    );
    let state = prior
        .snapshot(id.raw())
        .unwrap()
        .map(|b| serde_json::from_slice::<Value>(&b).unwrap());
    let output = state
        .as_ref()
        .and_then(|s| s["active"].as_array())
        .and_then(|a| a.get(2))
        .cloned();
    let lanes = frame.sensory().language_prior_neural_lanes();
    let active_targets = actor
        .phenotype()
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|a| {
            a.source_group() == SensorEncoderSourceGroup::SemanticPrior
                && lanes[128 + usize::from(a.source_index())] != 0.
                && row.replay_tick.encoded_inputs[a.target_neuron() as usize] != 0.
        })
        .map(|a| a.target_neuron())
        .collect::<Vec<_>>();
    let report = json!({"kind":"real-remote-luna-native-delivery-probe-v1","seed":seed,"scene":scene,"execution_domain":"native-cpu","input":observation,"actual_model_output":output,"provider_metrics":prior.metrics,"nonzero_actual_encoded_prior_targets":active_targets,"actor_unchanged":identity(&actor.checkpoint()).unwrap()==before,"elapsed_seconds":started.elapsed().as_secs_f64(),"demonstrates_retained_language":false,"local_slm":false});
    write_json(&directory.join("provider-probe.json"), &report).unwrap();
    std::env::set_current_dir(previous_cwd).unwrap();
    assert_eq!(prior.metrics.backend, "luna");
    assert_eq!(prior.metrics.requests, 1, "no fallback or automatic retry");
    assert_eq!(
        prior.metrics.failures, 0,
        "real provider failure: {:?}",
        prior.metrics.last_error
    );
    assert_eq!(prior.metrics.validated_provider_replies, 1);
    assert_eq!(prior.metrics.decision_frames_with_prior, 1);
    assert!(
        !active_targets.is_empty(),
        "reply did not reach actual encoded prior targets"
    );
    let output: alife_semantic::LocalSlmPriorOutput =
        serde_json::from_value(output.expect("actual delivered output")).unwrap();
    output.validate().unwrap();
    assert_eq!(
        prior.metrics.deliveries.back().unwrap().output_digest,
        blake3::hash(&serde_json::to_vec(&output).unwrap())
            .to_hex()
            .to_string()
    );
    assert_eq!(identity(&actor.checkpoint()).unwrap(), before);
}
