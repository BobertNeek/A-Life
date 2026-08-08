# Vision

## The destination

A-Life aims to become a persistent artificial-life game in which organisms live, learn, reproduce, communicate, and die inside one authoritative voxel world.

Each organism should be more than an animated record. It should have:

- a GPU-resident neural policy with inherited structure and lifetime learning;
- grounded perception of its body, nearby objects, other organisms, speech, and consequences;
- persistent identity, lineage, episodic history, and a durable archive;
- needs, endocrine state, sleep, injury, ageing, reproduction, and death;
- a bounded compositional language learned through ordinary experience;
- a visible relationship with the player, peers, and optional teachers.

The long-term experience is an observable digital ecology. The player should be able to watch a creature make a neural decision, see that decision change the world, teach through ordinary interaction, follow descendants, restore a save, and trust that the visible history came from the same simulation that generated the evidence.

## Design principles

### One authoritative world

The world describes what can be perceived and which unscored actions are legal. It validates the GPU-selected action, applies the outcome, and seals the experience. Rendering, UI, teachers, semantic providers, and tools only project or translate that truth.

### Organisms learn on the GPU

Production neural encoding, recurrent dynamics, selection, plasticity, memory, topology, sleep, and consolidation belong to WGSL pipelines. CPU neural helpers remain reference, test, or developer tools. They are not a hidden production policy.

### Inheritance is biological, not magical

Offspring inherit a foundation identity, compatible structure, endocrine traits, and genetic deltas. Personal memories, lifetime weights, learned vocabulary, current conversations, and world-local bindings do not silently cross the germline.

### Language is grounded

Words and symbols become meaningful through perception, demonstration, action, social feedback, and sealed consequences. A translator may render a creature's raw tokens for a human; it may not invent the creature's thought.

### Assistance never becomes authority

An external teacher acts through the same visible and audible world as everyone else. A private local SLM may provide weak developmental priors or surface translation, but it cannot act, choose a target, inject reward, change weights, or bypass perception.

### Evidence grows with the claim

A contract can be implemented without being integrated. An integrated feature can remain invisible. A visible feature can remain unproven. Promotion and release claims require causal receipts from the exact source, adapter, backend, and rendered path in question.

## Near-term product

The next credible product is not general intelligence. It is a small, honest artificial-life research game with:

- a live GPU-to-voxel loop;
- autonomous organism birth, ageing, death, and archive transactions;
- truthful pause, load, save, selection, and speech controls;
- a few organisms whose grounded learning can be observed and replayed;
- bounded performance and evidence on supported Windows hardware.

## Non-goals

- No claim of AGI, consciousness, sentience, or human-equivalent cognition.
- No Unity, C#, or HLSL production stack.
- No CPU neural fallback presented as GPU success.
- No fixed global `Standard2048` brain shape.
- No N4096 production promotion without separate causal, restore, equivalence, and budget evidence.
- No hidden teacher or SLM channel that writes organism cognition.
- No release claim based only on screenshots, UI surfaces, benchmark targets, or preflight receipts.
