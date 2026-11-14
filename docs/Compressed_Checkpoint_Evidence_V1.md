# Opt-in compressed checkpoint evidence

The CPU example-support `LocalEvidence` has explicit
`save_compressed_checkpoint(name, bytes, limit)` and
`retain_error_checkpoint_compressed(limit)` APIs. Nothing enables them by default.
Existing `begin`, `save`, `remember_valid`, JSON error retention and all native
checkpoint/runtime/biology/learning schemas remain unchanged. Existing files are
never converted, removed or overwritten. No experiment is changed by this API.

The `.cp.zst` file is an evidence envelope, not plain zstd or a model checkpoint.
Its fixed V1 header contains magic, little-endian original and payload lengths,
SHA-256 of original bytes, SHA-256 of compressed payload; a single level-3 standard
zstd frame and fixed footer follow. The reader rejects extra frames, skippable
frames, trailing bytes, truncation, mismatched hashes/lengths and unknown magic.
It returns exact original bytes without JSON parse/reserialization. The caller
must separately validate the native checkpoint before using or advancing it.
Hashes establish corruption detection, not authenticity against a writer who
can replace both data and hashes.

A caller-supplied uncompressed limit is capped at 256 MiB. Compressed input is
capped at the smaller of 257 MiB and zstd compress-bound for the admitted original
length, checked before payload allocation; decoded output allocation
is bounded by the admitted original length. Decoder window log is capped at28.
The writer holds the caller bytes, compressed envelope and verification output;
this is bounded whole-file compression, not constant-memory streaming. Callers
should choose smaller limits when appropriate. Compression is not encryption;
all payloads, hashes and learned state remain local/private.

Success ordering: create-only `<name>.pending`, write and sync, reread and verify
both hashes/exact bytes, then create-only hard-link to final `<name>`. The pending
link is retained on success (same inode, no second payload storage) and on error.
No final name is linked before verification. No run-complete marker is written;
the caller may mark its run complete only after all required writes return success.
A stale partial pending file prevents retry; use a new run, retaining evidence.
Existing final names are never replaced. Filesystems without hard-link support
fail closed. The directory is synced on Unix. If that final sync fails after the
link, the committed file may already be visible but durability is uncertain:
inspect and report the error, never blindly retry or overwrite. This does not
claim transactional power-loss guarantees on every filesystem, nor security
against concurrent hostile mutation of the local evidence directory. Counting
both hard links' logical lengths will overstate actual disk allocation.

Dependencies are the already-used CPU `zstd` and `sha2`; no software install,
new crate dependency, private genetic fixture or World run is needed. Unit tests
are self-contained. This support module is compiled by the existing CPU examples;
focused tests can be selected with `compressed_checkpoint::tests` in a CPU example
that includes `support/terminal_evidence2135.rs`. Legacy tests also remain intact.

The reader accepts an explicitly named pending file for diagnosis/recovery after
full integrity verification. That does not turn the pending name into a committed
write or a completed run. Successful final and pending names refer to the same
inode; readers must not modify either. This format does not authorize deleting
old raw checkpoints after compression. It is for prospective evidence only.
