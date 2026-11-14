# V4 cumulative follow-up: bind the complete latest working parent

Current parent is the #344 working child from main `2d196898bc65c822f19a311bfeb0bc252d5816e9`, as a pair:

- `docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-bearing4.alife-foundation`: SHA-256 `ec710f2176ae4cfccdd342953c7778385f1880efc7a0216ca5412aad68d6628c`.
- Its `.alife-foundation.genome.json`: SHA-256 `5c63a675c89be9195de6b2bdedfc51a0f6f95fa11917ef8db6b75492bba025f7`.

Both exact files were fetched from that commit and verified. The prior pursuit-memory16 asset remains an explicit one-head control, not this generation's base. V4 is cumulative against published terminal draft `a451394fcdd06aaaf61c6359728a45840451d654`; it includes the V2 finite-gain and V3 evidence-retention changes. Their frozen packets remain intact. No source or weight belonging to the accepted new parent is edited.

## Full construction, not weight-only acceptance

The new example takes foundation path, exported construction-genome path, and optional LOCAL evidence root. It verifies both input-file hashes before decoding. It loads the exact typed `CreatureGenome`, verifies byte-exact canonical serialization and its embedded foundation against the supplied asset, expresses that genome and uses the real `foundation_construction_development` helper. No ad-hoc chromosome mutation or weight rebinding is needed. Canonical factory seed remains 2026111701, count 1, Standard2048, manifest sensor profile.

The temporary canonical template is replaced with that exact exported genome/expression using the same World record and `genetic_fixed_digest` route as current `evolve_pursuit::prepare_founder`. Compiled parent weights are asserted bit-exact. The inherited body/genotype and two-head chromosome are not reconstructed approximately. The typed recipe now uses schema 2 and binds both parent hashes, motor-head count, compiled phenotype and compiler-input identities. Old enabled schema-1 terminal recipes are not silently migrated to another parent; preserve/reopen them with their original revision. Default-off None still adds no recipe to ordinary CP bytes.

Full genome verification occurs at enable and cold admission. The ongoing parent check uses the authentic asset digest, native compiled phenotype/input digests, profile and actual motor-channel layout; it does not rehash a 200 KB genotype during each decision's memory work.

## Native no-World verification completed locally

A current-Core native compile of the exact exported pair, using the source-identical production-class construction normalization, established:

- Genome JSON roundtrip byte-exact; embedded and external foundation byte-exact.
- GroundedObjectSlotsV1, 2,048 neurons, 32,768 unchanged supplied genetic weights.
- Actual motor channels `[Locomotion, Manipulation]`, head count 2.
- Phenotype hash `[8227526484390048264, 5321359027925582130, 15675904855675944572, 6368561322487548543]`.
- Compiler-input digest `[14503636050429498377, 16782976114342650278, 15437838106123747822, 12386633400297544944]`.
- Native checks reject a one-head construction genome and a compiled one-head phenotype using the very same foundation weights. Thus passing the weight-only check cannot accidentally admit the old one-head architecture.

These are measured native compiler identities, not fabricated compatibility digests. They do not replace remote full-host compilation, original-host Off equality or live behavioral qualification. The local probe made zero World calls. Eighteen context/head/admission component tests and strict Clippy passed; V3's three unchanged retention tests also pass.

## Normal low-resource admission does not stop the life

Insufficient residual head ATP now produces `InsufficientAtp`, while bank >64 produces `BankCapacityExceeded`. Both are explicit typed no-read/no-contribution results: no bank export, record search or head evaluation occurs, and the ordinary neural step plus existing learning continue. Empty new-head candidate receipts leave ordinary scores untouched. Invalid/corrupt recipes still fail closed.

The read/head tariff is not lowered. Bounded admission bookkeeping is reported and fully billed through ordinary cognitive-work accounting. Its dispatch-reservation debit is limited to residual ATP; any unfunded bookkeeping remainder is explicit in the receipt. Even zero residual ATP never authorizes a hidden free memory read or a destructive state reset. Active, ZeroHead and Mask share this admission policy. The assay reports insufficient-ATP and unfunded-bookkeeping counts. Component controls cover zero/low residual ATP, the exact affordable boundary, capacity abstention and matched control arms; real host continuation and biological accounting remain to be checked remotely.

## Required remote closure

1. Compile current #344 source and verify the exact parent pair and compiled identities above. Reproduce its unchanged native baseline with `motor_heads:2`, zero additional targeting/memory/readout mutations, and the exported child asset. A request that reloads only weights with one head is not a valid baseline.
2. On the identical prepared fixture, compare original current-main host against patched host with recipe absent. Compare full pre-first-decision and subsequent cold CP bytes, native selected joint action/log-probability/RNG, sensation, neural/lifetime/eligibility/optimizer, work and biological state. Both controls must use the same exported genome. No live Off-equivalence result has been fabricated locally.
3. Run enabled ZeroHead, Active and post-read Mask controls with identical full parent and declared lead-in. Use the new LOCAL retention protocol. Measure real first hunger and actual meal hunger relief; any longer lead-in is an explicitly matched assay condition, never another genotype.
4. Require an actual informative terminal read after genuine consumption, then compare postexperience behavior against ZeroHead/Mask. Bank >64 abstention and conservative ATP reservation remain prototype limitations. Distinct appearances still need native distinguishability/prelearning-bias checks and counterbalancing; alias remains an uncertainty control. No new behavioral success or promotion is claimed.

```sh
cargo test -p alife_training --features cpu-backend --lib terminal_context
cargo test -p alife_training --features cpu-backend --example terminal_context2135
cargo build -p alife_training --features cpu-backend --example terminal_context2135
cargo clippy -p alife_training --features cpu-backend --lib --example terminal_context2135 -- -D warnings

target/debug/examples/terminal_context2135 \
  docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-bearing4.alife-foundation \
  docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-bearing4.alife-foundation.genome.json \
  /tmp/terminal-context2135-LOCAL \
  < crates/alife_training/examples/terminal_context2135.requests.jsonl
```

Requests explicitly name `pursuit-twohead-bearing4` and use fresh `twohead-v4` evidence IDs. Provenance now saves both hashes, exact parent construction genome, native head/channel/compiler identities and source hashes. All acquired CP/optimizer/event evidence remains LOCAL; publish only the authored source and safe aggregate results. No shared Cargo target, hosted-CI success, new local World run or automatic retry is claimed.
