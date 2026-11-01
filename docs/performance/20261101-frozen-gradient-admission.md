# Disabled frozen-gradient requested-byte admission

All explicit frozen-gradient cache allocations now pass a source-owned
requested-byte policy check. Normal APIs remain fresh. No app activation flag,
hardware qualification, performance rerun or whole-training gain is claimed.
PR225's `b1fd11290c99ec0ca6aa457a466cca046831885e` head is unchanged.

The executed source is `1f227eafc8917b821f03b8480abba8ea2d58c3f6`, on the separate
`codex/frozen-gradient-requested-admission` branch. A checked count-only plan
shares replay layout and all ten buffer descriptor sizes with actual allocation.
It includes initial 32-tick allocation, legal dendritic maxima, mapped-init
payloads, training/two header uploads and zero speech targets. It creates no
dummy replay windows or branch/input arrays.

After the first existing frozen metric wait, the guard reads the actual shared
Vulkan device's allocator report. The baseline includes managed live/offline
buffers and both host-visible/device-local allocator blocks. It adds two maximum
replay arenas, mapped-init and queued-write payloads, PPO header/rows/scale,
metric/status readback and K, using checked arithmetic and the existing profile
ceiling. The legal bounds are `D≤min(4096,4N)` and `I≤32D`.

This supports only pinned wgpu 29's ordinary Vulkan allocation path. Missing,
invalid, overflowing or insufficient proof refuses the optimization. DX12 and
other backends fall back. No raw/external imports or other same-device
allocations are permitted during the cache lifetime, including allocations from
loader callbacks or custom window Borrow/Drop implementations. The actual
continuous sealed loader and owned/Arc fixture handles satisfy that contract.
Future continuous geometry can come from existing `max(end - burn)` spans;
the application still calls its ordinary API.

Every existing callback remains in order. Geometry overflow or speech/auxiliary
work disables K before ordinary evaluation. A conditional retirement barrier
drains queued cache work before destruction and larger/fresh allocation; normal
admitted consumption clears pending work through its existing status wait and
adds no retirement wait. Early errors/unwinds also use this cleanup. A caught
poll error or fatal panic marks the backend's actual shared device-loss latch,
preserving the original Result/unwind rather than continuing on an unproved
retirement. The handle is gated by `gpu-tests`, cloneable and monotonic, with no
reset/raw flag. Normal factory, required features/limits and admission bodies
are unchanged.

## One software oracle

One ignored actual-controller test passed on Cpu/Vulkan llvmpipe (LLVM 19.1.7,
Mesa 25.0.7), using synthetic N2048/S32768/m4 replay, one warm update, windows
256/0 and 384/128, one epoch, effective batch eight and value coefficient 0.5.

| Path | Initial reserved baseline | Additional requested upper | Declared ceiling | Cache result |
| --- | ---: | ---: | ---: | --- |
| Bound 384 | 114,545,664 | 360,615,260 | 2,147,483,648 | 278,544 bytes; two cached/two reused |
| Bound 256 | 114,545,664 | 244,842,332 | 2,147,483,648 | Initially admitted; later geometry drop; one cached/zero reused |
| One-byte test policy | 130,790,400 | 360,615,260 | 1 | Refused; zero cache allocation |

The one-byte profile is a real valid declaration used to refuse optional K.
It does not impose a global one-byte GPU limit: existing fresh raw-training
allocation behavior remains unchanged and its control accepts the same update.

The geometry path recorded exactly one cleanup-wait trace; admitted/refused
paths recorded none, and no cleanup failure occurred. All five controller paths
kept loader order `[0,1,0,1]`. Admitted, geometry-fallback and tiny-profile-refused
paths matched their fresh controls' receipts and complete learned actor/value
checkpoints in-run, with accepted actor/value steps 2 and policy 6. Selected
weight/age progress establishes accepted updates; no stronger per-plane moment
change or raw scratch-array equivalence is claimed. Retained JSON contains
selected words and assertion results, not every checkpoint word.

Three CPU sizing/policy checks passed at `9cd0fba4fdc8de544a4534ecebb0e9b31b805db5`.
Their captured identity and logs are retained; that earlier executable was
overwritten by the final CPU build and was not separately preserved.
The new caught-cleanup-error/panic test passed at executed source `1f227eaf`,
preserving original Results and unwinds. All 45 ordinary non-`gpu-tests` library
CPU tests passed for the shared descriptor/layout implementation, which remained
unchanged afterward. Strict `gpu-tests` all-target Clippy and formatting/diff
checks passed. The single GPU oracle took 21.44 seconds in the unoptimized test
profile; that duration is a test log, not a throughput measurement.

The unique Cargo target excluded all workspace artifacts when seeded. Current
workspace crates were compiled, then the exact executable was selected from
Cargo's compiler-artifact JSON and hashed before execution:
`4e517c84938dd01ca651bbe23de9a5817d93ecb68a0e4c85dcfd9cd157d64995`.
[The receipt](20261101-frozen-gradient-admission.json) retains source/tree/file,
compiler/artifact/library identities, actual adapter, exact invocation, initial
admission observations, CPU/Clippy logs and the oracle summary/log. Raw files
remain at `/workspace/software-ppo-frozen-admission/`. Independent review
verified these bindings and observations without rerunning the oracle.

The existing software-only CI list gains this exact fixture (12 explicit cases
total); its PR-base filter also accepts the dependent disabled experiment branch.
Main-only push and read-only permissions are unchanged; release timing remains
excluded. After the run, the assay alone gained a best-effort admission diagnostic
after its elapsed timer and before assertions, so a future hardware refusal
prints its initial decision. Production library and admission-oracle Rust bytes
remain exactly executed `1f227eaf`. No GPU/performance rerun followed these
logging/documentation/CI edits. Reserved baseline plus future requested payload is
**not** a bound on future reserved-block growth, physical VRAM, free VRAM or a
reservation. Empty allocator-block retention, driver/external memory, physical
correctness, useful whole-training throughput and activation remain open gates.

## Prepared hardware handoff for the guarded source

This addendum targets published
`7665339c770a06bc1be243075dc522b0deddcaf9`, including the post-timer admission
diagnostic. The [earlier hardware guide](20261031-frozen-gradient-hardware-readiness.md)
pins `53c56c62`, which predates this guard; its qualification does not cover
requested-byte admission. These instructions are prepared only. The sole PC
owner must first reconcile cc7 admission and obtain the GPU window, and the
stacked Windows compile-only CI gate must pass. No Windows or physical GPU
command has run for this addendum.

Reuse the earlier guide's GUID-isolated checkout/output/target directories,
Cargo `--release --locked --features gpu-tests --test software_ppo_controller
--no-run --message-format=json` build, and source/tree/compiler/artifact/library
hash receipt. Replace its source pin and fetch line with:

```powershell
$source = '7665339c770a06bc1be243075dc522b0deddcaf9'
git -C $repo fetch origin codex/frozen-gradient-requested-admission
if ($LASTEXITCODE -ne 0) { throw 'Guarded source fetch failed' }
```

Stop that guide before its correctness/assay invocations; use the gates below.
Keep `ALIFE_DISPATCH_REQUIRE_HARDWARE=1`, the unchanged `production_v1` policy,
and a fresh synthetic capture directory and `ALIFE_FROZEN_ASSAY_STOP_FILE`.
Unset/`0` selects software and cannot qualify hardware. Malformed selectors
fail before device construction. Both commands must use the **same** release
`$boundExe` selected and hashed from Cargo JSON, with no intervening rebuild.
No private Current or personal trained checkpoint is opened; checkpoints are
generated procedural fixture state.

The hardware-capable correctness gate is:

```powershell
if ($env:ALIFE_DISPATCH_REQUIRE_HARDWARE -ne '1') { throw 'Hardware selector must be exactly 1' }
if ((git rev-parse HEAD).Trim() -ne $source) { throw 'Wrong guarded source' }
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
$correctnessLog = Join-Path $runRoot 'guarded-correctness.log'
& $boundExe frozen_gradients_match_actual_controller_epochs_and_shared_restore --exact --ignored --test-threads=1 --nocapture 2>&1 | Tee-Object -FilePath $correctnessLog
if ($LASTEXITCODE -ne 0) { throw 'Guarded hardware correctness failed; stop' }
$metadata = @(Get-Content -LiteralPath $correctnessLog | ForEach-Object {
    if ($_ -match 'actual_controller_execution_metadata=(\{.*\})') { $Matches[1] | ConvertFrom-Json }
})
if ($metadata.Count -eq 0) { throw 'Missing actual adapter identity' }
foreach ($entry in $metadata) {
    if ($entry.execution_mode -ne 'required_hardware' -or
        $entry.adapter_info.device_type -eq 'Cpu' -or
        $entry.adapter_info.backend -eq 'Noop' -or
        $entry.adapter_info.backend -ne 'Vulkan') { throw 'Guard qualification requires actual non-CPU Vulkan hardware' }
}
$proofs = @(Get-Content -LiteralPath $correctnessLog | Select-String 'frozen-gradient full PPO oracle: epochs=')
if ($proofs.Count -ne 4) { throw 'Expected all four active-cache correctness contexts' }
```

At this exact source, successful owned/shared active-cache assertions require
initial `Some(admitted=true)` under the guard, in addition to identical captured
semantics and accepted original/cache updates. That admission is source-derived;
the correctness test does not print its decision numbers. Its windows contain
only 3/5 ticks. It **does not prove** admission of the assay's larger
256/384-tick geometry.

The assay's existing warmup pair is the admission gate for accepting measured
results at representative 256/384-tick geometry. Its cached run prints
`frozen_gradient_admission_snapshot=Some(...)` after its controller timer and
checks active-cache assertions before the warmup pair completes. A successful
cached run requires initial `Some(admitted=true)` under the unchanged policy.
This is timed validation excluded from the five measured pairs, not a pre-timer
allocation probe. Refusal aborts before measured pairs begin; every later cached
run checks again before its pair can be accepted. Do not substitute the software-only
`admission_qualification::requested_admission_accepts_refuses_and_drops_before_larger_replay`
test, which deliberately rejects selector `1`, or bypass/change the guard.

After the hardware-capable correctness gate passes, the exact existing release
assay invocation is prepared for the sole owner when the Windows/cc7 gates above
are satisfied:

```powershell
if ($env:ALIFE_DISPATCH_REQUIRE_HARDWARE -ne '1') { throw 'Hardware selector must be exactly 1' }
if ((git rev-parse HEAD).Trim() -ne $source) { throw 'Wrong guarded source' }
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
& $boundExe frozen_gradient_assay::representative_actual_ppo_frozen_gradient_release_assay --exact --ignored --test-threads=1 --nocapture 2>&1 | Tee-Object -FilePath (Join-Path $runRoot 'guarded-release-assay.log')
if ($LASTEXITCODE -ne 0) { throw 'Guarded assay failed or refused admission; stop without bypass' }
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
```

Require the warmup and every retained measured cached-run decision to be present
and admitted, including its actual reserved baseline, requested upper and declared
ceiling. Keep the unchanged policy and all source/binary/adapter
receipts. Creating the printed stop path requests a stop between matched pairs;
the current original/cache pair and proof finish first. This remains disposable
synthetic terminal replay: continuing-world context, physical correctness,
allocator-block growth and useful whole-training throughput remain unqualified.
No default activation or merge follows from the prepared handoff.
