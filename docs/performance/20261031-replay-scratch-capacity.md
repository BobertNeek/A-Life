# Replay scratch capacity reuse

The trainer can reuse one current set of GPU replay buffers when a new logical
window fits their existing byte capacities. Window lengths and packed offsets
remain specific to the actual sequence; the WGSL and packet format are unchanged.
No CPU packet cache or per-window GPU buffer pool is retained.

A changed window still validates its source phenotype, inputs, dendrites and
optimizer age, builds the actual metadata/layout, checks device limits, and packs
all headers and upload words before modifying reusable resources. Metadata is
uploaded again when dendrite values or topology change. If only metadata grows,
only that buffer and its binding are replaced. Larger tick scratch follows the
original fresh-allocation path and copies the complete weights and actor Adam
state, including integer per-coordinate ages. Persistent effective-batch
accumulation remains outside this scratch set.

Full scratch clears still cover the retained backing buffers. Internal replay
readback uses actual logical extents. PPO, frozen-value and imitation admission
retain the previous logical word bounds after every objective rebind; spare
capacity does not become extra admitted rows. Public raw buffer accessors expose
backing capacity and require callers to respect the current logical view.

The fresh-allocation estimator is not a measurement of retained bytes or physical
VRAM. A capacity mix can retain a prior longer tick allocation together with newer
larger dendritic metadata. Group admission therefore adds a conservative
2,179,072-byte metadata reserve above its existing twice-fresh replay allowance
and separate auxiliary allowance. The reserve follows the legal bounds of 4,096
branches, 32 inputs per branch, five descriptor words per branch and four combined
input/outgoing words per input. A CPU regression exercises a long/small-metadata,
short/maximal-metadata, then slightly longer/small-metadata history. This is a
buffer-accounting bound; it does not establish that an earlier total device cap
was exceeded or measure physical VRAM.

The direct-device fixture compares actual reusable preparation with fresh
allocation through changing lengths, burn-in, dendrite values/topology/raw signed
zeros, structural context and sparse/absent speech. Its oracle compares logical
metadata, packets, state, outputs and gradients, complete weights/moments/integer
ages, accepted Adam updates and frozen coordinates. Refused host preparations
must leave the previous layout and bytes unchanged. Software Vulkan is permitted
only inside this synthetic fixture; physical GPU, full PPO-controller continuation
and whole-training performance remain separate qualification gates. No speedup
is claimed by this implementation note.
