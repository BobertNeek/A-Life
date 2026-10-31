# Bounded replay scratch capacity reuse

A release-profile software-Vulkan fixture measured **13.92% lower paired median
replay-preparation elapsed time**. Including frozen forward/BPTT/Adam work, the
paired median reduction was **2.315%**, with one pair slower. Reuse retained
**46,901,760 extra backing bytes (44.73 MiB)** after a 384→256-tick shrink. These
are component results, not a whole-training or physical-GPU speedup claim.

The trainer reuses one current set of fitting GPU buffers. Actual per-sequence
layouts, packed offsets and WGSL stay unchanged. Changed dendrites get fresh
metadata; metadata-only growth replaces that buffer and its binding. Larger tick
scratch uses the original allocation/copy path, preserving weights and complete
actor Adam state. No CPU packet cache or per-window GPU buffer pool is added.

PPO, frozen-value and imitation admission use the current logical output/state
word bounds after every rebind, even when backing capacity is larger. Full
scratch clears remain. Raw public buffer access exposes backing capacity;
`replay_buffer_word_lengths()` supplies checked logical extents and rejects stale
replay. Previously encoded work must be submitted before preparing another window.

## Measured scope

The clean measured source is `cf75133975dc11a8523ef144158572a5f5484d6f`, based on
main `a9288027`. [Raw evidence](20261031-replay-scratch-capacity-trials.json)
contains all warmup/measured rows, source/tree/binary hashes, actual Cargo profile
fields, logs, per-buffer capacities, commands and the separate precursor cohort.
Later source changes only remove a redundant conversion in the diagnostic probe
and annotate the intentional release-only timing assertion for Clippy.

Both modes start already allocated for 384/128. Timed windows are
256/0 → 384/128 → 256/0 ticks/burn-in; initial construction and growth to 384 are
excluded. The fresh-allocation control models the original changed-shape path;
every transition changes shape. It uses the same current resource constructors
and packed kernels, not a separately built old application binary.

The synthetic graph has N2048, 32,768 synapses, 17 routes, one dendritic branch
with two inputs, and every third synapse trainable. All four microsteps are active
on every tick. Candidates alternate two/one; route and structural context also
vary. Speech supervision is disabled. Each scope receives two warmup pairs and
seven measured pairs in alternating order, with both warmups retained in the raw
receipt. The three updates are a frozen component fixture, not a PPO effective
batch or a count of unique retained training rows.

| Timed sum across three windows | Fresh median | Reuse median | Median paired elapsed reduction | Ratio-of-medians elapsed reduction |
| --- | ---: | ---: | ---: | ---: |
| Prepare, submit and wait | 150.339 ms | 128.403 ms | 13.924% | 14.591% |
| Prepare, frozen forward/BPTT/Adam, submit and wait | 2553.643 ms | 2464.240 ms | 2.315% | 3.501% |

Paired reductions range from 12.684–17.539% for preparation and −1.340–4.912%
for the broader component. No outlier is removed or confidence interval claimed.
The broader result is small and noisy. An earlier mixed 0–4-microstep cohort is
preserved separately; its warmup timings were not saved and its original
`candidate_count=2` field meant a maximum, not a constant count.

Each primary span includes real validation, metadata/layout/packet/header work,
queue uploads, resource creation/copies when required, encoding where applicable,
submission and wait. Receipt construction, capacity sampling, precomputed
allocation estimates, correctness probes and final fixture destruction are outside
those spans. Retirement/drop of resources replaced inside `fresh_prepare` is
inside the preparation span. Inclusive sequence wall is recorded separately. World collection, replay IO,
PPO/value objectives, checkpoint admission/persistence and full host lifecycle are
excluded. This does not establish retained learning or capability per hour.

Hardware is Linux x86_64 on an AMD EPYC 9V74 cloud VM, five visible CPUs with a
four-CPU quota, unpinned execution and two llvmpipe threads. Adapter identity is
truthfully Cpu/Vulkan, Mesa 25.0.7 with LLVM 19.1.7. Rust is 1.99.1/LLVM 23.1.1;
the actual release test artifact records opt-level 3, debug assertions and overflow
checks off. There was no concurrent compilation. A sibling reported one roughly
0.03-second JSON parse during the primary run. VM/allocator/cache contention and
observer effect are otherwise uncontrolled; physical RTX 3050 behavior is unknown.

## Requested resource bytes

These are per-trainer `Buffer.size()` values, not physical allocation or VRAM.
The fixture holds separate reference and candidate instances only for comparison.

| Resource | Fresh 256-tick set | Retained 384-tick backing |
| --- | ---: | ---: |
| Training packet | 42,710,016 | 64,052,736 |
| Neural state | 25,190,400 | 37,773,312 |
| Gradients | 25,673,728 | 38,371,328 |
| Outputs | 294,928 | 442,384 |
| Headers | 262,144 | 393,216 |
| Metadata | 1,409,292 | 1,409,292 |
| Weights | 131,072 | 131,072 |
| Optimizer | 393,216 | 393,216 |
| Mask | 131,072 | 131,072 |
| Step readback | 16 | 16 |
| **Total** | **96,195,884** | **143,097,644** |

The fresh path's old-plus-new requested-resource envelope for each resize in this
sequence is 239,293,528 bytes. Steady reuse needs no tick growth in this sequence;
later growth still takes the old-plus-new path. Backend allocations and the
unchanged 131,072-byte persistent actor accumulation are outside this table.
Process RSS was not sampled; physical VRAM and actual allocator peaks are unknown.

A retained tick capacity can coexist with independently grown dendritic metadata.
Group admission therefore adds a conservative **2,179,072-byte** metadata reserve
above its existing twice-fresh allowance and separate auxiliary allowance. The
reserve follows 4,096 branches, 32 inputs per branch, five descriptor words per
branch and four combined input/outgoing words per input. A CPU regression checks
64/small metadata → 1/maximal metadata → 65/small metadata. This strengthens the
buffer envelope without borrowing unused auxiliary phases; it does not establish
that an earlier complete deployed cap was exceeded.

## Correctness and remaining qualification

The accepted device-helper oracle covers ten changing-length/burn-in updates,
dendrite topology/values/raw signed zeros, structural context and sparse/absent
speech. Every update was accepted, changed 9,593–9,842 trainable weights, advanced
the expected integer ages above 2^24, and preserved frozen weights/moments/ages.
Logical metadata/upload/state/output/gradients and complete actor state match the
fresh reference bit for bit. Seven cases reused scratch and two grew metadata
alone. Three invalid preparations preserved previous bytes and layout.

The device-helper test supplies step headers itself; it does not prove the public
trainer's counter or effective-batch owner. A [separate actual public-PPO
comparison](https://github.com/BobertNeek/A-Life/blob/0235b5b1029537ebe077b685148ea42e476ac044/docs/performance/20261031-software-ppo-controller.md)
matched 18 sections and 1,470,668 raw words, including effective-batch/tail,
positive/zero value, restore and cancellation behavior. Its short synthetic
windows exercise controller continuation rather than representative performance.
All 47 default training library tests, the mixed-capacity accounting regression,
strict training Clippy with GPU test targets and the app's foundation-training
all-target check pass. Physical GPU
benefit versus retained capacity and useful whole-training throughput remain open;
no default promotion or performance promise follows from this report.
