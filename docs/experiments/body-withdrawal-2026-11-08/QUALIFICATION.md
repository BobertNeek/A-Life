# Bounded physical withdrawal component

## Scope

This change adds an explicit body-only world API. Ordinary callers remain on the
unchanged, disabled response path. No production cognition caller is activated.
It is a qualified component for later integration, not evidence of normal-game
activation, GPU wake/sleep durability or an autonomous escape policy.

The two public entry points advance one authoritative biology interval and return
physical evidence. The staged entry point requires the enclosing caller to own a
complete rollback transaction. There is no voluntary command, candidate choice,
policy-credit identity, optimizer update or cognitive cycle completion.

## Physical rule

Fresh external contact pain produces one short translation away from credibly
localized contact. If acute damage has no horizontal source direction, the fallback
is backward along the current body heading. Chronic injury without fresh damage
or contact does not renew it. Coincident or vertically aligned sources use that
fallback; conflicting contact normals cannot invent a path. The response ends
when fresh input ends. It does not turn the body or choose a navigation target.

The existing locomotor capacity/effector gain, terrain resolution, swept solids,
carried grip reach and player-held restrictions still apply. External source count
is bounded at 256. Failure is atomic in the public wrapper. The body response can
interrupt physical sleep recovery, but it does not alter cognitive sleep state.

Stationary damage and response costs are composed before exactly one biological
advance. An organism already advanced through an action does not advance twice.
The ordinary damage-dose correction from #305 remains a separate prior change.

## Two reproduced defects repaired before publication

The initial extraction passed its old suite but missed two cases. Five additional
native fixtures on the unchanged kernel produced three passes and two failures:

- Moving away from an already overlapping peer was incorrectly blocked because
  the closest point included the initial overlap.
- A carried harmful object was incorrectly treated as a stationary external
  blocker of withdrawal from a different source.

The four-added/five-removed-line kernel repair reuses ordinary motion_sphere_entry
for the final sweep and excludes the organism's own carried hazard from external
blocker classification. Strict initial overlap may only separate monotonically;
inward motion, boundary tangency and unsafe new swept entries remain blocked.
Carried-pose validation is retained. No peer is moved.

Both original failing assertions pass after repair without test changes. Controls
also cover swept unrelated hazards, grip blockers, rollback, one biology advance,
unchanged peer/yaw, source bounds, zero locomotor capacity, unknown direction,
stop/recovery and portable continuation.

## Exact source and native qualification

- Ordinary base: b0b4e5ab85473f1ace63f0629ecf4704e7b0fb81,
  tree 0a34d588488c09f9a7fd4d8bc6115be61bdcd750.
- Initial extraction: 00507abe4fd0292881d467a9f6791ef53eac9aa7.
- Prerepair fixtures: d76e4ed39fab120b88e7c564c0e11638046d2edb,
  tree 12b2b5d80dcde77e6d89af497dec7cea5435aa70.
- Repaired native source: 23baecf19e92f42ca9511e07c1e77cce222d43f4,
  tree 60768b5b398cb9bbce916f4db944bf81efdeb5ab.

Official Rust 1.99.1, offline locked Cargo, one build job, incremental compilation
and dev/test debug symbols disabled. Copied source inputs were refreshed before
native compilation; build logs identify the actual checkout.

- Five added overlap/grip fixtures pass, including both unchanged prerepair failures.
- Body-only focus: 24 passed, no failures or ignores.
- Full alife_world: 587 passed, no failures, three existing manual tests ignored.
  The 24 focused tests are included in this total, not added to it.
- Strict all-target world Clippy, workspace formatting, whitespace and reverse
  patch checks pass.

The three ignored tests are the manual contact timing comparison, mating-query
CPU timing and extended headless soak. They were not executed or counted as passes.
No GPU device or existing private creature/checkpoint was an input. All runtime
changes are confined to alife_world; core/brain/save schemas, genes, learned state,
training, GPU evaluator and production call sites are unchanged.

## Integration still required

A future caller must keep response evidence separate from voluntary choice and
compose it into the same world transaction. Sleep interruption must preserve the
real consolidation request, staged output, generation lineage and durable commit.
This component alone does not establish that boundary. Default activation requires
that integration and its own tests; the present PR must not claim it occurred.
