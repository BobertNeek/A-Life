# P04.5 Performance Contract

Status: v1 scaffold amendment.

Plan: P04.5 - Performance contract, GPU/CPU boundary, and population budget amendment.

This document freezes performance-relevant assumptions before P05-P09 split into concurrent branches. It is an architecture contract only. It does not implement neural kernels, GPU shaders, Bevy or Avian adapters, world simulation, playground code, SLM code, D2NWG, ETF tooling, or runtime neural loops.

## 1. GPU-to-CPU Transfer Contract

Normal active gameplay uses this transfer rule:

- Bulk neural readback is forbidden during normal active gameplay.
- Blocking per-synapse, per-lobe, arbitrary activation, and weight readback are forbidden in the standard tick.
- Compact batched action summaries are allowed at tick boundaries.
- Diagnostic/export readbacks are allowed only through explicit capture/export paths.
- One-frame-late or double-buffered staging is allowed when it avoids stalls.
- The CPU remains the source of world/action truth until GPU parity and adapter contracts explicitly say otherwise.

The GPU may accelerate sparse neural math and write compact motor summaries into staging buffers. The CPU remains responsible for validating world state, decoding public actions, rejecting impossible actions, sealing experience patches, owning save/log/export semantics, and deciding when diagnostic capture is active.

Allowed active-gameplay transfer examples:

- One action-summary record per hot brain per tick or per action-arbitration cadence.
- Aggregate counters such as active tile count, rejected action count, overflow count, or NaN rejection count through planned telemetry buffers.
- Double-buffered staging where tick `T` reads summaries produced by tick `T - 1` if latency is documented.

Forbidden active-gameplay transfer examples:

- Reading all activations each frame for CPU-side decision logic.
- Reading raw `W_genetic_fixed`, `W_lifetime_consolidated`, `H_operational`, or `H_shadow` during the standard tick.
- Reading arbitrary lobe slices or per-synapse traces as a prerequisite for action execution.
- Blocking on GPU completion just to inspect cognition internals.

## 2. Action Staging Contract

A compact internal GPU motor summary may exist later, but only as backend-private representation. A one-byte or thin GPU token is allowed only inside the backend if it is decoded into the public structured action contract before world execution.

The public core motor ABI remains structured. P09 must preserve at least:

- action ID and action kind
- optional target entity
- optional target position
- intensity
- duration ticks
- confidence
- drive/source mask
- teacher/lesson response metadata
- optional speech, writing, or vocal payload reference
- arbitration trace reference

The future internal GPU staging record should be fixed-size, versioned, class-bucket compatible, and double-bufferable. It should hold only enough data to identify the winning motor candidate and references into side buffers. It must not become the semantic action format exposed to the world.

## 3. VRAM and Memory Ledger

All byte totals in this section derive from explicit formulas in the machine-readable ledger below. The validation test in `crates/alife_tools/tests/performance_contract.rs` recomputes the totals from these fields.

Definitions:

- `dense_synapses = neurons * neurons`
- `dense_tiles = (neurons / 16) * (neurons / 16)` for the required aligned classes
- `active_synapses` is the v1 per-agent active sparse budget
- `active_tiles` is the v1 per-agent active tile budget
- `double_buffer = 2`
- `dense_worst_case_total_bytes` is the naive dense total if every listed component were dense per brain. It is for warning/comparison only.
- `sparse_per_creature_live_bytes` excludes shared species/template memory and includes per-creature live plastic/runtime buffers.
- `shared_species_template_bytes` is shared sparse `W_genetic_fixed` for one species/template in this v1 ledger.
- `sparse_population_total_bytes = shared_species_template_bytes + sparse_per_creature_live_bytes * population_count`

The ledger distinguishes these categories:

- Dense worst-case memory: a warning case for dense `N x N` allocation. It is not the active runtime target.
- Sparse active-budget memory: bounded memory sized by active synapses, active tiles, neurons, and double-buffered staging/log pages.
- Shared species/template memory: `W_genetic_fixed` lives here by default.
- Per-creature live plastic memory: `W_lifetime_consolidated`, `H_operational`, `H_shadow`, activations, accumulators, byproduct/autophagy state, tile metadata, action staging, and packed experience logging buffers.

No compressed total is claimed unless the storage rule is explicit in the ledger. In v1, the only explicit compression-like policy is that `AlphaMask` defaults to tile overrides rather than dense per-synapse storage.

Component storage policy:

| Component | Dense worst-case formula | Sparse active-budget formula | Sharing policy |
|---|---:|---:|---|
| `W_genetic_fixed` | `dense_synapses * 2` | `active_synapses * 2` | shared species/template |
| `W_lifetime_consolidated` | `dense_synapses * 2` | `active_synapses * 2` | per-creature live plastic |
| `AlphaMask` | `dense_synapses * 1` | `active_tiles * 1` | per-creature override map by default |
| `H_operational` | `dense_synapses * 2` | `active_synapses * 2` | per-creature live plastic |
| `H_shadow` | `dense_synapses * 1` | `active_synapses * 1` | per-creature live plastic |
| activations | `neurons * 4` | `neurons * 4` | per-creature live state |
| accumulators | `neurons * 4` | `neurons * 4` | per-creature live state |
| byproduct/autophagy state | `dense_synapses * 1` | `active_synapses * 1` | per-creature live maintenance |
| sparse tile metadata | `dense_tiles * 16` | `active_tiles * 16` | per-creature routing metadata |
| action staging buffers | `double_buffer * 64` | `double_buffer * 64` | per-creature staging |
| packed experience logging buffers | `double_buffer * 256` | `double_buffer * 256` | per-creature staging/export |

Required class/population ledger:

| Class | Dense worst-case total bytes | Sparse per-creature live bytes | Shared species/template bytes | Population 1 | Population 10 | Population 50 | Population 100 | Population 250 | Population 500 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Nano512 | 2,380,416 | 54,976 | 16,384 | 71,360 | 566,144 | 2,765,184 | 5,513,984 | 13,760,384 | 27,504,384 |
| Small1024 | 9,511,552 | 109,312 | 32,768 | 142,080 | 1,125,888 | 5,498,368 | 10,963,968 | 27,360,768 | 54,688,768 |
| Standard2048 | 38,027,904 | 216,896 | 65,536 | 282,432 | 2,234,496 | 10,910,336 | 21,755,136 | 54,289,536 | 108,513,536 |
| Large4096 | 152,076,928 | 433,152 | 131,072 | 564,224 | 4,462,592 | 21,788,672 | 43,446,272 | 108,419,072 | 216,707,072 |

```performance-ledger-v1
PERFORMANCE_LEDGER_V1_BEGIN
CLASS,Nano512,512,8192,64
CLASS,Small1024,1024,16384,128
CLASS,Standard2048,2048,32768,192
CLASS,Large4096,4096,65536,384
POP,1
POP,10
POP,50
POP,100
POP,250
POP,500
COMPONENT,W_genetic_fixed,2,dense_synapses,active_synapses,shared
COMPONENT,W_lifetime_consolidated,2,dense_synapses,active_synapses,per_creature
COMPONENT,AlphaMask,1,dense_synapses,active_tiles,per_creature
COMPONENT,H_operational,2,dense_synapses,active_synapses,per_creature
COMPONENT,H_shadow,1,dense_synapses,active_synapses,per_creature
COMPONENT,activations,4,neurons,neurons,per_creature
COMPONENT,accumulators,4,neurons,neurons,per_creature
COMPONENT,byproduct_autophagy_state,1,dense_synapses,active_synapses,per_creature
COMPONENT,sparse_tile_metadata,16,dense_tiles,active_tiles,per_creature
COMPONENT,action_staging_buffers,64,double_buffer,double_buffer,per_creature
COMPONENT,packed_experience_logging_buffers,256,double_buffer,double_buffer,per_creature
TOTAL,Nano512,2380416,54976,16384
POP_TOTAL,Nano512,1,71360
POP_TOTAL,Nano512,10,566144
POP_TOTAL,Nano512,50,2765184
POP_TOTAL,Nano512,100,5513984
POP_TOTAL,Nano512,250,13760384
POP_TOTAL,Nano512,500,27504384
TOTAL,Small1024,9511552,109312,32768
POP_TOTAL,Small1024,1,142080
POP_TOTAL,Small1024,10,1125888
POP_TOTAL,Small1024,50,5498368
POP_TOTAL,Small1024,100,10963968
POP_TOTAL,Small1024,250,27360768
POP_TOTAL,Small1024,500,54688768
TOTAL,Standard2048,38027904,216896,65536
POP_TOTAL,Standard2048,1,282432
POP_TOTAL,Standard2048,10,2234496
POP_TOTAL,Standard2048,50,10910336
POP_TOTAL,Standard2048,100,21755136
POP_TOTAL,Standard2048,250,54289536
POP_TOTAL,Standard2048,500,108513536
TOTAL,Large4096,152076928,433152,131072
POP_TOTAL,Large4096,1,564224
POP_TOTAL,Large4096,10,4462592
POP_TOTAL,Large4096,50,21788672
POP_TOTAL,Large4096,100,43446272
POP_TOTAL,Large4096,250,108419072
POP_TOTAL,Large4096,500,216707072
PERFORMANCE_LEDGER_V1_END
```

## 4. Alpha Storage Policy

`AlphaMask` must not default to dense per-synapse storage unless later benchmarks prove that dense storage is necessary and affordable.

Preferred hierarchy:

1. Projection default alpha.
2. Lobe/tile override alpha.
3. Sparse per-synapse alpha override only when exceptional.

P06 owns the detailed weight split and alpha semantics, but it must start from this hierarchy. Dense per-synapse alpha may appear as a debug/reference comparison or a benchmarked opt-in mode, not as the default live runtime layout.

## 5. GPU Buffer Sharding Policy

Future GPU plans must use this vocabulary:

- `PopulationShardId`: identifies a class/profile/residency bucket of creatures.
- `ProjectionShardId`: identifies a grouped projection family inside a population shard.
- `BufferPageId`: identifies a bounded allocation page inside a GPU buffer pool.
- `TileRange`: identifies a contiguous range of sparse microtiles or supertile-local tile spans.
- Page-relative offsets: all shader-facing offsets are relative to a page or shard base, not raw host pointers.

No GPU plan may assume a single monolithic multi-GB storage buffer. P24-P29 must plan page sizes, shard assignment, bind group layout, staging buffers, and page-relative offsets before adding any shader contract.

## 6. Active Synapse and Update Budget Policy

The v1 default active budgets are conservative and intentionally lower than dense capacity:

| Brain class | Neurons | Active synapses per agent | Active tiles per agent | Essential lobe priority |
|---|---:|---:|---:|---|
| Nano512 | 512 | 8,192 | 64 | metabolic, sensory, motor |
| Small1024 | 1,024 | 16,384 | 128 | metabolic, sensory, motor, compact association |
| Standard2048 | 2,048 | 32,768 | 192 | metabolic, sensory, motor, association, episodic |
| Large4096 | 4,096 | 65,536 | 384 | metabolic, sensory, motor, association, episodic, working memory |

Per-lobe/projection budget concept:

- Every lobe and projection receives a soft budget derived from class, salience, residency, and essentialness.
- Essential projections cover survival, sensory grounding, motor arbitration, homeostasis, and immediate safety.
- Non-essential projections cover low-salience memory expansion, optional concept/topology updates, curiosity exploration, and slow semantic modulation.

When a budget is exceeded:

- Essential lobes/projections keep their minimum reservation.
- Non-essential projections are decimated first.
- Warm/cold or low-salience agents reduce update cadence before reducing survival-critical coverage.
- Structural growth requests are queued for sleep/consolidation instead of resizing active buffers.
- Overflow must increment a counter and may produce a bounded frustration/recovery signal later; it must not corrupt learning state.

Update throttling/decimation policy:

- Online plasticity can be decimated by projection, lobe, or residency state.
- Memory expectancy and topology updates can run slower than sensory/motor paths.
- Sleep/consolidation can drain queued edits outside the active tick.
- No plan may assume that every synapse or lobe updates every 60 Hz frame.

## 7. Cadence Policy

Not all cognition runs at 60 Hz for all agents.

Default v1 cadence bands:

| Subsystem | Hot agents | Warm agents | Cold/dormant agents |
|---|---:|---:|---:|
| sensory/motor path | 60 Hz target | 10-30 Hz time-sliced | event/summary only |
| endocrine/homeostatic update | 10-30 Hz | 2-10 Hz | coarse decay/checkpoint |
| action arbitration | 60 Hz target for hot actors | 10-30 Hz | only on wake/event |
| online plasticity | 15-60 Hz by lobe/projection | 1-10 Hz decimated | paused or sleep-only |
| memory expectancy | 5-15 Hz | 1-5 Hz | checkpoint/query on wake |
| topology/concept graph update | 1-5 Hz | below 1 Hz or batched | sleep/export only |
| sleep/consolidation | off active tick | scheduled windows | scheduled/offline |
| logging/export | packed every tick if enabled; export async | decimated packed summaries | checkpoint/export path |

P07, P08, P09, P12, P13, P15, P20, and P24-P29 must treat cadence as part of contract design. A 60 Hz render or motor target does not imply all cognitive subsystems run at 60 Hz.

## 8. Benchmark Counters and Gates

P14/P15/P20/P24-P29 must expose or reserve these counters:

- active synapses per frame
- active tiles per frame
- supertiles skipped
- dispatches per frame
- CPU patch allocations per frame
- packed log bytes/sec
- GPU pass timing
- CPU sensory time
- CPU memory/topology time
- action staging latency
- invalid/rejected action count
- NaN/out-of-range rejection count

Gate expectations:

- P14/P15 must make counters meaningful for CPU reference execution.
- P20 must define benchmark tiers and acceptance thresholds.
- P24-P29 must compare GPU counters against CPU reference/parity fixtures before any performance claim.
- Packed logging counters must distinguish active runtime staging from diagnostic/export capture.

## 9. Branch Guidance for P05-P09

P05 must read this document before defining compute budgets, lobe throttling, and routing metadata.

P06 must read this document before defining `AlphaMask` and weight split storage semantics.

P07 must read this document before defining learning-rate/endocrine cadence assumptions.

P08 must read this document before defining sensory/context packing assumptions.

P09 must read this document before defining `ActionCommand`, `ActionDecision`, and action staging assumptions.
