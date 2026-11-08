# V7 CPU/core interruption qualification

Frozen source: /workspace/scratch/ee5e08a3ca9b/motor-reflex-cognitive-v7
Commit3e7239c6cd9b4a34d199a4e513ea2559cb7d748b, tree8fd3ca2375e4613b2dea5997b8b3964ba7c6b633, base68eaeaed2b9e4f5c07f3fb129f6ddd4afd417913.
Delta patch /workspace/shared/cognitive-interruption-cpu-qualified-v7.patch SHA256298833de61a5657dcaab12147fc58b7956de6e51f7234053372a2b63e5506190.

## Implemented and measured
- Explicit interrupted phase tags6–9 and local SleepState schema3. Existing schema2 bytes, old phase discriminants and literal nested bincode sentinel remain unchanged. No appended nested sequence field. New malformed schema3 cannot invoke old missing-cycle migration.
- State preserves original entered tick, cycle identity, completed count, original sleep trigger and full pending/prepared/submitted/completed/committed payload. Ordinary advance cannot complete interruption; forced-sleep cannot clear it. Resume returns to original phase and restarts dwell conservatively.
- None or genuinely Committed allow neural writes while interrupted; frozen intermediate stages permit only real driver drainage. No new intent during interruption, no fake commit or duplicate completion.
- CPU candidate mode uses actual action/body damage after the single world biology boundary. Quiet intervals resume; saved interrupted states also release safely under ordinary step() without requiring the caller to repeat a mode flag. No-damage ordinary/default checkpoint remains exact.
- Native CPU full suite32 passed,0 failed,1 existing explicit replay-benchmark ignore. Includes actual recovery sleep→acute physical response→one affordable aroused decision→same-cycle resume→one genuine consolidation; exact save/restore continuation; ordinary-mode release; quiet exact no-op; duplicate-credit/export controls.
- Native core sleep contracts14/14, deterministic runtime scheduler9/9, strict core/runtime/training library+tests Clippy, workspace fmt/diff pass.

Logs under /workspace/scratch/ee5e08a3ca9b: reflex-cognitive-core-final-v7.log, reflex-cognitive-scheduler-v7.log, reflex-cognitive-cpu-final-v7.log, reflex-cognitive-clippy-retry-v7.log, reflex-cognitive-fmt-v7.log.

## Boundaries
CPU natural execution covers None/Committed, not a real frozen GPU optimizer job. Scheduler drain tests use a deterministic driver; they are not device-backed GPU transaction qualification. GPU arousal activation is still OFF; journal, authority, Completed permits and dispatch gates need separate work. Visual labels are source-integrated but not claimed as a full GPU-runtime native qualification at this stage.

Repeated actual fresh damage may sustain arousal with at most one real policy decision per interval. This is not a one-decision-total cap. No scalar chronic pain renews it. Release follows a quiet interval, preserving valid sleep/rest. The transition's existing trigger field retains the original sleep cause; the separate physical response receipt and interrupted phase identify acute arousal.

## Failures retained honestly
Initial legacy literal fixture assumed old schema1; actual legacy is2. Corrected expectation and introduced distinct new3; final old-wire guard passes. First CPU run caught an unnecessary sleep-seal timestamp rewrite even with no damage. Production fixed to reseal only real transitions; unchanged guard passes. Reviewer found ordinary restored mode could latch interrupted phase; state-authoritative release and actual restore→ordinary regression now pass. Initial strict Clippy found an existing redundant borrow in our duplicate-credit fixture; one-character cleanup qualified on final full CPU rerun.

The raw first CPU failure log includes full synthetic checkpoint assertion arrays, so it must NOT be exported in source/scalar packages. Use the concise failure summary here, not that raw log. No private lives or checkpoints were read.
