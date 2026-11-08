# Accepted-main integration

The frozen [mechanism results](RESULTS.md) describe the original candidate and
its event-gated successor. This note records a later integration without
rewriting those historical measurements.

## Source

- Accepted-main base: `f24122a60d54db6240fff7d6ff8fdad1d5494393`.
- Original final candidate: `6172e04b9325d4ebdc345a88ca7f0c700900590c`.
- Integrated source: `f2fea3499098c79ac02d85a49662f3073a64b1e6`.
- Integrated tree: `c065ab588234bd635347e8bad6d88d884b28e5d2`.

All 13 candidate files are byte-identical to the original final candidate. The
intervening main change added developmental Rest tests and documentation, not
runtime changes. This integration adds no new coefficient, neural route,
optimizer state, ordinary action interpretation or default activation.

## Checks on the integrated source

Rust 1.99.1, offline Cargo, one build job, incremental compilation disabled:

- Core mechanism: 2 passed.
- World candidate and existing Rest fixtures: 17 passed.
- Ordinary legacy-wire fixture: 1 passed.
- Canonical new-game fixtures: 9 passed.
- Accepted-main developmental Rest and portable continuation: 2 passed.
- Strict core/world all-target Clippy with `-D warnings`: passed.
- Workspace formatting, diff checks and clean source status: passed.

The 31 executed test functions include many internal assertions and bounded
physiology intervals; they are not 31 autonomous creature lives. No autonomous
study, new brain policy, GPU execution or practical self-care result was run.
The mechanism still requires a fresh inherited circuit and an explicit typed
world request. Ordinary Rest continues its existing bodily sleep behavior.

Local evidence SHA-256 identities:

| Evidence | SHA-256 |
| --- | --- |
| Core mechanism log | `7cfebad8e622942a98bb945d841e91e81b6b39b09889a9954030e320a77eb338` |
| World/Rest log | `662e502494c1e43fbd967d62653fcca12e56c3665efca5c09ea355732763e5fa` |
| Wire/new-game/development log | `ebbeb09f744dfdc1dbcfe44b4cec97c6773516da52f833bac7fc746d33e1bf91` |
| Strict Clippy log | `510b719a809e874976fcd34038a5806d3ab28a22e9ba0e34663e16c024458df3` |

Native executables and disposable save captures are not publication assets.
Exact-head hosted checks remain a separate merge gate.

## Why this belongs in the brain work

Tiredness relief and sleep-pressure relief must be distinct physical outcomes
before a neural controller can learn or arbitrate between them. The
[Creatures Genetics Kit manual](https://cdn.cloudflare.steamstatic.com/steam/apps/1838430/manuals/Genetics_Kit_Manual.pdf?t=1640163808)
and primary chemical catalogue support that functional distinction, not this
specific coefficient. The original
[weight-space programming paper and experiments](https://rikiyat.github.io/blog/weight-space-programming/)
remain the methodological reference: targeted repair, whole-network construction
and iterative feedback are distinct claims. Here the tested result is a body
mechanism. Neural action selection and sleep admission require their own
versioned integration and matched controls.
