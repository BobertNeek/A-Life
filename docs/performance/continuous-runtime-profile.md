# Continuous runtime profile

Set `ALIFE_FOUNDATION_PROFILE` to opt in, using the same presence semantics as
serial stage timing (even a value of `0` enables it). This observes existing
counters in the Native same-individual continuous producer. It adds no per-tick
timer, window output, pacing controller or learning/checkpoint policy.

The producer captures counters twice: after restore/legacy policy adoption and
at its final runtime boundary, including an ordinary error return. It writes
at most one `runtime-profile.json` in its already-owned `.run/live` directory.
Serialization and file contents are capped at 64 KiB. The file is created
exclusively and never overwrites a predecessor. Missing parents, exhausted
quota, serialization or I/O failure do not replace the producer result. A
partial diagnostic file can remain; it is never checkpoint authority.

The optional write runs after the core result is prepared and validation and
recovery guards are released, while aggregate quota accounting and the store
lease remain active. The original producer budget receipt is a snapshot before
this optional write; the write itself is still charged to that quota. Recovery
headroom is larger than the maximum profile. No profile record or counter field
is added to Native stdout or stderr; their existing 8 MiB log guards remain.

Before Native cleanup the supervisor reads the bounded file from the stopped
producer. It does not embed the counter payload in owner state, recovery data
or the mandatory retirement receipt. Native cleanup still inventories its one
owned file and validates that inventory normally. Only after retirement commits and housekeeping
succeeds does it emit one compact JSON diagnostic on the supervisor's stderr.
That display is independently capped at 64 KiB including its newline. It writes
directly to stderr's file descriptor, without leaving data in Python's text
buffer that could fail again during interpreter shutdown. Missing descriptors,
closed pipes and other ordinary write errors discard the optional display;
custom sinks without a file descriptor are unsupported. A partial write may
leave an incomplete record, and a stalled sink can still block. It neither
redirects process descriptors nor flushes unrelated buffered diagnostics.
Diagnostic sink failures do not change the committed result. Native logs, canonical
receipts and normal supervisor stdout retain their existing schema and budgets.
The stderr record may coexist with other ordinary diagnostics; select its
`continuous-runtime-profile-v1` kind rather than assuming every line is JSON.

This is best-effort profiling, not durable recovery. A crash during cleanup can
lose the in-memory diagnostic before display; retrying an already committed
retirement does not reconstruct it. Disabled, missing, malformed or unavailable
profiling must not be interpreted as measured zeros. No complete profile is
promised for panic, abort, OOM or failed output. A `measurement_status` of
`complete` describes the runtime measurement boundary, not successful owner
publication, retained learning, independent feeding or gameplay capability.
The reader validates v1 envelope and snapshot versions, unsigned counter and
tick types, interval relationships, and checked deltas against both snapshots.
Unknown versions or malformed deltas make the report unavailable. Supported
counter-set changes and regressed world ticks retain raw snapshots with null
interval deltas; they do not become measured zeros. This diagnostic validation
does not admit canonical state or certify counter accuracy.

Initial and final snapshots retain raw runtime-instance cumulative counters.
Checked deltas span that whole interval. Reset, wrapped or saturated counters
have unavailable deltas; valid zeros stay zero. The sleep-journal high-water
mark is cumulative only. Timing values are inclusive host wall nanoseconds
around existing calls, with nested/overlapping work, not additive GPU kernel
time. Runtime teardown, owner publication and the diagnostic write lie outside
these counter snapshots. Outer producer/host timings can include profiling
overhead. File handling uses the existing serialized, trusted local producer
paths; no new race-free lookup or crash-durability guarantee is claimed.

The local synthetic component assay used the app's Cargo test-profile
opt-level=2 override, one warmup pair and seven alternating retained pairs of
100 profiles per arm. Directory setup was outside timing. Enabled construction,
two counter captures, serialization, exclusive write and file sync measured
about 0.31 ms/profile on the disposable local filesystem. Example native reports
were about 13.8 KiB. The examples populate a wall-time counter that the timed
fixtures leave at zero; these are synthetic values, not runtime stage times.
Environment lookup, host parsing/display and actual runtime work were excluded.
No dedicated-host isolation, CPU affinity, memory or other-platform measurement
was performed. These component observations do not establish application speed.

## Why aggregate and separate the output

Opt-in per-window export still exceeded the existing log cap in the enabled
mode: the earlier compact synthetic 1,954-window fixture grew from 687,808 to
18,045,190 bytes. Periodic stdout samples also spend the ordinary log budget,
and even one large final stdout record can cross an almost-full log boundary.

Two counter snapshots and a separately capped sidecar provide a fixed-size
starting point for investigating unknown current costs. They sacrifice
per-window attribution and crash-durable profiling history. They preserve the
ordinary log and retirement limits rather than increasing them or dropping
core evidence. No current GPU, whole-game, learning or end-to-end performance
improvement is established by this diagnostic design or its synthetic tests.
