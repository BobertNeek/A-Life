# Readable transport and safe failure summaries, V2

This is a named source-only revision of the unactivated V1 qualification packet.
V1's source files, compressed evidence, seal and archive remain untouched.
It is not a retry of an old pending publication, activation or GitHub write.

Narrow changes:

1. Replace binary gzip in the publication map with three UTF-8, newline-boundary
   patch parts of 563,200, 563,195 and 415,718 bytes. Per-part hashes and the
   existing concatenated 1,542,113-byte patch hash are verified before application.
2. On nonzero Cargo compilation, project at most 12 deduplicated Edddd codes,
   each with at most three primary repository-relative source locations in the
   exact bound source corpus. No rendered/message/snippet/registry/private-path
   data is emitted. Invalid protocol discards all projected diagnostics.
3. Keep completed pinned test names/counts in ordinary failure summaries.
   Runtime failure otherwise identifies the selected pinned test and static code.
4. Add 31 pure synthetic tests, including mocked compile-error plumbing and
   negative transport, output, source/seal and activation cases. No native fixture
   or subprocess is invoked by those tests. Source reconstruction separately uses
   read-only Git-object inspection and in-memory patch application.
5. Name the revised draft branch, workflow, artifact path and plan ID V2. The
   two-stage activation-marker contract and contents:read permissions are
   unchanged. Only boredom V2 and world-reflex are eligible; meal/GDS stay disabled.

Rust variant patches, source bindings, source recipe bytes, fixture names and
per-test call bounds are byte-identical to V1. No performance or behavioral
qualification is added. Stdout/stderr and resource stops are monitored
thresholds which may overshoot between polls, not hard filesystem quotas.
The final sanitized JSON has a hard 32 KiB serialization bound.

No PR, actual ACTIVATE.json, dispatch, publication, native build, native fixture,
target/dependency allocation or deletion occurred. PUBLISH.json contains only
UTF-8 files; the optional compressed handoff archive is not a publication input.
