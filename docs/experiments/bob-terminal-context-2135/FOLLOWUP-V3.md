# V3 cumulative follow-up: retain acquired native lives locally

This review-only follow-up applies to published draft#345 at `a451394fcdd06aaaf61c6359728a45840451d654`. It includes the unchanged V2 finite-gain follow-up plus checkpoint retention. V1/V2 source packets remain immutable. No push or native World run has been performed locally.

The assay now accepts an optional evidence-root argument, defaulting to `terminal-context2135-LOCAL`. It creates that root as needed and adds a create-only `.gitignore` containing `*` if none exists. An existing ignore file is not overwritten. Each request must provide a unique simple `evidence_id`. Its directory is created atomically; any existing directory rejects before simulation. All evidence files use create-new writes and are synced. On Unix, their directory entries are also synced. Paths need not be absolute, specially named or outside a checkout. Regardless of location, raw evidence is LOCAL ONLY and must never enter Git, a Space backup or a publication allowlist.

Per request it saves:

- `REQUEST.json` and `PROVENANCE.json`: actual request, exact parent SHA/native digest/manifest, original factory seed, compiled phenotype hash and source hashes. Full native genome/development is retained in checkpoint world records.
- `FOUNDER-OPTIMIZER-BIRTH.json` and `FOUNDER-OPTIMIZER-FINAL.json`: full native founder optimizer state, asserted byte-identical because no founder/Adam update is authorized here.
- `PREPARED-BIRTH.cp.json` plus `RECIPE.json`: complete prepared native host state and optional new-head recipe before the first decision.
- `EVENT-*.cp.json` and matching event records after every genuine Consumed decision and the first informative terminal-context delivery. Event records retain the actual native decision/patch/context receipt, not manufactured evidence. A nonzero experienced desirability with genuine consumed sources counts as informative context even in the ZeroHead/mask arms; it does not alone establish a behavioral effect.
- `FINAL.cp.json`, `FINAL-RECIPE.json`, and `COMPLETED.json` only after final typed cold roundtrip succeeds and parent/founder optimizer equality checks pass. The CP is written before checking the roundtrip so a later failure does not discard the life.

Before each native step, after any prespecified environmental arrivals, the validated complete native CP is retained in evaluator memory. After a committed step it is refreshed before event serialization. A step/serialization/validation error or caught panic saves this state as `ERROR-LAST-VALID.cp.json` and writes `ERROR.json`, then reports failure. It does not retry, resume, overwrite or create another life. Early failures explicitly report that no native CP existed yet. Storage failures are surfaced with separate checkpoint/receipt-write errors; disk exhaustion cannot be claimed to have saved bytes that did not persist. A hard process/OS kill can leave only the earlier durable birth/event CPs and an incomplete claimed directory, which still must not be silently reused.

These checkpoints retain native fast/lifetime/eligibility and all host memory/biology/RNG state. Evidence serialization is evaluator I/O, not a creature sensory input or a neural-work exemption. No biological state, reward, action or brain weight is altered by retention.

## Remote run

```sh
cargo test -p alife_training --features cpu-backend --example terminal_context2135
cargo build -p alife_training --features cpu-backend --example terminal_context2135
# Root may be omitted; requests already carry distinct evidence IDs.
target/debug/examples/terminal_context2135 \
  docs/experiments/astra-pursuit-2026-11-13/pursuit-memory16.alife-foundation \
  /tmp/terminal-context2135-LOCAL \
  < crates/alife_training/examples/terminal_context2135.requests.jsonl
```

Use a new evidence ID only for a deliberately authorized new experiment, not to hide/retry a failure. Keep the aggregate JSON outputs separately from the LOCAL evidence. Do not upload the checkpoint/optimizer/event files. Full-host compilation and native controls remain remote qualification gates; no live-head benefit is claimed. The V2 note's bank>64/ATP prototype limitations and informative-postconsumption-read requirement still apply.

Local verification: 3 persistence component tests and strict Clippy pass for create-only directories/files, retained error-state bytes, missing-state reporting, and root/ignore creation. V2's 16 component tests/strict Clippy remain applicable; no gain/read/head source was changed in V3. Formatting and request/TOML checks passed. No Cargo target was used and no native World trial was started.
