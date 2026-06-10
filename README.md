# A-Life

A-Life is a cognitive-first artificial-life engine for sparse, fixed-shape 2048-neuron brains. The engine is designed around variable GPU memory profiles, explicit brain residency states, and tri-phase `ExperiencePatch` processing.

The CPU owns world state, organism scheduling, memory topology, and persistence decisions. The GPU performs sparse neural math for hot and warm brains, while lower-salience organisms can be time-sliced, compressed on host memory, compacted during sleep, or backed by disk.

## Architecture

- Fixed-shape brains use 2048 neurons split into lobes for sensory, associative, memory, value, motor, and homeostatic processing.
- `ExperiencePatch` loops are tri-phase: ingest sensory and memory context, execute sparse neural activation, then emit action commands and consolidation hints.
- GPU memory profiles scale the same brain format from a 2 GB minimum profile to high-memory profiles for hundreds of full-fidelity organisms.
- Brain residency states define whether an organism is hot at 60 Hz, warm and time-sliced, cold and host-compressed, sleep-compacted on GPU, or dormant and disk-backed.
- Weight ownership is split between immutable genetic priors, consolidated habit memory, and short-lived operational state.

## CPU/GPU Contract

The CPU is authoritative for simulation state and memory placement. It selects residency for each organism, assembles sparse activation batches, and routes resulting `ActionCommand` values back into the world.

The GPU is an accelerator. It receives compact brain tiles, active synapse slices, sensory cache pages, and scratch buffers sized by a `GpuMemoryProfileManifest`.

## Diagram Placeholder

```text
World State (CPU)
  -> ExperiencePatch ingest
  -> Sparse neural math (GPU)
  -> ActionCommand output
  -> Habit consolidation / sleep compaction
```

## Usage

Install Rust with `scripts/setup.sh`, then build and test:

```sh
scripts/build.sh
```

Launch the starter demo configuration:

```sh
scripts/run.sh
```

From the repository root, the Makefile wraps the same commands:

```sh
make setup
make build
make run
```

The current crate is a library scaffold. Runtime systems, Bevy app wiring, GPU compute kernels, Unity adapters, Python bindings, SLM memory, and external datasets are intentionally left as follow-on modules.
