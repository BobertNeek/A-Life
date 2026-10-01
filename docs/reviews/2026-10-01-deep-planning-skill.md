# Deep-planning skill review — 2026-10-01

Retain the original method and make a focused workflow revision. The useful
core is ordinary baseline → structural abstraction → independent domain
questions → literal translation → comparison → bounded executable handoff.
The age of the model is not a reason to replace it. The recovered text contains
no GPT-5.4 requirement, model-version gate, or authored version number.

## Exact source and disposition

- Original: `Complete_Deep_Planning_SKILL.md`, skill name `deep-planning`, explicit
  command `/deep-plan`, created `2026-05-22T18:25:27.846916Z`.
- [Original Library download](https://chatgpt.com/api/library/files/libfile_f47486e6bc4c819195bc444d8f54e614/download).
  Library identity `libfile_f47486e6bc4c819195bc444d8f54e614`; stored file
  `file_00000000903871f5bbd26497a00c235e`; provider version ID is `null`.
  Full read: 755 lines, reported size 25,505 bytes, `has_more=false`.
- The original remains unchanged in Library. It was absent from the cloud skill
  catalog, installed executor skills, checkout `.codex/skills`/`.agents/skills`,
  and fetched repository skill history. The repository contained Graphify and
  spec-loop-ponytail, neither of which was substituted for it.
- Revised repository skill: [deep-planning](../../.codex/skills/deep-planning/SKILL.md).
  This is the 2026-10-01 adaptation of the recovered original, not an assertion
  that an older repository installation existed. No personal Library skill was
  replaced or installed through an unavailable skill-management service.
- Revision guidance: [OpenAI skill-creator](https://github.com/openai/skills/blob/main/skills/.system/skill-creator/SKILL.md),
  read on 2026-10-01. It was not installed; its official source was recovered.
  The official `quick_validate.py` was fetched read-only through GitHub; blob
  `0547b4041a5f58fa19892079a114a1df98286406`.

## Focused changes and rationale

| Original seam | Revision | Practical example |
| --- | --- | --- |
| Phase 10 assumes a downstream worker with limited judgment and requires a contract, logs, recovery sections, and a fixed giant template for every invocation | Assume a competent implementer; require essential evidence, milestones, validation, recovery, and receipt, with detail proportional to risk and existing tracking | Add six measured/hypothetical tasks to the existing roadmap and one bounded performance handoff instead of inventing a contract system |
| Baseline and comparison require many integer scores but do not require a causal performance budget | Require source-bound measurements, serial/parallel/wait/scaling terms, replacement cost, new bottleneck, and a falsifiable prediction; label scores as judgments | A 71% lower elapsed time in a synthetic repeated-query fixture does not establish 71% better game FPS; a faster renderer does not establish more cognitive ticks |
| Questioner payload names excluded source domains while requiring blindness to them | Preserve the strict blank-context rule; use a domain-native payload without those source-domain cues | Send the external expert only its literal problem; keep repository paths and baseline with the main agent |
| Tool-free isolation is requested but actual interface limitations are not operationally distinguished | Disable access where possible, otherwise prohibit tools and verify no calls; report the achieved boundary honestly | Fresh conversation via `fork_turns=none`, no project prompt, no source paths, and no tools used; general platform instructions still exist |
| Repeated fixed phase/template explanations increase attention cost; patching can encourage premature commitment to one mapping | Condense the workflow; keep competing causal explanations open until a distinguishing experiment; stop recursion when it adds no useful mechanism | Measure preparation and host wait on the integrated source before deciding all memory should move to GPU |
| Validation advice can be read as unconditional red/green TDD | Match validation to consequence; retain meaningful regression and crash/corruption/learning checks, avoid tests mirroring a reversible documentation edit | Run documentation and skill checks here; require candidate-specific memory and attention regressions for the proposed runtime change |

No safety instructions, authority boundaries, integrity requirements, or
blank-context requirement were weakened. Trigger scope, baseline-first order,
mechanism translation, standard/novel/hybrid comparison, local refinement,
metaphor removal, anti-bloat pressure, honest fallback, and self-contained
handoff remain.

## Representative application

The user requested significant A-Life optimizations. The retained original
method was run with one independent `gpt-6.1-sol` Domain Questioner in a fresh
conversation (`fork_turns=none`). It received a literal external-field problem
and the original role/behavior/output instructions, no chat history, repository
paths, baseline plan, or source mapping. It was instructed to use no tools and
used none. The interface exposes general platform instructions/tool capability;
this is conversation isolation, not proof of a sandbox with revoked capabilities.

Translated findings were then checked against actual source: perform only
attention-required preliminary work, finalize once after routing, measure
resource occupancy and waits, limit outstanding work, maintain individual
provenance, and report presentation independently from cognition. The source
also revealed repeated topology reconstruction and immutable phenotype
preparation, supporting additional hypotheses. The ordinary profile-first plan
was retained with these bounded changes, rather than a wholesale relocation.

The [50-creature plan](../performance/20261001-fifty-creature-optimization-plan.md)
is the representative handoff. Its arithmetic exposes why eliminating the
first full finalization alone is insufficient. No runtime implementation,
training launch, desktop control, or target GPU benchmark occurred in this task.

Validate the revised skill with official `quick_validate.py`, review the
representative plan against source, and run existing documentation checks.
Package metadata generation is unnecessary for this adaptation of an existing
named skill: the original has no `agents/openai.yaml`, and the revision adds
only the required `SKILL.md`. Future real uses can validate whether the shorter
workflow improves handoffs; this single application does not prove superiority
across other architectural or research tasks.

Validation completed: official skill validator passes; revised skill is 178
lines; documentation assertions pass 77/77; local links and timing arithmetic
check; `git diff --check` passes. Independent Sol 6.1 source review found no
P0/P1 issues and confirmed the bounded single-finalization seam, ownership
constraints, topology staging opportunity and evidence limits. Its elapsed-time
wording correction is incorporated. No runtime source changed.
