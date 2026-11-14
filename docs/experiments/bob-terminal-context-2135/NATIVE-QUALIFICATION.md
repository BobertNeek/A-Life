# Native CPU qualification and channel observation

The original qualification base was `bf79c651794f02a6872d5e599ee20d500a7f0b85`.
Native tests were committed at `6f4c5a38`; integration commit
`e87639413a7341538a0007d8c56317b4a3224d47` includes main
`13f97750f3ea71c5797d4c69e674a75f83744422`. Results here are native CPU,
Rust 2021, rustc/cargo 1.98.1. No hosted CI or GPU qualification is claimed.

## Current-main integration

Combined source commit `f62b9055cc515cd14f3b60979859f83c0f2570a5` merges
main `fc12ee1658d9752decacef828c0f902a68cf0678` (#348) into observer fix
`90ee82084bcaa1f3355894a4bc168f08adc354b3`. Main's nine-file delta changes
only `evolve_pursuit.rs` and experiment documentation/assets/results. Core,
host and library source remain the qualified observer revision. Hold4 assets
are retained from main; Contact20 remains the accepted fixture and parent.
Its two hashes below were independently rechecked unchanged.

On that exact combined source commit these commands passed:

```sh
ALIFE_ASSAY_GIT_COMMIT=f62b9055cc515cd14f3b60979859f83c0f2570a5 cargo build -p alife_training --features cpu-backend --example terminal_context2135 --example evolve_pursuit
cargo test -p alife_training --features cpu-backend --test cpu_terminal_context2135 --test cpu_terminal_compat2135 -- --nocapture
cargo clippy -p alife_training --features cpu-backend --lib --tests --examples -- -D warnings
cargo fmt --all --check
git diff --check
```

Both examples built; all five native integration tests passed, with the same
one explicitly external old-executable fixture ignored in the ordinary run.
Strict checks passed. The unchanged 152 library cases were not repeated for
this example/docs/assets integration. Earlier receipts remain intact.
The subsequent qualification-note commit changes only this Markdown file.

Combined terminal example SHA256:
`5ca4279258ffb7b2a12ab0147db05bea8bbff3b22e88e322e3b0fc28e6301c10`.
Combined pursuit example SHA256:
`d852f6561be78085a95263bc93824b7d35bb33df2126d17332917fb9eb986e23`.

## Exact parents and retained baseline

The legacy bearing4 fixture retains foundation SHA256
`ec710f2176ae4cfccdd342953c7778385f1880efc7a0216ca5412aad68d6628c`
and complete genome SHA256
`5c63a675c89be9195de6b2bdedfc51a0f6f95fa11917ef8db6b75492bba025f7`.
Both independent channels are Locomotion and Manipulation; Orientation is not
independent. Foundation bytes alone do not establish that construction.

New functional observation tests use the approved Contact20 pair: foundation
`d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1`,
genome `557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189`.
The constructor binds the actual pair and compiled phenotype/input digests;
cross-parent combinations reject. Hold4 was not adopted.

Astra's source-only handoff `82b22e3519f4248ba66a80357c75e7cf5cb1a16e`
contains the exact v6 pilot requests/results, inspection and feedback aggregates.
Its four 256-step lives each consumed once but produced zero terminal sources:
Manipulation consumed while another family was representative. Each enabled
arm continued through 69 naturally occurring read-ATP abstentions. Those are
historical v6 measurements, not results of the new observer. The cohort was not
repeated here. Pet's measured loneliness relief in that handoff is not replaced
by a zero-need interpretation.

## Observation policy and compatibility

Recipe2 preserves the original representative observer and byte/seal rules.
An actual pre-change schema3/recipe2 checkpoint restored byte-exactly and
continued with no new observation route. Default-off retains schema2.

Fresh recipe3 stores channel-terminal observations separately from ordinary
episodic recall, in the same staged host transaction. Core authenticates the
sealed patch, exact finalized candidate key, committed Manipulation command and
that channel's Consumed target. World supplies per-channel physical outcomes;
its kinematic observations currently cover Locomotion only. No missing
Manipulation kinematic receipt is fabricated. Physiology remains the measured
joint transaction. Ordinary representative memory and neural credit are applied
once through their existing paths.

The collection has a cold duplicate ledger and a 64-observation bound. Additional
observation work is reserved before validation/copying. Unavailable observation
ATP/capacity produces typed abstention and paid bounded bookkeeping. The existing
head-read tariff and biological cost policy remain unchanged. Active, ZeroHead
and Mask have identical reads/reservations on the same retained state, including
an actual consumed source. Recipe3 never duplicates ordinary consumed sources
in its terminal readout.

## Commands and actual results

```sh
cargo build -p alife_training --features cpu-backend --example terminal_context2135 --example evolve_pursuit
cargo test -p alife_training --features cpu-backend --lib
cargo test -p alife_training --features cpu-backend --lib terminal_native_tests -- --nocapture
cargo test -p alife_training --features cpu-backend --test cpu_terminal_context2135 --test cpu_terminal_compat2135
cargo test -p alife_training --features cpu-backend --test cpu_collector --test cpu_collect_only --test cpu_backend --test checkpoint_validation --test cpu_experimental_graph --test cpu_signed_motor_lives --test cpu_independent_orientation
cargo test -p alife_world --test player_interaction_v1
cargo clippy -p alife_training --features cpu-backend --lib --tests --examples -- -D warnings
cargo fmt --all --check
```

- Full library run: 152 passed, 6 ignored. Two later parent/legacy tests also
  passed in the focused run: four observer tests passed, one external fixture
  remained ignored in the ordinary invocation and was explicitly executed below.
- Existing seven integration suites: 69 passed, 2 ignored. Native terminal
  integration: 5 passed, 1 ignored. A stale test expectation counting only the
  ordinary bank was corrected to include the new channel collection, then all
  five native integration tests passed.
- Baseline World player-interaction suite: all 9 passed. Final default-off
  comparison also includes the complete Praise/Scold/Pet/Slap branches.
- Contact20 consumed through Manipulation at interval4/sequence5 while Approach
  remained representative. Exactly one authentic channel observation was stored.
  Wrong channel/target, duplicate delivery and corrupted cold observation rejected;
  native nonconsumed toy contact produced no consumption record. Full cold state,
  next step and founder optimizer bytes matched. Later Approach read the source.
- Original host `2d196898bc65c822f19a311bfeb0bc252d5816e9` was compiled with the
  unchanged compatibility probe. Its actual executable rejected a real schema3
  checkpoint and enabled row wrapper. This is an executable result, not only a
  structural/projection test.
- Original versus final default-off: all 16 files, 13,719,215 bytes, matched
  exactly. Ordered transcript SHA256:
  `f7eb193f9bf5eee4b21aca3f5d04cca947b10a24ef00af3b613a040b19515ef0`.
- External recipe2 restore was explicitly run with
  `ALIFE_NATIVE_LEGACY_CP` and `--ignored`; byte equality and continuation passed.
- Strict Clippy and formatting passed.

The supplied genetic parents do not include historical Adam files. The native
parent roundtrip therefore preserves actual birth optimizer state, not invented
historical moments. Existing regressions separately exercise nonzero moments,
integer ages, frozen coordinates and real ordinary learning updates.

Capacity-above-64 read rejection and unavailable observation ATP/capacity remain
component-tested. A bounded 128-step native capacity probe reached only 18 records;
it did not establish a full native capacity crossing and was not retained as a
passing regression. The low-ATP native regression deliberately constructs an
ATP0.051 branch after real native memories; it is distinct from Astra's natural
v6 abstentions. No new feeding-selection winner or live-prey capability follows
from these fixtures. All raw CP, Adam, neural arrays and local transcripts stay
LOCAL and ignored; only source/tests and these safe aggregates are published.
