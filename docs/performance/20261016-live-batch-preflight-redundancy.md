# Live batch preflight: remove duplicate local verification

## Scope

The ordinary GPU batch preflight no longer reconstructs a freshly encoded,
checked-rebased perception upload merely to compare it with itself in release
builds. The explicit `GpuPerceptionUpload::validate_against` diagnostic remains
available and the local self-comparison remains enabled with debug assertions.

Caller-owned memory uploads still receive full validation at the original early
preflight boundary. An owned, immutable `ValidatedLocalMemoryUpload` proof then
passes through payload sizing into consuming rebasing. Its private field and
absence of mutable access prevent payload edits or proof reuse. Rebasing keeps
its exact binding/offset checks without rescanning the unchanged local payload.

Keeping the early boundary preserves malformed-input error priority even when
the same row would also exceed payload capacity. This matters because consumers
can classify backend-capacity errors differently from invalid frame errors.

## Work eliminated

Per dispatched row, release builds skip one full frame validation/digest pass,
one repeated perception encoding/rebase and three temporary vectors. Memory rows
also skip one duplicate context reconstruction/comparison and its temporary
vector. At 32 candidates with a 54-lane decoder stride, those four eliminated
vectors contain 12,404 bytes in total, excluding allocator overhead. This is a
source-derived allocation/work count, not a measured frame-rate improvement.

The GPU work, route schedule, selection, outcome, learning, world mutation,
checkpoint formats and persistence validation are unchanged. This preserves
AOA-AUTH-002/004/005 and AOA-TIME-001/003: causal bindings and transaction checks
remain authoritative; no CPU neural execution or independently advancing state
is introduced.

## Verification

The CPU-only `closed_loop_pipeline::batch_projection_tests` exercise the actual
production preflight. The original six full-batch golden identities were captured
with the full checks on `69f00c05ec6774c3655ae87c5d4baa5df3861929`.
Adaptive exploration intentionally adds logical identity words to each row, so
its combined layout requires distinct golden identities. For integration on
`73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`, those identities were independently
recaptured after restoring that original full memory-validation/rebasing path
while retaining the exact new exploration layout. Reference source SHA256:

- `closed_loop_pipeline.rs`: `f12a6bc2c10630e0c68e325a6e1413116cf0b1c46e2b48b4f2ca4d6bb28a2c33`
- `closed_loop_memory.rs`: `25b0db72d7477f763e47146263a287ef85fbe5df36b2ccf90b13aa19e1d333d5`

The optimized combined path matches all six new identities. They cover N512,
N1024 and N2048, two rows, both activation sides and optional memory payloads.
The complete batch data, including GPU words and host bindings, must match.
An additional regression independently checks exact logical identity words and
memory, cognitive and receptor record adjacency for every memory row, so the
full-check comparison alone cannot bless an identical layout error in both paths.

Additional checks reject nine malformed caller-owned memory cases without
mutating the input, preserve invalid-memory-before-capacity error priority, and
retain activation-side, rebase-overflow and repeated-rebase rejection.

Run the focused tests both normally and with the production diagnostic branch
disabled:

```sh
cargo test -p alife_gpu_backend --lib batch_projection
cargo test -p alife_gpu_backend --lib batch_projection --config 'profile.test.package.alife_gpu_backend.debug-assertions=false' -- --nocapture
```

These are host-preflight/ABI checks. They do not establish physical GPU speed,
whole-game FPS, persistence acceptance or full-workspace qualification.
