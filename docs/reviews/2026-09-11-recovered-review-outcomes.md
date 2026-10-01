# Recovered review outcomes — 2026-09-11

Historical record consolidated on 2026-09-24 from the closed campaign.
Baseline: `8c2149654540f01ff353b16634699cc4e7c9b644`. These are recorded dispositions, not a new review
of current HEAD. Validation was **static source review only; compilation and
runtime were unverified**.

The campaign closed with 23 integrated repairs, two already fixed findings,
one finding judged not a defect, and two unresolved design questions.
[Original packets, detailed reviews, and campaign state](https://github.com/BobertNeek/A-Life/tree/67e5d7475f607bab3f84789d973e67ac9ddfaa8d/docs/superpowers/plans/2026-09-10-recovered-review)
and the [old bulletin](https://github.com/BobertNeek/A-Life/tree/67e5d7475f607bab3f84789d973e67ac9ddfaa8d/AGENT_BULLETIN.md) remain in Git history.

## Unresolved at campaign close

- **C001 — receptor aggregation:** ordering effects were established in
  `crates/alife_core/src/biochemical_graph.rs`, but intended body/endocrine
  reduction was unspecified. Neural additive reduction alone did not establish
  the intended rule. Determine the semantics before changing behavior.
- **C004 — EnergyStability:** the metric in `crates/alife_core/src/evaluation.rs`
  and `crates/alife_tools/src/era1_evolution.rs` was mean energy, with no specified
  temporal formula. Preserve persisted meanings until the intended metric and
  any migration are decided.

These questions were not resolved by documentation cleanup. Check current source
and later decisions before treating either as an active defect.

## Recorded dispositions

| Finding | Scope | Disposition | Integration commit(s) |
| --- | --- | --- | --- |
| C001 | receptor aggregation semantics | unresolved | — |
| C002 | Owning-state and biochemical graph tick consistency | integrated | [84a2d8c3](https://github.com/BobertNeek/A-Life/commit/84a2d8c3) |
| C003 | embodiment tick regression | integrated | [a9173383](https://github.com/BobertNeek/A-Life/commit/a9173383) |
| C004 | EnergyStability metric semantics | unresolved | — |
| C005 | departed patch retention | already fixed | — |
| N001 | Bit-exact newborn body initialization | integrated | [6e3dd8ed](https://github.com/BobertNeek/A-Life/commit/6e3dd8ed699354086d22a7e9c684e390e9b0105c) |
| N002 | organ energy debit | integrated | [c1670704](https://github.com/BobertNeek/A-Life/commit/c16707045ce0b6a50ed0ce6add880ed20071141a) |
| N003 | Elapsed organ upkeep cadence | integrated | [06307185](https://github.com/BobertNeek/A-Life/commit/06307185) |
| N004 | zero elapsed reactions | integrated | [cb97e0fd](https://github.com/BobertNeek/A-Life/commit/cb97e0fdde4e67c342ea3e1b4e13d4a2b8bb1e3f) |
| N005 | emitter cadence catch-up | integrated | [120c71df](https://github.com/BobertNeek/A-Life/commit/120c71dfc2640c1b3a019b06466cbb42c040b356) |
| N006 | bounded time fidelity | not a defect | — |
| N007 | Material balance in mixed reactions | integrated | [87a045e7](https://github.com/BobertNeek/A-Life/commit/87a045e7), [378b447b](https://github.com/BobertNeek/A-Life/commit/378b447b954d20fb1ac1f7e379c4213096e80097) |
| N008 | chemical range validation | integrated | [4b908941](https://github.com/BobertNeek/A-Life/commit/4b9089413b55a7fa5cc42e7d09a47988ba417020) |
| N009 | neuroemitter work accounting | integrated | [83e17615](https://github.com/BobertNeek/A-Life/commit/83e176156f1411415990710e380b8b8c356ec0da) |
| N010 | Ambient mating cues during motor actions | integrated | [71393bf7](https://github.com/BobertNeek/A-Life/commit/71393bf7), [f9cc324e](https://github.com/BobertNeek/A-Life/commit/f9cc324e3257b675d0cfa8ce2f7c5656517cf4c1) |
| N011 | parental cadence readiness | integrated | [3e3b1754](https://github.com/BobertNeek/A-Life/commit/3e3b175481020f45be6b492aeb42ad936acf44e3) |
| N012 | passive patch counting | already fixed | — |
| N013 | Bounded sealed-patch retention | integrated | [f91dd9d7](https://github.com/BobertNeek/A-Life/commit/f91dd9d7), [b11a1ed9](https://github.com/BobertNeek/A-Life/commit/b11a1ed9c2fa42ba8e32ecb8e2cf2dbb8d972f45) |
| N014 | truthful environment exposure | integrated | [2fd32c60](https://github.com/BobertNeek/A-Life/commit/2fd32c60ec81b55ff9c85fa597217cef3a5296e1) |
| N015 | grounded communication metrics | integrated | [5548be00](https://github.com/BobertNeek/A-Life/commit/5548be00181ffddea80b289ff19b59d1f2f15f3f) |
| N016 | grounded avoidance rates | integrated | [c6bc35cd](https://github.com/BobertNeek/A-Life/commit/c6bc35cdc68ab25bb3ec0642ae7c095e7ccffd92) |
| N017 | Atomic observation of finalized statistics | integrated | [d86ebe50](https://github.com/BobertNeek/A-Life/commit/d86ebe50dd77f311415a91ed11c74f1184489251) |
| N018 | Preserve selected Idle semantics | integrated | [9da8091e](https://github.com/BobertNeek/A-Life/commit/9da8091e), [c3e61cfc](https://github.com/BobertNeek/A-Life/commit/c3e61cfc305673751cacca52c8ddd85170f47028) |
| N019 | physical reach and ownership | integrated | [61fd9947](https://github.com/BobertNeek/A-Life/commit/61fd9947), [4e84084a](https://github.com/BobertNeek/A-Life/commit/4e84084a825e864de800181eee42614ed33599e6) |
| N020 | carried object motion | integrated | [eff576d8](https://github.com/BobertNeek/A-Life/commit/eff576d8282db734e07a42260f9f794e3e80c220) |
| N021 | Independent contact and transfer outcomes | integrated | [8f0996b4](https://github.com/BobertNeek/A-Life/commit/8f0996b48343e5e4cff43e4c43dac1a38d5b8cc1) |
| N022 | current interval velocities | integrated | [3acbbc1c](https://github.com/BobertNeek/A-Life/commit/3acbbc1c4e8b59cb49a4a29e2eb88d6f7fe21607) |
| N023 | Collision along the movement segment | integrated | [af2c66ea](https://github.com/BobertNeek/A-Life/commit/af2c66ea) |

N006's capped catch-up policy was permitted by AOA-TIME-004; that did not prove
numerical fidelity for arbitrary large jumps. N012 and C005 were already handled
by N013. Accepted static reviews do not substitute for executed regression tests.
