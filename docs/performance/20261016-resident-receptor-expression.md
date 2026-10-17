# Resident neural receptor expression — 2026-10-16

Base: `69f00c05ec6774c3655ae87c5d4baa5df3861929`.
This is a CPU implementation improvement with identical receptor effects, not a
new cognitive policy or a change to the controlling v2.0 architecture.

## Work removed

Each awake row used to call `NeuralReceptorPhenotype::compile` before applying
its current biochemical receptor frame. Compilation recomputes the canonical
hash of the immutable brain phenotype, traversing every compiled synapse and
neuron to derive ten fixed gains. An N2048 foundation has 32,768 synapses and
2,048 neuron records. Repeating that construction every tick provides no new
information.

The app now compiles the expression once for each resident, at fresh admission,
curated cohort replacement, and exact restore. The ordinary path compares the
already-stored four-word source hash and applies the same
`NeuralReceptorEffects::from_frame` function to the current frame. It never
recomputes a phenotype hash. Receptor-frame validation and the authoritative
source-tick check remain unchanged. Expression compilation retains the complete
canonical check at every admission/restore boundary.

The private derived value is skipped by resident serialization. No save schema,
foundation payload, weights, optimizer state, episodic memory, phenotype hash,
biology, neural action, tick frequency, rollback transaction or work-accounting
value changes. The change is independent of trainer pipeline/replay optimizations.

## Immutability and replacement boundaries

- `BrainPhenotype` is a compiler-authored aggregate with private source fields
  and borrowed public accessors. Its deserializer validates its canonical hash
  and contracts. App code cannot mutate a resident's synapse/lobe/receptor source
  through those accessors
- Ordinary development updates `ResidentCognition.development` and authoritative
  world biochemistry. Production foundation construction deliberately normalizes
  chronology and dynamic gates; current chemical activations are still generated
  each tick from the organism's current biochemical phenotype/state
- Lifetime/fast learning and v11 growth/pruning change separate GPU learning,
  structural and dendritic state. The current structural phase clones the
  immutable phenotype and changes live ranges/slot/v11 state; those mutations do
  not change this expression's lobe shares or plasticity receptor profiles
- A genuine phenotype replacement, such as a new compiled lobe allocation,
  neuron count or plasticity receptor profile, must compile a new derived value.
  Reusing a cache with a different source hash fails closed
- Birth and re-admission use `ResidentAuthorityPlan::into_fresh_resident`.
  Curated replacement builds the candidate cache before residency publication.
  Restore builds it from the final validated authority, after any existing exact
  legacy Nano512 checkpoint adoption, before publishing the restored resident
- Research N2048-to-N4096 handoff exists in the shared session and is not wired
  into the production app. Any future app-level phenotype replacement must also
  reconstruct this value; this patch adds no new growth route or supported tier

This preserves the same-individual causal binding (AOA-INV-011), fresh derived
chemistry views (AOA-TIME-003), and GPU-authoritative neural execution. It removes
repeated implementation cost without changing biological capability
(AOA-INV-012). Whole-population performance claims still need the broader
measurements required by AOA-PERF-005.

## Verification

Six focused CPU regressions pass: complete effect equality for all three
promoted brain classes with changing frames/ticks; rejection and rebuild after
a valid source replacement; canonical phenotype JSON restore and corrupt-source
rejection; unchanged malformed-chemistry rejection; actual fresh-resident
construction, clone behavior and serialization exclusion; and stable production
construction across chronology inputs. The chronology check is not a live
staged development or fresh-process GPU restore test.

The existing CPU preparation suite also passes 19 tests (three timing tests
ignored), and both existing CPU newborn-admission/rollback fixtures pass. Strict
GPU-runtime library/test Clippy, formatting, core boundaries and 77 documentation
assertions pass. Exact source/executable hashes and raw samples are in the
[source-bound receipt](evidence/20261016-receptor-expression.json). GPU execution, target-machine
FPS/TPS, rendered acceptance and end-to-end fresh-process GPU save/load are
**Unknown** for this patch. No GPU, training or policy-collection run is launched.

## CPU timing and replay

Optimized Linux CPU component medians, per complete population tick:

| Residents | Recompile/hash + fresh effects | Retained expression + fresh effects |
| ---: | ---: | ---: |
| 8 | 14.993 ms | 0.000721 ms |
| 50 | 89.130 ms | 0.004117 ms |

All 10,440 timed complete effect rows compare exactly equal. These large local
savings remove the repeated canonical traversal, rather than simplifying neural
or biological behavior. Admission still pays the one-time compile cost. They
must not be presented as an FPS/TPS improvement or added to earlier component
receipts as a measured full tick. The processor is an AMD EPYC 9V74 cloud CPU;
the harness is pinned to CPU 0 and the core/harness use optimization level 3,
with debug assertions off. Local heavy work was paused for accepted timings;
other host tenants are uncontrolled.

The [component harness](evidence/20261016-receptor-expression-bench.rs) imports
production `neural_receptor_cache.rs` directly. Both modes use the exact same
core compiler/projection implementation and fresh receptor frames. It compares
full effect values outside the timer, with independently owned N2048 phenotype
records, 8/50 residents, 20 changing ticks per sample, nine alternating samples,
and two warm-up pairs. Setup/admission, whole cognition preparation, world work,
GPU execution, persistence and rendering are excluded. Output allocation is the
same in both modes and destruction/equality checking are outside the timer.

Build the core with a memory-bounded optimized configuration:

```sh
cargo --config profile.release.package.alife_core.codegen-units=256 \
  build --locked --release -p alife_core --lib --message-format=json > core.jsonl
# Set CORE_RLIB to alife_core's rlib filename emitted in core.jsonl.
rustc --edition=2021 -O docs/performance/evidence/20261016-receptor-expression-bench.rs \
  --extern alife_core="$CORE_RLIB" -L dependency=target/release/deps \
  -o receptor-expression-bench
./receptor-expression-bench
```

The optional app-library ignored test exercises the same component comparison;
its output is not a gameplay benchmark. The standalone optimized harness avoids
needing to optimize the entire GPU-enabled app test executable on the cloud
computer. Full symbols and LLVM optimization of this large core can exceed
cloud build memory; the receipt discloses the successful configuration and any
failed build attempt.
