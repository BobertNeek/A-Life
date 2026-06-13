# A-Life Schooling and Teacher Architecture

Status: detailed future architecture. The master spec defines boundaries; this document expands the school system.

## Core Principle

The teacher LLM is not the creature's subconscious. The teacher is an external social actor. It teaches through ordinary perception: speech, hearing, writing, gesture, pointing, demonstration, objects, praise, correction, and world outcomes.

The internal SLM is a private subconscious semantic prior. It receives compressed sensory summaries, drives, and limited ExperiencePatch summaries. It may bias attention, lexicon/concept activity, memory recall, and weak plasticity. It never issues direct actions.

## P23 v0 School Contract

The implemented v0 school contract is stricter than the older bootstrapping notes below. Teacher-facing code in `alife_school` exposes lesson roles, lesson IDs, prompts/cues, expected observations, verifier checks, feedback events, and lesson response channels. Creature-facing teacher inputs are only spoken tokens, gestures, object highlights, social feedback, visible reward, and visible punishment. Hidden vectors, direct motor selection, private reward injection, and weight edits are not represented in the v0 teacher event API.

`LessonResponse` metadata may annotate an `ActionCommand` candidate through the existing teacher lesson metadata field, but it does not select the action. P09 arbitration still chooses between proposals by score, confidence, and traceable tie-breaking. P23 tests cover both a teacher-tagged candidate losing to a higher-scored ordinary candidate and a teacher-tagged candidate winning only when normal arbitration selects it.

P23 verification consumes sealed `ExperiencePatch` logs plus bounded memory/topology summaries. Verifiers check perceptual evidence such as heard teacher tokens, visible reward/feedback outcomes, absence of hidden semantic/Gaussian vectors, and whether selected actions came from arbitration rather than a teacher bypass.

## Teacher Roles

- Tutor.
- Examiner.
- Critic.
- Curriculum planner.
- Verifier.
- Translator.
- Storyteller.
- Peer tutor.

These roles can be separate modules even if one model initially implements several.

## Developmental Stages

1. Preschool grounding.
2. Language bootstrapping.
3. Writing and self-report.
4. Math through objects, then symbols.
5. History through maps/timelines/agents/causes/evidence.
6. Science through prediction and experiment.
7. Tool use and social reasoning.
8. Independent exams.

## Hidden Feedback Rules

Private grading is allowed. Private hidden training scores are allowed. Feedback should normally be converted into in-world signals. Limited direct reward/plasticity injection is allowed only as logged early bootstrapping. Clean-grounding experiments must disable it.

## Ablation Tests

Every school skill should be tested with teacher off, internal SLM reduced/off, novel environment, novel speaker, novel written material, delayed recall after sleep, and peer teaching transfer.

## Preschool Grounding

Preschool Grounding should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Preschool Grounding should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Preschool Grounding should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Preschool Grounding should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Language Bootstrapping

Language Bootstrapping should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Language Bootstrapping should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Language Bootstrapping should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Language Bootstrapping should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Writing

Writing should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Writing should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Writing should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Writing should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Math

Math should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Math should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Math should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Math should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## History

History should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

History should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

History should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

History should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Science

Science should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Science should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Science should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Science should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Social Reasoning

Social Reasoning should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Social Reasoning should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Social Reasoning should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Social Reasoning should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Tool Use

Tool Use should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Tool Use should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Tool Use should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Tool Use should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Peer Teaching

Peer Teaching should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Peer Teaching should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Peer Teaching should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Peer Teaching should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.


## Independent Exams

Independent Exams should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Independent Exams should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Independent Exams should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.

Independent Exams should be introduced only when the creature has the necessary grounding and brain capacity. The teacher may arrange world objects through a world-authorized lesson API, speak through the hearing channel, write through visible glyph surfaces, and provide social feedback. The evaluator may score performance privately, but the creature's learning loop should depend on perceived state, action, outcome, and local plasticity. Curriculum should advance by mastery and transfer, not by lesson count alone.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.



# Detailed School Design Requirements

## Puppy-stage limitation

Advanced learning must wait until the creature has the neural and experiential prerequisites. A creature that cannot track objects, speakers, roles, drive conflict, delayed reward, and basic cause-effect should not receive abstract curriculum. The school system should query a readiness profile before generating lessons. Readiness includes brain class, working memory budget, sensory ABI support, lexicon capacity, sleep stability, social motivation, and prior mastery.

## Teacher as world actor

The teacher LLM may privately decide what to do, but must act through an avatar or authorized world interface. It can speak, point, write, demonstrate, arrange objects, create tasks, and provide praise or correction. It cannot write to internal weights, hidden lexicon vectors, or action proposals. Limited direct bootstrapping reward is possible only under a logged experimental flag.

## Curriculum structure

Each curriculum unit should define prerequisites, lesson objects, teacher utterances, expected creature actions, verifier/evaluator hooks, failure modes, remediation steps, sleep replay requirements, delayed tests, and ablation tests. This is more important than generating many lessons. A small number of well-instrumented lessons is better than thousands of opaque LLM interactions.

## Example staged flow

A color lesson starts with two objects and clean labels. A role-reversal lesson tests “A chased B” versus “B chased A.” A writing lesson asks the creature to describe what it did, then checks whether the report matches the ExperiencePatch ledger. A math lesson uses objects first, then symbols, and is graded by deterministic verifier. A history lesson uses a simulated timeline and asks who, what, when, why, evidence, and uncertainty.

