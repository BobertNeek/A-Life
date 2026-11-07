# Episodic outcome provenance: pickup is not a meal

## Why this was changed

The native continuous-life brain experiment exposed a memory-rule defect rather than lost learning. A harmful consumed meal remained stored, but a later painless pickup performed by the same Ingest action could become the current family expectation. The old selector compared numeric outcomes without retaining the physical phase, so a preparation could temporarily reverse the meal's remembered value. Repeating the experiment longer would not repair that rule.

The regression was reproduced on unchanged main `fc7dd78bf0b12f0e94a8fb872db230076d251286`: a harmful meal followed by an admitted, harmless Touch changes the recalled Ingest value. The new regression fails before the production patch and passes afterward. The Touch fixture has prediction error 0.2 so it is genuinely admitted; an earlier weak fixture was ignored by the low-information gate and was replaced before interpreting the result.

This is a grounded memory correction, not a new avoidance policy. It neither forces action selection nor supplies hazard labels, rewards, or synthetic experience.

## Scope and design

- Record optional exact physical contact provenance only for tracked-object Ingest episodes. Bind it to the representative primitive's concrete target and, for a joint bundle, its exact channel and target receipt. Missing, mismatched or ambiguous evidence remains Unknown. An aggregate Consumed on another target or channel is insufficient; two absent targets do not establish identity.
- For individual Ingest family recall, qualifying known Consumed records take precedence over Unknown records, which take precedence over known preparations. Within that scope the existing context similarity and causal recency rules remain. Repeated preparations are still useful before the first meal. Other families, untracked actions, target-latent cross-action observation evidence and category generalization retain their prior rules.
- Prevent numerical compatibility from merging different known phases. Scope bounded write and sleep-reversal shortlists by phase so more than 64 preparations cannot hide a genuine meal. Prioritize consumption before the runtime family shortlist, so the 16-record runtime cap does not discard it merely because preparations have high salience.
- Preserve true consumed-outcome reversals in both directions, including same-tick causal sequence order. At capacity, only a genuine direct reversal receives the existing targeted replacement privilege. Preparations retain ordinary bounded retention, not that shortcut.
- Preserve old-save learnability: a newer known Consumed may supersede a contradictory Unknown episode of the same identity under the existing 0.98 context similarity threshold. This is one-way admission/compaction precedence, never relabeling or merging Unknown into Consumed. Without it a full old bank, or sleep compression, could incorrectly resurrect an old harmful expectation after a genuine safe meal.

Each phase bucket still searches at most 64 records. A known Consumed write may additionally search the separate Unknown bucket, also capped at 64, for the legacy transition. This is bounded, not compute-identical to the old writer. Overall bank capacity and runtime retrieval caps are unchanged.

## Preservation and integrity

The optional field defaults to Unknown and is omitted from serialized legacy records. Unknown provenance contributes no additional canonical digest bytes. Existing bank JSON, portable JSON, record digest and active-bank digest goldens were captured before the change and remain exact. No float-based migration, eager rewrite, memory reset or checkpoint guard relaxation is used.

Known provenance adds a versioned, domain-separated digest extension. Altering it fails record and authenticated active-bank validation, including after rehashing the portable wrapper. Raw MemoryBank JSON and portable known-phase roundtrips rebuild derived indices and retain the learned records and family expectation.

Before editing production, a disposable six-step native CPU life was captured with nonempty learned memory on the old release. Its old test executable SHA-256 is `ad917e9187b76850e5a8d413c56bb177c0c1b68363b1a4dcc4295a9ca36d6bc0`. The patched release restores that checkpoint byte-for-byte, produces the same next decisions as the old executable, and produces identical complete next checkpoints for hot and cold restored hosts. Actor parameters, optimizer moments and ages remain unchanged. This is a real old-release continuation check, not a fixture created and restored solely by the new implementation. The fresh private state itself is not included in this source-only change.

No acquired private life, trained asset, Adam update, neural weight, world rule, sampling rate, save envelope or recovery seal is modified by this patch. Corrected future recall is an intended behavioral change, not a claim that every future decision remains identical to the old bug.

## Local checks

- Core library: 40 passed, 1 existing manual timing test ignored. Includes exact-channel/target, missing-target and ambiguous representative provenance tests.
- Candidate memory retrieval: 67 passed, 1 existing microbenchmark ignored. Includes 17 new phase, saturation, reversal, migration, authentication and serialization tests.
- Actual pre-change native checkpoint restoration/continuation: 1 passed, invoked explicitly with the external fresh fixture.
- Strict core Clippy, formatting and quick core-boundary checks passed.
- Full native collection/checkpoint suite: 8 passed; the historical-fixture test is ignored by default and was additionally run explicitly as above. Nonzero optimizer histories, source immutability, recovery, archive tampering and legacy plain checkpoint tests passed.
- Strict CPU Clippy passed with warnings denied. All native checks used the development profile on this constrained host; hosted optimized/GPU qualification is separate.

The full native collection suite uses disposable fixtures, including nonzero optimizer histories and immutable source/recovery checks. No result here establishes that a newly designed brain learns sustained avoidance. The experimental brain currently needs an explicit readout of acquired memory before this corrected family evidence can influence its actions.

## Reproduce the old-release check

1. In an isolated checkout of `fc7dd78`, append `capture-old-native-helper.rs` to `crates/alife_training/tests/cpu_collect_only.rs`. Set ALIFE_OLD_NATIVE_FIXTURE to a fresh, nonexistent local directory. Do not point it at a real retained life.
2. Run the ignored capture test with native CPU enabled. Preserve the resulting inputs, actor, checkpoint and next decisions locally. Do not add private fixture files to the repository.
3. On the patched release, set the same environment variable and explicitly run `pre_outcome_provenance_release_restores_exactly_and_continues` with `--ignored`. Ordinary CI does not pretend to recreate a historical release; that compatibility test is deliberately manual.

Use one Cargo job on constrained hosts. The engineering run reused the installed official Rust toolchain and serialized native builds to avoid memory pressure.

## Next experiment

Cross this memory-rule correction independently with an explicit experimental Ingest readout of acquired family valence. Keep genetic brains, physical consequences, exposure limits and sampling unchanged within the matched arms. Preserve reader-off and old-rule controls. A change in recalled expectancy is not yet a demonstration of learned avoidance, and repeated exposure must stop at the predeclared welfare limits.
