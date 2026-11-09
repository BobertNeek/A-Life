# Future-only lossless M2 research journal v1

Status: opt-in source module and focused opaque-byte qualification. NOT activated,
NOT published to GitHub, NOT an acquired-life conversion or learning result.
Existing research runs, old loaders and all acquired files remain untouched.

## Integration in a NEXT run

Apply `LOSSLESS-JOURNAL-v1.patch` to the reviewed A-Life source. It adds only
`lossless-research-journal` (default off), two dependency feature references to
already-present sha2/zstd, and the new module. Explicitly enable that feature in
a separately owned future runner. Do not alter existing uncompressed runners or
reinterpret their records. Patch application was checked against
`grouped-m2-durable-pair-v1`; full Cargo workspace qualification is NOT performed.

Create a new private directory using `Journal::create`. Keep the existing
scientific row serialization, complete native snapshot serialization, source
seals, optimizer seals and sequence semantics unchanged. Pass the complete
original bytes to `publish`; do not decode/re-encode, project, quantize, drop
fields, or delta-encode them. Use one sequence per complete snapshot. Never
silently skip a failed publish; stop the run. A claim is a consumed sequence,
not evidence that a scientific interval completed.

On explicit future recovery, call `read_snapshot` and compare its raw SHA and
sequence against the authoritative future run's source/scientific seals BEFORE
passing the returned original byte buffer to the existing native loader. This
module checks opaque integrity, not source authorization or scientific validity.
No resume writer is offered: stopped/partial runs require reviewed recovery into
a new owner/run, never automatic retries or reuse of claims.

Keep the original interval cadence and source state. Measure the next authorized
run end-to-end, including encode, disk sync, decode, maximum record size and
memory, before changing storage reservations. Current synthetic sizes are NOT
representative checkpoint corpus or end-to-end throughput estimates. Compressed
incompressible bytes can expand; reserve the declared compressed maximum, not a
predicted ratio. Never lower old raw trace guards based on this module's results.

## Binary contract

New OWNER literal: `alife-m2-lossless-research-journal-v1\n`.
Files: decimal 20-digit sequence + `.m2z`; old filenames/loaders are unchanged.
Header is 104 bytes:

- 0..8: `M2ZJNL01`.
- 8..12: little-endian u32 format version 1.
- 12..16: little-endian u32 codec 1 (one standard Zstandard frame).
- 16..24: little-endian u64 sequence, checked against caller expectation.
- 24..32: little-endian u64 original raw length.
- 32..40: little-endian u64 compressed frame length.
- 40..72: SHA-256 of exact original raw bytes.
- 72..104: SHA-256 of exact compressed frame bytes.
- 104..end: exactly one standard Zstandard frame, no suffix or second frame.

Unknown magic/version/codec, old/raw input, wrong owner, wrong sequence,
truncation, wrong lengths/hashes, skippable/concatenated frames, corrupted frames
and trailing record bytes fail closed. Standard zstd frame content checksum is
enabled by the writer as additional protection. SHA checks are integrity checks,
not authentication: an attacker able to rewrite records and their hashes is out
of scope. The caller's independently preserved raw seal supplies that binding.

Scientific identity is the raw length/SHA and original bytes. Compressed bytes
are not a stable identity across codec versions. Keep codec/toolchain/build
provenance in future run receipts, but do not use compressed SHA as proof of a
scientific change. Writer currently uses zstd 0.13.3, level 1, maximum 8 MiB window,
pledged original size, and no external dictionary.

## Limits and allocation

Caller-selected maxima cannot exceed raw=64 MiB and compressed=65 MiB. Declared
sizes are validated before allocation. Reader holds at most the bounded compressed
frame, decodes in <=16 KiB chunks with an 8 MiB maximum zstd window, and requests
at most remaining original bytes + 1 to detect oversized output. Raw bytes are
returned only after full validation. No unbounded read-to-end of frame/raw data.
The original raw buffer is not allocated based on an untrusted raw declaration.

This is bounded streaming *decompression*, not constant-memory archival: the
reader retains the bounded frame and final original bytes. Vec capacity growth
can add bounded overhead. At the largest permitted limits a process may need
several hundred MiB including codec buffers, caller input, frame and record
copies; the 128 MiB synthetic qualification allowance is NOT a production memory
promise. Select lower caller maxima from measured future-run requirements.

## Publication and failure semantics

Target: trusted private local Unix filesystem supporting fsync on directories
and atomic no-overwrite hard links. Hostile concurrent directory replacement,
network filesystem durability and Windows are unqualified. Paths must have an
explicit existing parent. Root creation uses exclusive mkdir and parent fsync;
OWNER uses create_new, readonly permission and file fsync, then root fsync.
Incomplete root creation is a permanent partial claim, never reopened by writer.

For each record: encode first; exclusively mkdir `sequence.claim`; fsync root;
create_new `snapshot.pending` in claim; write complete record; set readonly;
fsync file; fsync claim; hard-link it to final `.m2z`; fsync root. Hard-link
publication cannot replace an existing final name. Permanent claims prevent
concurrent/repeated publication of a sequence even after failure. Stage and final
are two links to one inode, not duplicated payload storage. They remain as
immutable recovery evidence. The module never deletes or overwrites anything.
Readonly is API/ordinary-file protection, not an adversarial write-once store.

Before final link, a process exit leaves absent final data and a partial claim.
After final link but before successful root fsync, a process/I/O failure has
uncertain *durability*, though a complete valid record may already be visible.
After successful fsync, success is returned. Readers still validate every byte.
An I/O error is not permission to retry or reclaim a sequence. Keep failures and
partial directories; never infer completeness from a claim or pending file.
Snapshot publication is atomic; a separate scientific index/receipt is not
atomically committed with it and needs an explicit future protocol.

Tests exercise process exits, not power loss or real EIO/ENOSPC injection.
Durability depends on filesystem/hardware honoring sync and hard-link semantics.
Do not claim power-cut qualification from the process-exit tests.

## Qualification scope

Standalone native Rust tests link existing verified zstd/sha2 rlibs read-only.
No Cargo invocation, dependency rebuild, shared-target write, checkpoint read,
checkpoint replay, brain/body/world forward, activation, upload or publication.
Only fixed literal bytes, byte ramps, repeated bytes and deterministic xorshift
opaque bytes are used. Test data stays in this packet's fixtures directory.
See TESTS.log and QUALIFICATION.json for final results and bounds.

Focused rerun: choose a NEW empty fixture parent for each invocation, then run
`JOURNAL_TEST_ROOT=/absolute/new/fixture-parent build/opaque-tests --nocapture --test-threads=1`.
The test harness deliberately never cleans up claims. Sandbox process IDs may
repeat across command invocations, so reusing a fixture parent correctly fails
exclusive creation. An intermediate attempt did exactly that; its unmodified
failure output is retained in TESTS-existing-fixture-refused.log. The unchanged
final executable passed all 8 tests under fixtures-final.

Resource accounting: the first standalone compile used 162,960 KiB peak RSS,
exceeding the originally estimated/approved 128 MiB. This is a recorded deviation,
not a compliant measurement. Before the final compile the coordinator explicitly
raised the compiler allowance to 192 MiB; final compile was 170,216 KiB. Final
serial test RSS was 21,352 KiB, within its unchanged 128 MiB allowance. Native
link output is ~2.75 MiB; the entire retained packet is well below the 40 MiB
cumulative cap. No installation or dependency compilation was necessary.
