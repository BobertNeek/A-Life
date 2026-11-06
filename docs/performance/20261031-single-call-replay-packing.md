# Single-call packing of frozen replay weight offsets

The production packer now copies each tick's contiguous effective-weight offsets
through one checked destination slice. It keeps the existing zeroed Vec and
copies the same raw float bits into the same words. This removes repeated scalar
indexing across 32,768 offsets per N2048 tick without retaining cohort packets.
Only this dense-offset copy changes in normal code; validation, headers, reserved
lanes, speech initialization, GPU ABI, forwards and optimizer/state handling are
unchanged.

The bounded cloud measurements cover CPU host serialization used by offline GPU
replay. Device execution and end-to-end training times are unmeasured. In the
release-like comparison, 512-tick packing took 33.020 ms for the frozen original
body and 27.126 ms for the actual production candidate (17.8% lower); the
overlapping validation/packing/header subset took 38.857 versus 34.901 ms (10.2%
lower). Both implementations requested the same single zeroed allocation and
had the same tracked packet-allocation peak.

[Raw trials, exclusive-stage samples, memory observations, commands and source/binary hashes](20261031-single-call-replay-packing.json)
retain checked and release-like runs separately, including the original actual
production measurements made before implementation. Base source is
`1d58e143b1b35128bedd7f15724147b325e4421c`. The fixture comes from the eligible
packing assay `893e6395`; no held work is incorporated.

## What the old packer spends time doing, and why

An N2048 terrain fixture (seed 99) has 2,048 neurons, 32,768 synapses, 17 routes,
four microsteps, 25 candidates/tick, 64 canonical initial dendrites with 128
inputs, nonzero periodic inputs/private offsets, and no structural edges. Replay
shapes are 512/0, 1/0, 256/0 and 384/128 ticks/burn-in. These are production data
types and packing code with synthetic contents, not an acquired trajectory.

The following are exclusive **median spans inside a test-only instrumented copy
of the exact original packer**, for 512 ticks. Every output is checked against
the actual original production packer. Instrumentation and compiler context can
change costs; these spans are not an exact decomposition of a separately timed
production call. Per-trial spans plus timer/bookkeeping remainder reconcile in
the raw data. Their medians need not sum to the median enclosing time.

| Exclusive old-packer span | Checked ms | Release-like ms | Purpose and necessity |
| --- | ---: | ---: | --- |
| Zeroed allocation call | 0.015673 | 0.015473 | Supplies the complete contiguous upload layout, including reserved zero lanes. This is allocation-call time; lazy zero-page/first-touch work is charged to later writes. |
| Encoded inputs | 1.648986 | 1.593441 | Copies captured sensor inputs required for conditional replay. |
| Candidate records | 2.084696 | 1.759905 | Supplies actual decoder features, family and innate bias; untouched padding stays zero. |
| Context and route flags | 0.213299 | 0.226784 | Preserves captured projection gain, threshold, microsteps, route enables and structural count. |
| Dense effective-weight offsets | 32.297783 | 30.378848 | Copies required frozen lifetime/fast-weight context. The bytes are necessary; repeated per-word checked address arithmetic is avoidable. This span also includes its first-touch page work. |
| Structural edges | 0.012882 | 0.012639 | Fixture has no edges, so this is empty-loop/timer overhead. Nonempty-edge correctness is tested; its performance is unmeasured. |
| Initial neural/homeostatic state | 0.013801 | 0.013661 | Establishes the detached replay boundary. Its cost is small here. |

The 512-tick packet is 85,395,456 bytes, of which 67,108,864 (78.6%) are dense
offset words. The original loop performs 16,777,216 indexed assignments. The
candidate still reads/writes all those required bytes, using a checked slice and
`copy_from_slice(bytemuck::cast_slice(...))`. The source slice is borrowed: there
is no intermediate Vec or float arithmetic. Code inspection confirms a memcpy
call in the candidate. The zeroed allocation is retained; changing to uninitialized
storage or explicitly writing all zero gaps was not justified by this assay.

Before the change, actual production / frozen reference / instrumented reference
512-tick medians were 36.819 / 36.066 / 36.476 ms in checked mode and
32.195 / 33.694 / 34.094 ms in release-like mode. Thus the instrumentation control
differs from the frozen reference by about +1.1% / +1.2% for this shape, rather
than measuring pure clock overhead. Across the larger shapes, the release-like
frozen reference differs from actual production by 3.3–5.4%; the one-tick case
differs by 26.7%. Compiler-context effects and tiny-case overhead therefore
matter. The primary comparisons below use the two bodies in the same binary;
the separate-build before/after runs are retained as controls, not a matched
speedup. Component spans must not be added to those production totals.

## Actual production candidate comparisons

Each mode receives two warmups and eleven timed trials per shape. Trial order
alternates forward/reverse. A black-boxed function pointer calls the actual
production packer or the frozen original body. Full output-word equality is
checked outside **every** timed span; output destruction is also outside. The
frozen reference's body is verified identical to the original, ignoring whitespace.

| Ticks / burn-in | Checked reference / candidate ms | Release-like reference / candidate ms | Release-like packet-time reduction |
| --- | ---: | ---: | ---: |
| 512 / 0 | 33.971520 / 27.306373 | 33.019743 / 27.126051 | 17.8% |
| 1 / 0 | 0.028163 / 0.011728 | 0.021292 / 0.011247 | 47.2% |
| 256 / 0 | 17.070383 / 13.687634 | 17.420833 / 14.090673 | 19.1% |
| 384 / 128 | 25.230969 / 20.389136 | 26.292292 / 21.430879 | 18.5% |

The one-tick result saves microseconds and should be read with the calibration
above. No aggregate training percentage is inferred from it or the larger rows.

A separate overlapping subset performs fresh `validate_for`, packing and fresh
headers with an exact integer age beyond 2^24, then checks both complete outputs.
It excludes layout construction, output destruction, device resource allocation,
queue writes/staging, speech upload, encoding/submission/waits, neural forward,
backward and Adam. It is not complete `prepare_replay` or PPO time.

| Ticks / burn-in | Checked reference / candidate subset ms | Release-like reference / candidate subset ms |
| --- | ---: | ---: |
| 512 / 0 | 40.378618 / 34.613500 | 38.856701 / 34.900503 |
| 1 / 0 | 0.089135 / 0.071107 | 0.090016 / 0.079931 |
| 256 / 0 | 19.905976 / 17.191516 | 20.115033 / 17.342665 |
| 384 / 128 | 30.703093 / 25.406384 | 30.390101 / 27.271772 |

Hardware is the cloud AMD EPYC 9V74 VM, five visible vCPUs, four-CPU quota,
16 GiB memory, unpinned CPUs 0–4 and Rust 1.98.1. Checked mode uses test profile,
training opt-level 3, normal debug assertions/overflow checks and core/world/
backend/runtime opt-level 2. Release-like mode changes only the training package's
debug assertions and overflow checks to false, retaining opt-level 3. It is not
an actual release application or GPU build. VM contention, frequency, allocator
and page history are uncontrolled; raw min/max and all trials are available,
without a formal confidence interval. Counters are disabled during timing, but
the candidate binary retains the allocator wrapper's TLS check; no unwrapped-
System overhead control is claimed.

## One-call memory and exactness

For each build, two fresh processes per mode and per 512/384 shape measure the
packet while it is held, then after it drops. Source fixtures and metadata stay
alive. Both modes make exactly one zeroed allocation, zero ordinary allocations
and zero reallocations inside the packer. Tracked live bytes return to zero
after output destruction. Peak requested packet bytes are identical:

| Shape | Original peak bytes | Candidate peak bytes |
| --- | ---: | ---: |
| 512 | 85,395,456 | 85,395,456 |
| 384 | 64,052,736 | 64,052,736 |

Release-like packet-held smaps RSS was 163.000–163.025 MB for the 512 baseline
and 163.070 MB for the candidate; for 384 it was 125.452–125.780 versus
125.448–125.776 MB. Checked-mode observations are also retained. These small
process differences do not establish a residency reduction. No cohort packet
survives the call's output lifetime.

Rust allocation requests exclude allocator rounding/arenas, fixture/sampling and
other threads. Linux status/smaps RSS/PSS/private-dirty include the whole process;
VmHWM includes initialization and approximate accounting. They are sampled
separately while the packet is live and after drop, not a strict continuous peak
bound or Windows working-set/pagefile admission measurement. Both modes and both
profiles produce identical full-packet SHA256 values for each memory shape.

Correctness comparisons include 324 valid combinations: N512/N1024/N2048,
object-slot/terrain profiles, three lengths/burn-ins, three dendrite topologies,
1/25/32 candidates, and empty/nonempty structural context. Whole-word comparison
and explicit target/speech zero-lane checks preserve reserved initialization.
Twelve raw float-bit patterns exercise signed zero, subnormals, finite extremes,
infinities, positive/negative NaN payloads and a signaling NaN at first/middle/last
offsets across three ticks. Raw private serialization preserves these bits;
public validation still rejects nonfinite replay inputs. Fourteen invalid cases
are rejected before invoking either packer in the validation wrapper. The
production validation call sites and ordering are unchanged by the source diff.
These are adapter-free serialization tests, not a GPU session/error-order test.

The CPU foundation workflow explicitly runs the new regression group without an
ICD. Focused and manual comparisons pass in both profiles; full applicable local
all-target results are 199 passed, zero failed and five ignored across sixteen
targets. Strict Clippy, the default-feature library check, formatting and the
exact CI regression selector pass. Commands and receipts are in the raw evidence.
The change preserves payload size and full uploads. Actual adapter, Windows
allocator/queue behavior and end-to-end improvement remain unmeasured; those are
the next falsifiers for any larger performance claim.
