# V2 proposal: unrestricted finite mutation gain, still unqualified

This is a separate follow-up to draft#345 at `a451394fcdd06aaaf61c6359728a45840451d654`. V1 source and manifest are preserved. Do not push or use this revision until the coordinator agrees the exact revision with the native-assay owner.

## Gain domain and numerical change

Gain64 was a search bound, not a necessary safety or architectural limit: output is already bounded at±0.5. V2 accepts every finite signed f32 gain, including ±f32::MAX, subnormals and signed zero. NaN and infinities still reject, including through the mutable setter/cold decoder.

For |gain|≤f32::MAX/2 the existing `(2*gain)*desirability` arithmetic is retained exactly. Above that magnitude only, evaluate `(gain*desirability)*2`: zero input cannot become infinity×zero, while a nonzero overflow safely saturates through tanh. Delta and gain-derivative bits are unchanged in the tested old[-64,64] domain. The derivative remains `desirability*(1-y*y)`; saturated derivatives are zero. Head logical work changes7→9 for the added absolute-value/comparison, on every active branch. The existing conservative host reservation already bounds this; no ATP tariff is relaxed. Active/ZeroHead/mask retain equal reserved reader/head costs.

The11-byte schema1 cold wire layout and old accepted recipe bytes remain unchanged. V2 widens its finite gain domain. Old V1 binaries reject new recipes outside[-64,64]; use the matching code revision with such a candidate. The complete candidate identity remains parent foundation plus additional head/config plus implementation revision, not the unchanged foundation alone.

## Full native construction lineage closure still required

The foundation SHA alone is insufficient. Both original `evolve_pursuit` and the new assay must use the same construction:

- `CanonicalNewGameConfig::phase3(2026111701,1)`; `Standard2048`; sensor profile from the exact pursuit-memory16 manifest (`GroundedObjectSlotsV1`).
- `create_canonical_new_game_with_n2048_experimental_candidate`; retain its actual native genome/brain-genome and development state, not a reconstructed approximate genome.
- `foundation_construction_development` using that brain genome, `BrainCapacityClass::n2048()` and tick-zero development; then `PhenotypeCompiler::compile_n2048_experimental_foundation_candidate` with the exact asset.
- Original recurrent-only mask and default Adam configuration; no PPO or optimizer updates during these lives. All parent genetic weights are asserted exact.
- Pin native canonical identities for genome/development, compiled phenotype/dynamics, decoder/motor layout and complete source asset. Capture actual native hashes/receipts in the remote test; no expected runtime hashes have been fabricated here.

When the remote full build is ready, run the same prepared mixed-life fixture on the original parent host and the patched host with the recipe absent. Compare full cold checkpoint bytes before the first decision and after each matched step, RNG counter/joint selections and log-probabilities, ordinary perception/context, neural/lifetime/eligibility, native work and biological state. No terminal field should appear on Off's wire. Hash matching compiled factory artifacts before comparing policy. The original `evolve_pursuit` baseline reproduction is an additional control, not a replacement for this same-fixture Off comparison.

Biological setup is deliberately separate from brain lineage. The prototype uses original body energy0.25 and declared600-tick physical lead-in. A longer lead-in is a separately declared matched assay condition across every arm, not another brain or a hidden hunger patch. Measure actual first-decision hunger and actual hunger delta after meals.

## Prototype limits and behavioral claim bar

Bank>64 abstention and the conservative ATP tariff are prototype limits. They may prevent any useful live read or impair pursuit. Do not describe the hook as a meaningful live learned-choice mechanism unless the remote assay actually admits at least one informative terminal read after genuine consumption: real source IDs/provenance, nonzero measured need-relevant consequence/desirability and active-head contribution, with corresponding ZeroHead and post-read source-mask controls.

Synthetic unit inputs and read-only old learner-bank probes are not this evidence. Repeated exposures/fresh-ID probes, successful preparation versus failed-contact controls, alias uncertainty, and counterbalanced prelearning appearance bias must be checked natively. Use postexperience changes relative to ZeroHead/mask, not one fixed-seed target switch or candidate-order artifact. No new World runs or Cargo-target competition occurred in this local follow-up; no behavior improvement or promotion is claimed.
