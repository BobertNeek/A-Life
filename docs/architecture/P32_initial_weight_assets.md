# P32 Initial Weight Assets

Status: v1 optional tooling contract.

P32 defines a JSON asset contract for generated inherited initial weights. The
contract lives in `alife_tools` because generation and interchange are offline
research tooling, not gameplay runtime requirements.

The asset schema is `alife.p32.initial_weight_asset.v1`. Each asset records:

- schema name and version
- canonical brain class, brain class ID, and neuron count
- stable lobe layout hash and lobe ranges
- sparse `W_genetic_fixed` entries
- alpha mask payload metadata
- density and mask metadata
- provenance and optional external D2NWG hook metadata
- validation digest

## Inherited vs Lifetime Weights

Generated initial weights are birth inputs. They populate inherited
`W_genetic_fixed` sparse payloads for a species/template and may include alpha
mask metadata that gates later plasticity.

Generated assets must not contain:

- `W_lifetime_consolidated`
- `H_operational`
- `H_shadow`
- sleep-consolidated lifetime traces
- hidden teacher reward, action, or semantic-vector edits

Lifetime learning remains local to the organism after birth. The runtime formula
stays:

```text
W_effective = W_genetic_fixed + W_lifetime_consolidated + alpha * H_operational
```

P32 import validates that any consumed asset is genetic-fixed-only before it is
converted into the existing sparse projection schema.

## External D2NWG Hook

The contract includes optional hook metadata for external training/generation
scripts. The Rust runtime does not execute Python, load ML models, or require a
D2NWG dependency. External scripts may emit the same JSON schema, and the Rust
tooling validates/imports the result.

When no real D2NWG model exists, `alife_tools::p32_weights` provides
deterministic procedural templates:

- survival baseline
- curious explorer
- social learner
- language-biased lexicon
- neutral control

These templates are fixtures and development seeds, not claims of trained D2NWG
behavior.
