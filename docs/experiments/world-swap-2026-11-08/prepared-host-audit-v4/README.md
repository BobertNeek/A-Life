# Prepared-host audit v4: bounded qualification passed

**Both world arms passed the fixed prepared-host audit. This qualifies the
tested instrumentation and its noninterference, not learning, autonomous
feeding, or the effectiveness of the atomic-swap world change.**

The result supersedes the earlier statement that prepared auditing was still
pending, without changing the [v3 archive](../receipt-repair-v3/README.md).
Frozen pre-execution package labels also remain historical: they are not the
status of this completed qualification. The native study main remains disabled.

## What was measured

The two world arms together completed:

- 30 zero-life unit tests and two exact prepared-fixture tests, with no native
  fixture failures or retries.
- Two fresh prepared hosts, two restores of their freshly made in-memory
  checkpoints, and four operational branches.
- 1,200 world-only preparation advances, eight ordinary host calls, eight
  audited host calls, eight immutable replay checks and eight strictly admitted
  projected rows.
- Four paired-state equality checks and four post-projection equality checks
  per world. Full canonical checkpoints and trainer state agreed, including
  initial body/biology, scene and sampling identity.

Each world produced four real decisions with nonempty candidate sets and
nontrivial neural activity. Each had two Manipulation channels and two actual
transfers. Safe-EAT candidates were present on three decisions while H was
held, but **none was selected: zero selected safe-EAT attempts and zero atomic
swaps in either arm**. Candidate presence is not an attempted action or a
demonstration that the obstruction was exercised.

The exact scalar result is [results.json](results.json), SHA-256
`660b1773811adfb7f8a05a84f18962714a222c7993c417c6b2d9695a75ed4ee3`.
Its `totals` object is the v4 batch. The separate cumulative audit field includes
an earlier audit batch with 16 host calls, eight replays and zero preparation
advances; it must not be mistaken for v4 alone. V3's two physical records, ten
scripted actions and eight world-clock advances are also separate, with zero
host calls, replays or preparation. Neither earlier fixture was rerun in v4.

## Design and rationale

The question was whether real, prepared CPU-host transitions could be audited
without changing the transition or the learned state. Each world used disposable
fixture seed `0x57535635` (`1465079349`), the frozen inherited asset and
`GroundedObjectSlotsV1`. It created one fresh host with exactly 600 world-only
preparation ticks, made its checkpoint in memory, and restored that checkpoint
once to form the paired ordinary/audited branches. It did not load a checkpoint
file, repeat preparation per branch, retune genes, inject memory, force actions
or sleep, or invoke a reserved study case.

At most four host steps per branch and four replay/projection checks per world
were available. Complete checkpoint/trainer equality at paired boundaries,
immutable replay checks and post-projection equality test noninterference on
those actual transitions. Ordinary internal transactional copies are not extra
fresh fixture identities. Living, health and injury guards run during
preparation and before subsequent paired steps; an error stops the fixture.

The four-step horizon was fixed rather than extended to seek interesting
behavior. Starting with no held object, acquisition and consumption require
separate single Manipulation-channel phases; this horizon cannot provide three
harmful meals. More candidate exposure or a longer life was not added to obtain
coverage.

Finite production-path diagnostics preserve stage/error categories and
requested/returned preparation, host and replay counters, plus projected and
collected row counts. They do not export arbitrary native error text. A
requested call that did not return remains uncertain; it is not reported as
zero exposure. The prepared fixture uses a stricter closed event reducer,
durably replacing one bounded scalar progress snapshot before and after each
operation and writing a bounded partial receipt on return. A hard crash leaves
the last snapshot; it does not fabricate completion or resume execution.

## Defects found and fixes qualified

The [v3 repair](../receipt-repair-v3/README.md) already corrected the projector's
use of a Locomotion-only observation as Manipulation execution evidence. V4
retains the actual per-channel success receipt and its consistency checks.
That general defect does not identify the original failed study event.

Source and synthetic regression review of v4 also caught two ways that a
partial receipt could overstate evidence: a transfer count one row ahead of
collection, and completed fresh construction without all 600 preparation
returns. Evidence counters now come exclusively from collected projected rows,
and dependency checks reject impossible operation ordering. A projected but
uncollected row supports no behavioral claim. Identity/activity flags remain
separate and cannot be promoted merely because another stage completed.

The enclosing binder and dispatcher check the materialized source-tree closure,
the frozen package and executable identities. They execute the verified Python
source bytes, and validate dependency relationships in a crash snapshot. Native
collection uses bounded field allowlists; the Python validator checks exact
scalar keys, finite values, enums, ranges, physical receipt semantics, ownership
and physiology before admission. Synthetic tests cover corrupted and partial
cases. No native crash was deliberately induced, and the successful fixed
fixtures do not establish that every failure category has occurred in a live
host.

Every native receipt remains `PARTIAL`, `fixture_passed`,
`study_completed=false` and native `strict_validation=pending`. The enclosing
verified Python validator strictly admitted the eight rows without changing
those labels or turning them into completed study cases.

## Limits and current boundary

- This is offline CPU qualification, not GPU gameplay or performance evidence.
- Equality is established at the tested boundaries, not for every possible
  trajectory. Selected safe-EAT and swap coverage is absent, so the result does
  not compare obstruction, learning or feeding outcomes between worlds.
- The world factor is a bundle of world/grounding changes. No isolated causal
  claim about atomic swap alone follows from these fixtures.
- The entire old 22xx allocation remains closed without retry. The original
  failed event/tick and its uncertain primary exposure are not retrospectively
  resolved by these later tests. No historical closed-study rows are reused.
- The fresh 23xx comparison is a separate disabled, unrun proposal. No new study
  has started. These qualification results do not activate it.

## Source and recovery map

- [common-audit-kernel.patch](common-audit-kernel.patch) is the exact frozen v4
  common source, including the disabled example, finite diagnostics, prepared
  fixture and native test source. It is archived, not applied to production.
  SHA-256:
  `56667f16b701f20e9de6ce58fe00c1e1a5888e67f1b82c833a8c990f06ff2a42`.
- The byte-identical
  [world-factor.patch](../failed-reserved-attempt/world-factor.patch) is reused
  from the earlier archive. SHA-256:
  `5cc4ea55ff8505bff852291ad0037bf05b7d1b1010a8db59cd264584332b4b2c`.
- [source/report_contract.py](source/report_contract.py),
  [source/prepared_fixture_contract.py](source/prepared_fixture_contract.py)
  and [source/diagnostic_contract.py](source/diagnostic_contract.py) preserve the
  exact scalar admission and diagnostic contracts.
- [source/bind_compile.py](source/bind_compile.py) and
  [source/dispatch_fixed.py](source/dispatch_fixed.py) preserve the exact
  qualification wrapper source for inspection. They are historical scripts,
  not a self-contained replay package or instructions to run new fixtures.
  Do not execute or import the dispatcher to validate this archive.
- [provenance.json](provenance.json) records curated source/plan/package hashes,
  setup, source path hashes, verification and limits. Original package metadata
  is bound by hash rather than copied. [results.json](results.json) is the exact
  aggregate scalar result, including historical executable and receipt hashes;
  those payloads are not included.
- [SHA256SUMS](SHA256SUMS) covers every other file in this directory.

Source reconstruction starts from base commit
`20bcb45104e5bec688b01abefa96cdf95bdcee60`, tree
`c644058f92ec5af185ba9ae27410a7989d57daa2`. Applying only the common patch gives
old-world tree `29ad675081fa724ba8802af56ee6fdb93abf6ce1`. Applying both patches
gives new-world tree `c0a3db9468bafd7617f630ba4ba56d05cb7bd26a`. The patches
commute exactly, and the inherited constructor matches the frozen base source.
These are tested source identities, distinct from any documentation commit.

During archiving, source-only checks reverified all 28 frozen package hashes,
the two reconstructed tree identities and manifest hashes, all 17 changed-path
hashes, patch commutation, constructor identity and disabled main. All 55
synthetic Python contract/process tests and 15 static source tests passed.
Copied Python files parsed, and aggregate/per-arm scalar counts agreed. The
separate completed source/receipt review also verified actual executable and
compile/command/attempt/start/return hashes, and independently admitted both
native receipts through the exact pure scalar validator. Archiving did not
repeat those binary or raw-receipt reads.

This archive contains source and scalar evidence only. It contains no private
state, trained checkpoint, memory bank, projected row payload, raw native log,
executable, asset or credential. Archive validation ran no native build, native
fixture or world life. Runtime source, genes, workflows and all previous archive
bytes are unchanged. Reading this result cannot recover omitted creature state
or turn unknown historical exposure into an exact measurement.
