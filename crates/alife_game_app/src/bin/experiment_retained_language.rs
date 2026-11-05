//! Disposable native CPU pilot. Continuation accepts only its measured public founders.
use alife_game_app::{
    assess_unseen_grounding, compare_witnessed_early_stop, continue_action_gated_language,
    continue_balanced_grounded_language, continue_contrastive_grounded_language,
    continue_embodied_language_curriculum, continue_event_confirmed_speech,
    continue_factorial_grounded_language, continue_focused_grounded_language,
    continue_language_curriculum, continue_linear_grounded_language,
    continue_physical_before_speech, continue_report_choice,
    continue_visible_balanced_grounded_language, continue_witnessed_replay,
    preflight_witnessed_replay, refine_language_payload, refine_language_shared_gates,
    reload_language_pilot_child, run_retained_language_pilot, LanguagePilotConfig,
};
use std::{collections::BTreeMap, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let [flag, directory] = arguments.as_slice() {
        if flag == "--experimental-reload-child" {
            println!("{}", reload_language_pilot_child(Path::new(directory))?);
            return Ok(());
        }
    }
    let mut args = arguments.into_iter();
    let mut values = BTreeMap::new();
    let mut disposable = false;
    let mut slm = false;
    let mut bridge = false;
    while let Some(key) = args.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("experiment_retained_language --disposable-public-founder --output NEW_DIRECTORY [--seed 847501] [--epochs 16] [--rounds 2] [--decisions 24] [--llamacpp-translation] [--experimental-word-bridge]\n\nExperimental native CPU food/eat versus toy/get teaching with grounded, shuffled-word, and untreated arms. The explicit experimental word bridge broadcasts two raw token bit vectors to native readouts and replaces the untreated arm with an equally trained no-speech arm. Continuation stages additionally accept one of --continue-curriculum SOURCE, --refine-curriculum-payload SOURCE, --refine-shared-gates SOURCE, --fork-embodied-speech SOURCE, --action-gated-payload SOURCE, --factorial-grounding SOURCE, --focused-grounding SOURCE, --contrastive-grounding SOURCE, --linear-readout SOURCE, --balanced-readout SOURCE, --balanced-visible-readout SOURCE, --event-confirmed-speech SOURCE, or --report-choice SOURCE; frozen assessment accepts --unseen-grounding SOURCE; physical ordering accepts --physical-before-speech SOURCE; witnessed replay accepts --witnessed-replay SOURCE or --witnessed-preflight SOURCE; matched early-stop comparison accepts --witnessed-early-stop SOURCE. Each validates its matching measured disposable source. The embodied stage creates new founder identities with raw bodily inputs to speech, retains actor/Adam history, and evaluates an untreated profile baseline; it does not migrate a source private life. Reuses ordinary spatial speech and the existing bounded translator. Optional llama.cpp translation uses its existing localhost defaults and must return confirmed translations; it never falls back silently. No semantic prior is injected. Evaluates player requests and silent lives with the teacher absent, reversed positions and visible distractors. Trained arms execute full assessment after fresh-process reload. Writes local experimental recovery data and a source-safe results JSON; does not promote a creature or change live training.");
                return Ok(());
            }
            "--disposable-public-founder" if !disposable => disposable = true,
            "--llamacpp-translation" if !slm => slm = true,
            "--experimental-word-bridge" if !bridge => bridge = true,
            "--output"
            | "--seed"
            | "--epochs"
            | "--rounds"
            | "--decisions"
            | "--continue-curriculum"
            | "--refine-curriculum-payload"
            | "--refine-shared-gates"
            | "--fork-embodied-speech"
            | "--action-gated-payload"
            | "--factorial-grounding"
            | "--focused-grounding"
            | "--contrastive-grounding"
            | "--linear-readout"
            | "--balanced-readout"
            | "--balanced-visible-readout"
            | "--event-confirmed-speech"
            | "--report-choice"
            | "--unseen-grounding"
            | "--physical-before-speech"
            | "--witnessed-replay"
            | "--witnessed-early-stop"
            | "--witnessed-preflight" => {
                let value = args.next().ok_or("missing argument value")?;
                if values.insert(key, value).is_some() {
                    return Err("duplicate argument".into());
                }
            }
            _ => return Err(format!("unknown or duplicate argument {key}").into()),
        }
    }
    if !disposable {
        return Err("--disposable-public-founder is required".into());
    }
    let output = values.remove("--output").ok_or("--output is required")?;
    let config = LanguagePilotConfig {
        seed: values
            .remove("--seed")
            .unwrap_or_else(|| "847501".into())
            .parse()?,
        epochs: values
            .remove("--epochs")
            .unwrap_or_else(|| "16".into())
            .parse()?,
        rounds: values
            .remove("--rounds")
            .unwrap_or_else(|| "2".into())
            .parse()?,
        decisions: values
            .remove("--decisions")
            .unwrap_or_else(|| "24".into())
            .parse()?,
        llamacpp_translation: slm,
        experimental_word_bridge: bridge,
    };
    if [
        "--continue-curriculum",
        "--refine-curriculum-payload",
        "--refine-shared-gates",
        "--fork-embodied-speech",
        "--action-gated-payload",
        "--factorial-grounding",
        "--focused-grounding",
        "--contrastive-grounding",
        "--linear-readout",
        "--balanced-readout",
        "--balanced-visible-readout",
        "--event-confirmed-speech",
        "--report-choice",
        "--unseen-grounding",
        "--physical-before-speech",
        "--witnessed-replay",
        "--witnessed-preflight",
        "--witnessed-early-stop",
    ]
    .iter()
    .filter(|key| values.contains_key(**key))
    .count()
        > 1
    {
        return Err("choose one experimental continuation stage".into());
    }
    let report = if let Some(source) = values.remove("--witnessed-early-stop") {
        compare_witnessed_early_stop(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--witnessed-preflight") {
        preflight_witnessed_replay(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--witnessed-replay") {
        continue_witnessed_replay(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--physical-before-speech") {
        continue_physical_before_speech(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--unseen-grounding") {
        assess_unseen_grounding(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--report-choice") {
        continue_report_choice(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--event-confirmed-speech") {
        continue_event_confirmed_speech(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--balanced-visible-readout") {
        continue_visible_balanced_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--balanced-readout") {
        continue_balanced_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--linear-readout") {
        continue_linear_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--contrastive-grounding") {
        continue_contrastive_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--focused-grounding") {
        continue_focused_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--factorial-grounding") {
        continue_factorial_grounded_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--action-gated-payload") {
        continue_action_gated_language(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--fork-embodied-speech") {
        continue_embodied_language_curriculum(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--refine-shared-gates") {
        refine_language_shared_gates(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--refine-curriculum-payload") {
        refine_language_payload(Path::new(&source), Path::new(&output), config)?
    } else if let Some(source) = values.remove("--continue-curriculum") {
        continue_language_curriculum(Path::new(&source), Path::new(&output), config)?
    } else {
        run_retained_language_pilot(Path::new(&output), config)?
    };
    println!("{report}");
    Ok(())
}
