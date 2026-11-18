# Detached need-specific reader: native CPU primitive qualification

## Result and scope

This implements the design's smallest staged neural arithmetic test. It is a
Cargo integration-test target, not an exported core module, phenotype, decoder,
foundation asset, inherited production gene, optimizer, or living creature.
No production file, Cargo manifest, replay lane, memory ABI, checkpoint format,
old sensor offset, or acquired weight is changed.

Base: ordinary main `550f79ad1f765132c634e7a272da56b521b57ec0`, tree
`c4e5230799be59df95d468be1343c4549892851d`.

The 14 frozen sealed-source bridge evaluations and 330 eleven-lane boundary
comparisons passed, with measured maximum absolute native/reference error **0**
under absolute tolerance `2e-7` on this CPU/toolchain. Two additional ATP
above-target/crossing controls passed. This does not promise
bitwise identity on every architecture. The original ten scalar tests also pass.
See `RESULTS.csv` for the fourteen observations and the verification section for
exact test commands and further qualification status.

A positive readout is a locally favorable association, not measured reward,
action causal credit, selected behavior, or successful need regulation.
The `.15` loneliness / `-.007499993` delta gives native pre-confidence readout
`0.000045886267`; diagnostic multiplication by source confidence `.8` gives
`0.000036709014`. This scale is intentionally small, not inflated to force a
behavioral outcome. No action probabilities or actual creature ticks were run.

## Actual engine operators

`tests/need_reader_native_v1/native.rs` compiles five `NeuralProjectionSchema`s
with real signed `CooTile` / `SynapseWeightSplit` weights and a real
`DendriticBranchSet`. It executes:

1. `cpu_spmv_projection`: retain current x and compute x+d from separate raw
   lanes; `finalize_cpu_activations(Identity, [0,1])` bounds the hypothetical
   successor. The constant lane is only for the typed ATP target, never a
   positive reward floor or a comparator to a fixed current need.
2. Sparse role projection and `finalize_cpu_activations(Relu, [0,1])`: burden x,
   availability `max(target-x,0)`, informational zero pressure. Both paired
   states use the same role contract. ATP target is an explicit gene.
3. `apply_dendritic_conjunctions`: four `tanh(max(p-knot,0))*basis` branches on
   each of 22 targets, followed by identity activation. This is **88 actual
   engine dendrites**, with four per target, not eight on one target.
4. Sparse signed comparison and identity finalization: B(current)-B(successor)
   for each of the eleven lanes.
5. Sparse magnitude-weighted sum, then a separate sparse gain projection and
   bounded identity finalization for the diagnostic readout.

No scalar-reference pressure, burden, successor, association, or readout is
written into native activations. Scalar code is used only for input recipe
validation and as a separate post-execution comparison oracle. Intermediate
successor, pressure, burden, signed association and readout are compared.

All evaluations, including Unknown and gain 0, perform 5 sparse projections,
10 active microtiles, 112 sparse synapse evaluations, 88 branch/input evaluations
(176 `DendriticWorkReceipt` units), and 6 activation finalizations. The existing
scratch constructor allocates 512 cells, so these finalizations visit 3072
padded cells. At most 23 logical scratch coordinates are live simultaneously;
this is **not an allocated 512-neuron phenotype**, nor a claim that this network
fits the current creature's budget. No energy debit, recurrent dynamics,
receptors, leak, EMA, homeostasis or production scheduling is qualified here.
The number of positive/gated branches varies with data; evaluated branch work
and gain-paired work do not. A projection edge with zero gain is retained.

## Inherited recipe, roles, weights and identities

The reference-species fixture retains all eleven raw readings and deltas:

- Hunger, fatigue, fear, pain, loneliness and temperature stress are burdens.
- ATP is availability with proposed target `.75`.
- Curiosity, reproductive arousal and untyped extensions are informational. They
  have zero *innate signed-consequence readout*, not zero activity throughout the
  animal. Arousal/curiosity changes do not imply universal success or harm.
- Extension 0 becomes a burden only for the exact fixture binding
  `species22/play-stimulation/drive-extension0/boredom/v1`; assigning untyped
  burden semantics or the wrong binding is rejected. This string is synthetic
  test metadata, not a source-validation certificate for a living organism.

Four knots `[0,.25,.5,.75]` and basis weights `[.125,.25,.25,.25]` recruit more
pressure-sensitive branches at larger burdens while preserving relevance at
small pressures. All basis weights are nonnegative with sum <=1. Seven relevant
reference magnitudes are `1/7`; typed boredom adds `.08`. Equal magnitudes avoid
inventing an unmeasured need hierarchy, target label, or action-family preference.
The `.08` boredom prior reflects the reference phenotype's separate coefficient.
Magnitude L1 sum is capped at 2. Gain is bounded by `.35`, with final signed
ceiling `.35`. ATP above target earns no extra replenishment prize. ATP losses
that cross below target can be unfavorable. The target `.75` and gain `.35` are
**proposed inherited priors**, not fitted or established biological constants.

There are **22 unique numeric gene coordinates**: 4 knots, 4 shared basis gains,
11 lane magnitudes (including four informational zeros), 1 ATP target, 1 readout
gain and 1 ceiling. Eleven role tags, schema, foundation/chemistry strings and
optional extension meaning are identity metadata, not extra float genes. The
compiled representation contains 88 threshold/gain/input occurrences and 112
sparse edge weights; those occurrences are not 88 independent learned bases.
Structural +/-1/0 edges implement transport and differences.

Each logical branch identity names lane, role, current/successor side and basis.
Both sides resolve the exact same `Knot(k)` and `Basis(k)` keys from a single
map; tests check equal identities and bit-identical compiled values across all
44 pairs, and each recipe's digest includes role semantics and every numeric
gene. Reference digest (four u64 words):
`[6627187470255709903,11208214143037123844,3463037732579544687,16230909564613200750]`.

These are frozen inherited fixture parameters, **not** a tied-parameter learning
or optimizer contract. No acquired offset is reset or loaded. The runner accepts
only immutable circuit plus synthetic row, creates fresh scratch, and accepts no
persistent state. Repeated interleaved rows leave compiled coefficients identical;
an unrelated synthetic acquired-state sentinel remains byte-identical. This is
an absence-of-mutation boundary test, not a private-checkpoint restore test.

## Evidence transport and exact bridge

The qualified store implementation from `need-consequence-main-v1` commit
`493bb172da46add5f6d5da43dd375d5387116473` is copied into the integration-test
support directory only. Production main does not gain this module. The original
store test suite is excluded; store implementation semantics are unchanged.
Its sealed `Fixture` / `ExperiencePatchBuilder` source constructor is copied with
only test visibility/formatting changes. These synthetic contract fixtures carry
valid sealed receipt structure, not actual simulated life measurements.

Every bridge evaluation reads current raw drives from a later, contract-valid
frame and raw delta solely from its exact retained observation. It checks source
sequence/frame digest, before/after tick, attribution, one representative
observed-target Inspect association, and absence of additional selected motor
channels. Measured source-before values are read directly from the sealed source,
not reconstructed from current minus delta. At tick 12, exact recall sees the
source ending at tick 11. The same context at a later tick is constructed by the
test; it is not proof that an ordinary next-world state matches.

The seven rows are evaluated at gains 0 and .35 with unchanged scratch/work:

1. Loneliness .15 with measured -.007499993.
2. Loneliness 0 with measured 0.
3. Loneliness .001 with measured -.001.
4. Loneliness 0 with measured +.007499993.
5. Loneliness .15 with measured 0.
6. Same .15 context with no measured physiology: Unknown.
7. Query loneliness 0 against .15 source: exact miss, Unknown.

Unknown, measured zero, measured confidence zero and unsupported evidence stay
separate. Raw deltas, confidence, age and similarity are independently reported.
Unsupported rows may expose arithmetic diagnostics, but their supported output
is zero. The known/support masks and confidence multiplication remain explicitly
**host diagnostics**, not qualified native dynamic gates. The raw native readout
must not be attached to a policy. Confidence is applied once, after comparison;
changing similarity never changes delta, confidence or the native readout.

There is no source generalization/admission API here. A boolean test support flag
has no living-source authority. The earlier cosine admission of loneliness 1
from a source around .15 remains unsupported even if similarity is high.
Correct zero-pressure arithmetic does not solve that problem. No .025 local
matcher, world/life/chemistry identity bridge, multi-command source matcher,
developmental matcher or independent context matcher is claimed. Those remain
separate, unqualified gates; exact misses remain Unknown.

## Controls and verification

The native tests additionally cover all eleven lanes at raw readings
0/.001/.15/.5/.75/1 with deltas -1/-.05/0/.05/1, both signs, clipping, current-zero
no-phantom-relief, emergent burden harm, ATP bounds, informational retention,
source confidence applied once, unsupported suppression, opposite burden
cancellation, and low-vs-high fatigue/ATP/loneliness with fixed consequences.
The recovery row in that competition test is **hypothetical arithmetic**;
it is not a retained Rest observation. Existing Inspect evidence cannot teach
Approach, navigation or targetless Rest.

A fixed-reference pressure control demonstrates loss of current conditioning;
a wrong-need-lane permutation removes benefit under the actual zero pressure;
two independent candidate rows keep opposing outcomes rather than collapse into
a family-common correction. That is scratch row isolation, not a complete
two-candidate perception/store integration or a candidate selector test.
Alternate nonnegative basis recipes, zero gain, ceiling clipping, invalid
nonfinite/out-of-range input, negative/unbounded recipes, unknown role semantics
and the wrong extension binding are tested. No new target semantics appear.

Commands (serial CPU, Rust 1.99.1, jobs=1, incremental=0, dev/test debug=0):

```sh
cargo test -p alife_core --test need_reader_native_v1 --offline -- --test-threads=1 --nocapture
cargo test -p alife_core --test neural_projection_cpu --offline -- --test-threads=1
cargo test -p alife_core --lib dendritic::tests --offline -- --test-threads=1
cargo clippy -p alife_core --test need_reader_native_v1 --offline -- -D warnings
cargo fmt --all -- --check
bash scripts/check_core_boundaries.sh --static
git diff --check
```

Final checks: **19/19** detached/native-oracle tests, **11/11** existing sparse
projection tests, **4/4** existing dendritic tests, strict focused Clippy, workspace
format check, static core dependency-boundary check and whitespace check all pass.
An initial compile found COO local indices supplied as u32 instead of u8; that
was fixed. The first strict Clippy run found an unnecessary range loop; that was
fixed and the final test/lint batch rerun. No failing test or unresolved check
remains within this stated scope.

The scalar oracle came from design `reader_algebra.rs` SHA-256
`1dc36b721009e7da25cbc70f2ec8c1d1ef2fe9c3f7b7b19dbdcaee103a81932d`.
After applying the same rustfmt, it differs only in module documentation and
visibility; its arithmetic and original ten tests are unchanged. The copied store
implementation is byte-identical after the same rustfmt, with its original tests
excluded. The support sources remain test-only.

Only those targeted checks are asserted. No full workspace, CPU/GPU gradient
parity, GPU hardware, live CPU policy, autonomous interval, fixed Approach
world fixture, acquired checkpoint or learning experiment was run for this task.

## Methods note for Astra

https://rikiyat.github.io/blog/weight-space-programming/ is methodological
inspiration, **not evidence about A-Life**. Keep these distinct:

- Experiment 4 reports selective repair of 3.5% of a trained 171,664-weight
  control network, from 0 to 100% over 1,024 evaluations.
- Experiment 5 generates whole 20,486-weight feedforward game models from zero.
- Experiment 6 uses iterative feedback.

This A-Life result is engineered sparse/dendritic arithmetic with reasoned,
bounded inherited priors. It is neither learned living behavior, whole-brain
weight generation, selective repair of a demonstrated learned policy, nor a
whole recurrent living-brain result. Iterative feedback here corrected and
qualified a detached test implementation. It does not license a bigger claim.

Before persistent integration, require independent retrieval/source support,
complete world/life/genome/chemistry/lineage binding, a typed evidence/confidence
port, candidate scratch scheduling and budgets, tied-gradient/moment/age
semantics, CPU/GPU and learning parity, replay capture, save/load validation and
a new default-off foundation/decoder identity. Do not reinterpret head 5 or old
sensor/replay offsets, copy adult acquired state into genes, or attach this
readout as a host logit bonus. The candidate stays detached.
