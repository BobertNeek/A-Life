# Wordless care dock boundary review, 2026-10-13

This is a bounded frontend implementation report, not a whole-system compliance
or causal-gameplay acceptance claim. Its source is the wordless-care-dock patch
over PR37 repair head `d36a3204ef5cfa23c1a6ca047dc1e1671066120e`, tree
`15637e9f3e6ab5b9aae9e6670468140ff2632cf1`. The handoff manifest binds the
complete changed bytes, full reconstructed tree and check receipts. The initial
PR37 head `8e4720873fe5990448e5ed38643defc3f8e21735` remains recorded as the
preceding source; the intervening repair only relocates unchanged UI assets.

## Requirements and source boundaries

- **AOA-INV-002 / AOA-INV-006:** `care_dock::dispatch_core` delegates existing
  food placement, physical ball offer and care-stimulus APIs. Physical returned
  identities indicate placement only. Touch and praise remain queued/unconfirmed;
  no tick, hormone rise or presentation inference supplies completion, learning
  credit, a creature action, or a favourable biological consequence.
- **AOA-INV-003 / AOA-AUTH-001:** selected care resolves the stable entity against
  the current canonical Agent and living registry record, checks body identity,
  and uses that body's current position. Cached selection tiles do not position
  creature care. Food intentionally keeps the preceding valid-reference-plus-tile
  eligibility, including selected object references.
- **AOA-INV-010:** missing runtime/target, rejected commands and accepted queued
  requests remain distinct. Exact error details stay in the existing F3 inspector.
  Player speech uses typed dispatch evidence; unrelated diagnostics or a successful
  unaided fallback cannot become a false rejection badge.
- **AOA-TEACH-004 / AOA-TEACH-005:** the UI opens and dispatches the existing
  teaching/speech paths without changing their text, addressee precedence,
  spatial-perception source, world authority or biological contracts. It does not
  write creature memories, concepts, weights or action credit. The pre-existing
  canonical care-stimulus mechanism is reused, not redefined here.
- **AOA-GAME-007 / AOA-OBS-004:** the five player controls use large, wordless,
  cream silhouettes and bounded evidence markers. The selected physiology
  remains read-only; its quantities and menu semantics are not redesigned.
  Small viewports prioritise the care dock and composer and temporarily hide
  colliding readouts. The existing full text composer chrome and explicit F1/F3
  explanations remain outside the wordless-dock claim.

## Verification boundary

Earlier care-dock candidates passed production-frontend all-targets type checking
and warning-denied library Clippy. Those checks remain historical, source-bound
evidence; neither was rerun as a fresh final-candidate gate.

On 2026-10-13, combined code source tree
`9e6054b6db37673eb299a347c5bf092a8c4744f6` was freshly compiled with locked
dependencies and `bevy-app,gpu-runtime`. It passed 131 unique CPU tests:
7 composer state-helper, 6 worker lifecycle/spatial speech, 6 keyboard opening
boundary, 17 neighboring lineage/speech, 93 renderer/care, and 2 scheduling tests.
The same reopened-draft test failed as expected on a separate negative-control
tree restoring the old unconditional draft clear, then passed again on the
unchanged final code. Rustfmt, static core boundaries, all 77 documentation
assertions, and the opening-source/Python-reference checks also passed. The
reference-model checks are not Rust runtime test execution.

These tests exercise production CPU state helpers and existing bounded fixtures.
The worker tests use a test-only synthetic translation callback with real worker
lifecycle and world spatial emission. They do not instantiate the GPU-only Bevy
input/completion handlers, drive the live composer, call an LLM service, or prove
GPU compositing, rendered caret visibility, neural comprehension, or causal
creature responses. No GPU, training, or live graphical qualification is claimed.

There is no per-care-command completion receipt in the reused path. Queued care
therefore stays unconfirmed, including while paused. This observation gap is
preserved explicitly rather than replaced by a success inference. No production
neural CPU fallback, GPU/training run or change to architecture authority is part
of this patch.
