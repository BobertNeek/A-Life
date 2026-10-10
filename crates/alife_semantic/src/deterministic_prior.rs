//! Bounded, deterministic foundation-lesson semantic hypotheses.
//!
//! This is a small rule provider, not an LLM or a recording of model replies.
//! It consumes only the canonical coarse observation clauses. Word relations
//! are candidates, not claims that their referents are present. Physical
//! prototypes require shape AND chemistry; color, object identity, position,
//! lesson goals, action choices, learner state and reward are never consulted.
//! AOA-SLM-001..004: nonauthoritative text-in/hints-out, without learner access.

use alife_core::BASIC_VOCABULARY_V1;

use crate::local_slm_prior::CA27_MAX_PROMPT_CHARS;
use crate::{
    BoundedSlmPriorProvider, LocalSlmPriorOutput, LocalSlmPriorRequest, SlmLexiconAssociation,
    CA27_SLM_PRIOR_OUTPUT_SCHEMA, CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
};

pub const DETERMINISTIC_PRIOR_ID: &str = "deterministic-foundation-prior-v1";
/// Match the live providers' requested hint bandwidth, despite the wider CA27 schema.
pub const DETERMINISTIC_MAX_ASSOCIATIONS: usize = 3;
const MAX_TAGS: usize = 4;

/// Stateless, bounded semantic associations for the current 18-word codebook.
/// All scores are confidence-like hints, never motor or action-selection scores.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicPriorProvider;

#[derive(Debug, Default)]
struct Context {
    heard: [bool; 18],
    heard_clause: bool,
    hunger: Option<bool>,
    tiredness: Option<bool>,
    smell: Option<[f32; 3]>,
    touch: Option<[f32; 2]>,
    objects: Vec<ObjectCue>,
    terrain: bool,
}

#[derive(Debug)]
struct ObjectCue {
    shape: [f32; 3],
    chemical: [f32; 3],
}

impl BoundedSlmPriorProvider for DeterministicPriorProvider {
    fn generate_prior(
        &self,
        bounded_context: &str,
        _unusable_hint_feedback: bool,
    ) -> Result<LocalSlmPriorOutput, String> {
        LocalSlmPriorRequest {
            request_id: 1,
            prompt: bounded_context.to_string(),
        }
        .validate(CA27_MAX_PROMPT_CHARS)
        .map_err(|error| format!("invalid deterministic prior context: {error:?}"))?;
        let context = Context::parse(bounded_context)?;
        let mut scores = [0.0_f32; 18];

        for (index, heard) in context.heard.iter().enumerate() {
            if !heard {
                continue;
            }
            scores[index] = 0.9;
            // General word relations. They never declare an observed object,
            // current need, legal action, or a correct lesson response.
            let relations: &[(&str, f32)] = match BASIC_VOCABULARY_V1[index].0 {
                "food" => &[("eat", 0.45)],
                "toy" => &[("play", 0.45)],
                "obstacle" => &[("look", 0.3)],
                "approach" | "retreat" => &[("look", 0.3)],
                "eat" => &[("food", 0.6)],
                "rest" => &[("tired", 0.45)],
                "hungry" => &[("food", 0.45), ("eat", 0.35)],
                "tired" => &[("rest", 0.45)],
                "play" => &[("toy", 0.55), ("activity", 0.35)],
                "ball" => &[("toy", 0.6), ("play", 0.45)],
                "root" | "fruit" | "seed" => &[("food", 0.6)],
                "activity" => &[("toy", 0.45), ("play", 0.4)],
                _ => &[],
            };
            for &(token, score) in relations {
                merge(&mut scores, token, score);
            }
        }
        if context.hunger == Some(true) {
            merge(&mut scores, "hungry", 0.65);
        }
        if context.tiredness == Some(true) {
            merge(&mut scores, "tired", 0.65);
            merge(&mut scores, "rest", 0.35);
        }

        for object in &context.objects {
            merge(&mut scores, "look", 0.2);
            // These small physical recipes are coarse hypotheses about current
            // sensed shape + chemistry, not raw world-kind classifications.
            // 0.061 admits the canonical builder's one-decimal rounding only.
            for (name, shape, chemical) in [
                ("root", [0.45, 0.9, 0.45], [0.8, 0.25, 0.35]),
                ("fruit", [0.85, 0.8, 0.85], [0.85, 0.65, 0.8]),
                ("seed", [0.55, 0.35, 0.45], [0.75, 0.1, 0.15]),
            ] {
                if near(object.shape, shape) && near(object.chemical, chemical) {
                    merge(&mut scores, name, 0.55);
                    merge(&mut scores, "food", 0.6);
                }
            }
            if near(object.chemical, [0.0; 3]) {
                if near(object.shape, [0.8; 3]) {
                    merge(&mut scores, "ball", 0.55);
                    merge(&mut scores, "toy", 0.5);
                } else if near(object.shape, [0.9, 0.65, 0.6]) {
                    merge(&mut scores, "activity", 0.55);
                    merge(&mut scores, "toy", 0.5);
                }
            }
        }
        if context.terrain {
            // The clause reports a solid surface, without its identity or
            // traversability. It supports attention, not an obstacle claim.
            merge(&mut scores, "look", 0.2);
        }
        // Smell is a partial signed-band sample and grip is generic contact.
        // Neither proves food, edibility, a held item, or its kind.

        let mut ranked: Vec<usize> = (0..scores.len())
            .filter(|&index| scores[index] > 0.0)
            .collect();
        ranked.sort_by(|&left, &right| {
            scores[right]
                .total_cmp(&scores[left])
                .then_with(|| left.cmp(&right))
        });
        ranked.truncate(DETERMINISTIC_MAX_ASSOCIATIONS);
        let known = !ranked.is_empty();
        let associations = if known {
            ranked
                .iter()
                .map(|&index| SlmLexiconAssociation {
                    token: BASIC_VOCABULARY_V1[index].0.into(),
                    salience: scores[index],
                })
                .collect()
        } else {
            // The schema requires a row. Zero carries no receiver input, and
            // keeps the interface within its fixed 18-word vocabulary.
            vec![SlmLexiconAssociation {
                token: "activity".into(),
                salience: 0.0,
            }]
        };
        let mut tags = vec!["deterministic-rules".into(), "uncertain-hypotheses".into()];
        if context.heard.iter().any(|value| *value) {
            tags.push("heard-words".into());
        }
        if context.hunger == Some(true) || context.tiredness == Some(true) {
            tags.push("bodily-needs".into());
        }
        if !context.objects.is_empty() || context.terrain {
            tags.push("visual-cues".into());
        }
        if context.smell.is_some() || context.touch.is_some() {
            tags.push("unclassified-senses".into());
        }
        tags.truncate(MAX_TAGS);
        let output = LocalSlmPriorOutput {
            schema: CA27_SLM_PRIOR_OUTPUT_SCHEMA.into(),
            schema_version: CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
            model: DETERMINISTIC_PRIOR_ID.into(),
            salience_labels: if known {
                ranked
                    .iter()
                    .take(DETERMINISTIC_MAX_ASSOCIATIONS)
                    .map(|&index| BASIC_VOCABULARY_V1[index].0.into())
                    .collect()
            } else {
                vec!["unknown".into()]
            },
            context_summary: if known {
                "Rule candidates from heard words, needs and sensed cues; identity and edibility unknown".into()
            } else {
                "No supported vocabulary association; identity and edibility unknown".into()
            },
            lexicon_associations: associations,
            perception_tags: tags,
            can_issue_actions: false,
            can_rewrite_weights: false,
            can_bypass_arbitration: false,
            hidden_vector_injection: false,
            bounded_context_only: true,
        };
        output
            .validate()
            .map_err(|error| format!("invalid deterministic prior output: {error:?}"))?;
        Ok(output)
    }
}

fn merge(scores: &mut [f32; 18], token: &str, score: f32) {
    if let Some(index) = BASIC_VOCABULARY_V1
        .iter()
        .position(|&(word, _)| word == token)
    {
        scores[index] = scores[index].max(score);
    }
}

fn near(actual: [f32; 3], prototype: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(prototype)
        .all(|(actual, prototype)| (*actual - prototype).abs() <= 0.061)
}

impl Context {
    fn parse(text: &str) -> Result<Self, String> {
        let mut context = Self::default();
        for clause in text
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Some(words) = clause.strip_prefix("heard words ") {
                if context.heard_clause || words.split_whitespace().count() > 6 {
                    return Err("duplicate or oversized heard-word clause".into());
                }
                context.heard_clause = true;
                for word in words.split_whitespace() {
                    if let Some(index) = BASIC_VOCABULARY_V1
                        .iter()
                        .position(|&(known, _)| known == word)
                    {
                        context.heard[index] = true;
                    }
                }
            } else if clause == "heard words" {
                if context.heard_clause {
                    return Err("duplicate heard-word clause".into());
                }
                context.heard_clause = true;
            } else if let Some(level) = clause.strip_prefix("hunger ") {
                parse_need(&mut context.hunger, level)?;
            } else if let Some(level) = clause.strip_prefix("tiredness ") {
                parse_need(&mut context.tiredness, level)?;
            } else if let Some(values) = clause.strip_prefix("smells chemical levels ") {
                if context.smell.is_some() {
                    return Err("duplicate smell clause".into());
                }
                context.smell = Some(parse_triple(values, 0.0, 1.0)?);
            } else if let Some(values) = clause.strip_prefix("feels contact pressure ") {
                if context.touch.is_some() {
                    return Err("duplicate touch clause".into());
                }
                let (pressure, grip) = values
                    .split_once(" grip ")
                    .ok_or("invalid contact clause")?;
                context.touch = Some([
                    parse_scalar(pressure, 0.0, 1.0)?,
                    parse_scalar(grip, 0.0, 1.0)?,
                ]);
            } else if let Some(values) = clause.strip_prefix("sees object color ") {
                if context.objects.len() == 2 {
                    return Err("too many sensed object clauses".into());
                }
                let (color, rest) = values
                    .split_once(" shape ")
                    .ok_or("invalid object clause")?;
                let (shape, chemical) = rest
                    .split_once(" chemical ")
                    .ok_or("invalid object clause")?;
                // Validate color, but do not derive meaning from it.
                parse_triple(color, 0.0, 1.0)?;
                context.objects.push(ObjectCue {
                    shape: parse_triple(shape, 0.0, 1.0)?,
                    chemical: parse_triple(chemical, -1.0, 1.0)?,
                });
            } else if clause == "sees nearby terrain surface" {
                context.terrain = true;
            }
            // Unrecognized clauses are not facts. In particular, appended
            // names, labels, IDs, goals or hidden positions cannot bias hints.
        }
        Ok(context)
    }
}

fn parse_need(destination: &mut Option<bool>, level: &str) -> Result<(), String> {
    if destination.is_some() {
        return Err("duplicate or conflicting bodily-need clause".into());
    }
    *destination = Some(match level {
        "high" => true,
        "low" => false,
        _ => return Err("invalid bodily-need level".into()),
    });
    Ok(())
}

fn parse_scalar(text: &str, minimum: f32, maximum: f32) -> Result<f32, String> {
    let value: f32 = text.parse().map_err(|_| "invalid sensory scalar")?;
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err("sensory scalar outside canonical bounds".into());
    }
    Ok(value)
}

fn parse_triple(text: &str, minimum: f32, maximum: f32) -> Result<[f32; 3], String> {
    let mut values = text.split(',');
    let mut output = [0.0; 3];
    for value in &mut output {
        *value = parse_scalar(
            values.next().ok_or("missing sensory component")?,
            minimum,
            maximum,
        )?;
    }
    if values.next().is_some() {
        return Err("too many sensory components".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(context: &str) -> LocalSlmPriorOutput {
        DeterministicPriorProvider
            .generate_prior(context, false)
            .unwrap()
    }

    fn score(output: &LocalSlmPriorOutput, token: &str) -> f32 {
        output
            .lexicon_associations
            .iter()
            .find(|association| association.token == token)
            .map_or(0.0, |association| association.salience)
    }

    fn context(words: &str, hunger: &str, tiredness: &str, objects: &str) -> String {
        format!("heard words {words}; hunger {hunger}; tiredness {tiredness}; smells chemical levels 0.0,0.0,0.0; feels contact pressure 0.0 grip 0.0; {objects}")
    }

    fn object(shape: [f32; 3], chemical: [f32; 3]) -> String {
        format!(
            "sees object color 0.1,0.2,0.3 shape {:.1},{:.1},{:.1} chemical {:.1},{:.1},{:.1}; ",
            shape[0], shape[1], shape[2], chemical[0], chemical[1], chemical[2]
        )
    }

    #[test]
    fn all_eighteen_heard_words_and_need_combinations_are_bounded() {
        assert_eq!(BASIC_VOCABULARY_V1.len(), 18);
        for &(word, _) in BASIC_VOCABULARY_V1 {
            for hunger in ["low", "high"] {
                for tiredness in ["low", "high"] {
                    let output = run(&context(word, hunger, tiredness, ""));
                    assert_eq!(score(&output, word), 0.9, "{word}");
                    assert_eq!(output.model, DETERMINISTIC_PRIOR_ID);
                    assert!(output.validate().is_ok());
                    assert!(output.lexicon_associations.len() <= 3);
                    assert!(output.salience_labels.len() <= 3);
                    assert!(output.perception_tags.len() <= 4);
                    assert!(output.context_summary.chars().count() <= 96);
                    assert!(output
                        .perception_tags
                        .iter()
                        .all(|tag| tag.chars().count() <= 24));
                    assert!(output.lexicon_associations.iter().all(|association| {
                        BASIC_VOCABULARY_V1
                            .iter()
                            .any(|&(word, _)| word == association.token)
                    }));
                    assert!(
                        !output.can_issue_actions
                            && !output.can_rewrite_weights
                            && !output.can_bypass_arbitration
                            && !output.hidden_vector_injection
                    );
                    assert!(output.bounded_context_only);
                }
            }
        }
    }

    #[test]
    fn heard_pairs_compose_without_an_exact_scene_lookup() {
        for &(first, _) in BASIC_VOCABULARY_V1 {
            for &(second, _) in BASIC_VOCABULARY_V1 {
                let output = run(&context(&format!("{first} {second}"), "high", "high", ""));
                assert_eq!(score(&output, first), 0.9);
                assert_eq!(score(&output, second), 0.9);
            }
        }
    }

    #[test]
    fn unknown_and_zero_senses_supply_no_usable_association() {
        let output = run(&context(
            "symbol999 seafood tiredish fruit42",
            "low",
            "low",
            "",
        ));
        assert!(output
            .lexicon_associations
            .iter()
            .all(|association| association.salience == 0.0));
        assert_eq!(output.salience_labels, ["unknown"]);
        assert!(output.context_summary.contains("unknown"));
        assert!(output.validate().is_ok());
        assert_eq!(
            run("unrecognized observation"),
            run("hidden food at 3,2,1; lesson fruit; goal eat")
        );
    }

    #[test]
    fn exact_heard_words_do_not_claim_object_presence_or_current_need() {
        let output = run(&context("fruit tired", "low", "low", ""));
        assert_eq!(score(&output, "fruit"), 0.9);
        assert_eq!(score(&output, "tired"), 0.9);
        assert!(!output
            .perception_tags
            .iter()
            .any(|tag| tag == "visual-cues" || tag == "bodily-needs"));
        assert!(output
            .context_summary
            .contains("identity and edibility unknown"));
    }

    #[test]
    fn needs_are_explicit_and_conflicts_are_rejected() {
        let low = run(&context("", "low", "low", ""));
        assert_eq!(score(&low, "hungry"), 0.0);
        assert_eq!(score(&low, "tired"), 0.0);
        let high = run(&context("", "high", "high", ""));
        assert_eq!(score(&high, "hungry"), 0.65);
        assert_eq!(score(&high, "tired"), 0.65);
        assert_eq!(score(&high, "food"), 0.0);
        for bad in [
            "hunger low; hunger high",
            "tiredness high; tiredness low",
            "hunger medium",
        ] {
            assert!(DeterministicPriorProvider
                .generate_prior(bad, false)
                .is_err());
        }
    }

    #[test]
    fn current_physical_recipes_require_joint_shape_and_chemistry() {
        for (word, shape, chemical) in [
            ("root", [0.45, 0.9, 0.45], [0.8, 0.25, 0.35]),
            ("fruit", [0.85, 0.8, 0.85], [0.85, 0.65, 0.8]),
            ("seed", [0.55, 0.35, 0.45], [0.75, 0.1, 0.15]),
        ] {
            let output = run(&context("", "low", "low", &object(shape, chemical)));
            assert_eq!(score(&output, word), 0.55, "{word}");
            assert_eq!(score(&output, "food"), 0.6);
            for bad in [
                object(shape, [0.0; 3]),
                object([0.0; 3], chemical),
                object(shape, [-0.8, 0.2, 0.3]),
            ] {
                let output = run(&context("", "low", "low", &bad));
                assert_eq!(score(&output, "food"), 0.0);
                assert_eq!(score(&output, word), 0.0);
            }
        }
    }

    #[test]
    fn toy_shapes_and_mixed_objects_remain_candidate_hints() {
        let ball = object([0.8; 3], [0.0; 3]);
        let activity = object([0.9, 0.65, 0.6], [0.0; 3]);
        assert_eq!(score(&run(&context("", "low", "low", &ball)), "ball"), 0.55);
        assert_eq!(
            score(&run(&context("", "low", "low", &activity)), "activity"),
            0.55
        );
        let mixed = format!("{}{}", object([0.45, 0.9, 0.45], [0.8, 0.25, 0.35]), ball);
        let output = run(&context("get play", "low", "low", &mixed));
        for word in ["get", "play", "food"] {
            assert!(score(&output, word) > 0.0, "{word}");
        }
        assert!(output.lexicon_associations.len() <= DETERMINISTIC_MAX_ASSOCIATIONS);
    }

    #[test]
    fn color_smell_grip_and_hidden_labels_cannot_declare_food() {
        let raw = "heard words ; hunger low; tiredness low; smells chemical levels 1.0,1.0,1.0; feels contact pressure 1.0 grip 1.0; sees object color 0.9,0.1,0.1 shape 0.2,0.3,0.4 chemical 0.0,0.0,0.0";
        let output = run(raw);
        for word in ["food", "fruit", "root", "seed", "get", "eat"] {
            assert_eq!(score(&output, word), 0.0);
        }
        assert_eq!(
            output,
            run(&format!(
                "{raw}; object_id food42; hidden position 3,2,1; lesson fruit; reward eat"
            ))
        );
        let prototype = context(
            "",
            "low",
            "low",
            &object([0.85, 0.8, 0.85], [0.85, 0.65, 0.8]),
        );
        assert_eq!(
            run(&prototype),
            run(&prototype.replace("color 0.1,0.2,0.3", "color 0.9,0.9,0.9"))
        );
    }

    #[test]
    fn deterministic_merge_order_tie_breaks_and_feedback_are_stable() {
        let first = context("food fruit eat tired play ball", "high", "high", "");
        let second = context("ball play tired eat fruit food", "high", "high", "");
        let output = run(&first);
        assert_eq!(output, run(&second));
        assert_eq!(
            output,
            DeterministicPriorProvider
                .generate_prior(&first, true)
                .unwrap()
        );
        for _ in 0..32 {
            assert_eq!(output, run(&first));
        }
        assert_eq!(
            output
                .lexicon_associations
                .iter()
                .map(|association| association.token.as_str())
                .collect::<Vec<_>>(),
            ["food", "eat", "tired"]
        );
        assert_eq!(
            run(&context("food food", "low", "low", "")),
            run(&context("food", "low", "low", ""))
        );
    }

    #[test]
    fn malformed_or_unbounded_canonical_input_is_rejected() {
        for bad in [
            "",
            "heard words food toy creature obstacle look approach retreat",
            "smells chemical levels NaN,0.0,0.0",
            "smells chemical levels -0.1,0.0,0.0",
            "feels contact pressure 0.0 grip 2.0",
            "sees object color 0.0,0.0,0.0 shape 0.0,0.0,0.0 chemical inf,0.0,0.0",
            "heard words food; heard words toy",
            "Entity(42)",
        ] {
            assert!(
                DeterministicPriorProvider
                    .generate_prior(bad, false)
                    .is_err(),
                "{bad}"
            );
        }
        assert!(DeterministicPriorProvider
            .generate_prior(&"x".repeat(CA27_MAX_PROMPT_CHARS + 1), false)
            .is_err());
        let too_many = object([0.8; 3], [0.0; 3]).repeat(3);
        assert!(DeterministicPriorProvider
            .generate_prior(&too_many, false)
            .is_err());
    }
}
