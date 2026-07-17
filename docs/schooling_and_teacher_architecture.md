# A-Life Schooling and Teacher Architecture

**Status:** controlling schooling, nursery, and teacher boundary for the N2048
foundation program.

The master specification and ADR-024/027/028 control when older P-plan prose
conflicts with this document. Production cognition is GPU-authoritative. The
world supplies same-snapshot unscored candidates, validates actions, and seals
outcomes. A teacher, translator, semantic prior, verifier, or UI may not become
a second action policy.

## 1. Three Separate Sources of Support

A-Life keeps these systems distinct and independently ablatable:

1. The curated inherited foundation supplies robust perception,
   proprioception, movement, eating, resting, survival instincts,
   content-neutral memory mechanics, and language mechanics.
2. The internal SLM supplies weak, fading developmental priors through
   `SemanticPriorRequest` and optional surface translation through the separate
   `SpeechTranslationRequest` schema.
3. The external teacher is a world actor that teaches through ordinary hearing,
   vision, glyphs, gesture, demonstration, objects, social feedback, and world
   consequences.

Neither SLM schema is a teacher action channel. The teacher never calls the
semantic-prior interface. Curated foundation knowledge is not evidence that a
creature learned a personal fact. Every evaluation report identifies which
foundation, teacher state, sensor profile, and SLM assistance state were active.

## 2. Non-Bypass Rules

Creature-facing instruction is limited to normal world perception:

- spatial spoken tokens,
- visible glyphs and labelled physical objects,
- gesture, pointing, and demonstrable actions,
- visible or audible social feedback,
- reachable reward or punishment objects,
- measured action outcomes sealed through `ExperiencePatch`.

The production teacher API has no hidden concept vector, direct lexicon write,
raw weight write, plasticity delta, selected candidate, action score, entity
target command, private reward injection, or world-legality bypass. A teacher
may privately grade and plan a lesson, but its feedback becomes a world event.
Research that uses hidden bootstrapping is separately labelled and cannot
satisfy clean grounding, foundation promotion, language, or intelligence gates.

## 3. Limited Compositional Language Nursery

The canonical codebook defines pronounceable symbols and grammatical roles,
not inherited world meanings. `LanguageCodebookV1` provides stable logical
codes for a bounded compositional language, while a newborn starts with empty
personal noun, action, name, alias, and dialect bindings.

The language nursery teaches through normal hearing, object presentation,
action demonstration, and sealed world feedback. A lesson can present an
object, speak a token sequence from a spatial teacher avatar, demonstrate an
action, let the creature act, and expose the consequence. It cannot attach a
hidden meaning to the token.

Player teaching and peer teaching use the same nursery path. Unknown words
remain novel tokens and may acquire learned aliases only after grounded
evidence. Localized player text maps to stable token IDs without changing the
creature's internal codebook identity.

### 3.1 Nursery lesson record

Every lesson carries:

- lesson, curriculum, codebook, sensory ABI, and teacher IDs,
- prerequisites and readiness requirements,
- visible objects and tracked-object bindings,
- spatial teacher utterances and exact raw token sequence,
- demonstrations and legal unscored action opportunities,
- expected observations rather than required hidden activations,
- verifier criteria and exposure counters,
- remediation and transfer variants,
- delayed post-sleep assessment,
- teacher, translation, and semantic-prior assistance flags.

The lesson record references sealed experience sequence IDs. It does not store
an authoritative expected motor command inside creature-facing input.

### 3.2 Language foundation mechanics

Offline foundation curriculum may pretrain:

- token and phoneme discrimination,
- auditory sequence position and bounded utterance memory,
- name/addressee attention,
- turn taking and speech-act sequencing,
- grammatical-role binding,
- vocal motor control,
- content-neutral internal-state-to-role association,
- `what`, `why`, `self`, `yes`, and `no` protocol scaffolding.

Training randomizes token IDs, surface forms, object appearances, speakers, and
role assignments. Success therefore requires language mechanics rather than
memorization of public English words or fixed semantic categories.

### 3.3 Live grounding gates

Language evaluation runs with SLM translation disabled. A stage requires at
least 256 held-out episodes, at least 90% task success with an 85% lower-
confidence bound, and no more than two percentage points of regression on
locked stages. Language-specific promotion additionally requires:

- at least 80% correct word-object or word-action grounding after 32 paired
  exposures,
- less than 5% false grounding to unpaired categories,
- at least 90% literal narration agreement with the GPU-selected action or
  dominant drive,
- transfer to unseen surface words and token permutations,
- novel-speaker, delayed-recall, and peer-teaching transfer.

Assisted scores are recorded separately and can never substitute for these
unaided gates.

## 4. Spatial Hearing and Player Speech

Teacher, player, and peer speech all produce `AudibleUtterance` world events.
The event has a source position, source kind, optional addressee, bounded raw
tokens, confidence/noise inputs, and stable utterance identity. Hearing range,
occlusion/noise, creature hearing ability, and attention determine perceived
confidence.

A named player message targets the named creature but is still inaudible when
out of range. An unnamed message reaches all creatures able and willing to hear
it. Correct perception does not force understanding, agreement, response, or
action. The same rules apply to teacher requests.

`HeardToken` is perception-only. It may not carry a hidden concept ID, action
score, reward, preferred candidate, or privileged entity target.

## 5. Authentic Self-Report and Narration

The world exposes `Vocalize` as one unscored legal opportunity when cooldown and
energy allow. If normal GPU arbitration selects it, the GPU speech head emits a
speech act and up to six raw tokens. The world validates, charges, broadcasts,
and seals the outcome. The host never converts internal neural telemetry into a
sentence.

Self-report lessons ask the creature to vocalize after actions, drive changes,
or queried `what`/`why` prompts. The verifier compares the literal raw token
receipt with the sealed action/outcome/drive evidence. It grades fidelity; it
does not generate the answer. Nearby creatures hear the same raw tokens that
the verifier grades.

Translation for the player consumes only the raw receipt and grounded language
ledger. Developer views preserve literal gloss, rendered text, confidence,
model identity, and assistance state. Low-confidence translation displays
uncertainty instead of inventing a fluent intent.

## 6. Teacher Roles

Roles are separate capabilities even when one model implements several:

- Tutor: presents lessons and remediation through a world avatar.
- Examiner: administers held-out and delayed tests without teaching during the
  test window.
- Critic: turns private grading into visible/audible correction.
- Curriculum planner: chooses the next lesson from readiness and mastery.
- Verifier: uses deterministic tools for exact or causal grading.
- Translator: renders existing raw tokens for humans without creating creature
  content.
- Storyteller/historian: creates grounded narrative artifacts and demonstrations.
- Peer tutor: an advanced creature teaching through the same speech, glyph,
  gesture, and demonstration pathways.

Teacher-private chain-of-thought, answer keys, curriculum state, and model
history never enter creature perception or saves as hidden neural context.

## 7. Developmental Readiness

Advanced curriculum waits until the creature can track objects and speakers,
distinguish self/other roles, tolerate delayed reward, retain post-sleep
learning, and sustain the required working-memory and lexicon load. Readiness
binds brain capacity, foundation version, sensor and language ABIs, working-
memory budget, hearing/glyph support, sleep stability, social motivation, and
prior mastery.

Failure to meet readiness yields a typed deferred lesson, not a simplified
hidden injection. N2048 uses the limited codebook curriculum; open-ended
language remains a larger-brain research profile.

## 8. Staged Curriculum

1. Grounded perception and object persistence.
2. Orientation, locomotion, stopping, withdrawal, contact, eating, and resting.
3. Hunger, fatigue, pain, poison, hazards, and action cost.
4. Content-neutral working-memory and association mechanics.
5. Detours, reversals, delayed cues, unfamiliar edibility, and survival.
6. Speech perception, turn taking, name attention, grammatical roles, and vocal
   mechanics.
7. Live vocabulary grounding through objects, demonstrations, actions, and
   outcomes.
8. Self-report of actions and drives under randomized abstract token mappings.
9. Held-out environment, morphology, language-surface, and dialect transfer.
10. Writing and visible glyph use.
11. Quantity through manipulable objects, then symbols and exact verification.
12. Simulated history, causal evidence, science experiments, tools, social
    reasoning, and peer teaching.

Each stage defines prerequisites, trainable/frozen foundation masks when
offline, bounded exposures, held-out worlds, deterministic seeds, failure modes,
remediation, sleep requirements, regression locks, and assistance ablations.

## 9. Evaluation and Ablation

Every learned skill is tested with the relevant support removed:

- teacher absent,
- SLM translation disabled,
- semantic-prior gain zero,
- novel speaker and surface vocabulary,
- unseen object appearance/material,
- changed layout and morphology,
- delayed recall after automatic sleep,
- reward/hazard reversal,
- auditory-route or speech-route lesion,
- peer transfer without the original teacher.

Removing auditory routes must causally remove comprehension. Removing speech
routes must remove meaningful narration. A creature that succeeds only while
the SLM or teacher is active has demonstrated assisted performance, not learned
mastery.

Private grading is allowed. Exact graders consume world truth, sealed patches,
raw utterance receipts, and bounded statistics. Their results may select future
lessons or foundation candidates, but cannot retroactively alter the evaluated
action or outcome.

## 10. Sleep, Persistence, and Provenance

Lessons produce ordinary fast learning and automatic GPU sleep consolidation.
Assessment includes immediate learning, post-sleep retention, save/load in every
sleep phase, and exact-once consolidation. School state stores curriculum and
exposure provenance; it is not a substitute for neural memory.

Genetic founders start with empty acquired vocabulary and meanings. Explicit
mind-state clones may retain durable learned language and dialect while clearing
current conversations, working memory, and world-local bindings. Every export
identifies foundation, codebook, curriculum, teacher model, semantic-prior
assistance, translation assistance, sensor profile, adapter, commit, and tree.

No foundation promotion may contain personal names, individual episodic
memories, teacher-private state, world-local entity bindings, or SLM-authored
semantic content. Audited distillation must demonstrate improved SLM-disabled
performance through the normal GPU brain before it becomes a future foundation.
