# Frozen-gradient experimental hardware fixture

RAW-GRADIENT EXPERIMENTAL remains disabled in normal controller entry points.
This follow-up changes only test adapter selection, identity logging and future
assay metadata. It preserves the original controller, arithmetic, configuration,
shader, cache lifetime and fresh evaluation after accepted updates. Historical
software measurements at `470dde2e` / `d1cc1707` are unchanged; no new performance
assay has run for this follow-up.

`ALIFE_DISPATCH_REQUIRE_HARDWARE` has strict fixture semantics:

| Value | Construction |
| --- | --- |
| Unset or `0` | Explicit software Vulkan, asserted actual CPU/Vulkan identity |
| `1` | Normal `new_required(production_v1)` plus Training session; asserted non-CPU, non-Noop identity |
| Anything else, including invalid Unicode | Fails before device enumeration or construction |

Hardware mode propagates normal admission failure. It never falls back to
software or relaxes required features/limits. The normal factory/acquisition
source is unchanged. The new adapter-info accessor exists only with `gpu-tests`
and returns actual adapter information. New assay output records execution mode,
device type, backend, name, vendor/device and driver fields; the inherited
`ALIFE_SOFTWARE_PPO_CAPTURE_DIR` name is retained for compatibility in either mode.

Cloud readiness checks passed at `53c56c62`: two pure parser tests, the one focused
active-cache software oracle (epochs 1/2 and value coefficients 0/.5), and strict
Clippy for this test target and its dependencies. Required-hardware mode on the
CPU-only cloud exited 101 with `NeuralBackendUnavailable` before session/trainer construction or PPO
execution; host procedural setup can precede admission. This expected refusal is not physical GPU qualification. The
[prepared-hardware receipt](20261031-frozen-gradient-hardware-readiness.json)
records the dedicated build, binary/log hashes, truthful CPU metadata and the
unchanged normal backend/historical raw-data identities. No performance assay or
Windows/PC command ran for these checks.

The commands below are prepared for the **sole PC owner after cc7 admission is
reconciled and its GPU window is available**. They have not run on Windows or a
physical GPU. They use a new checkout, output directory and stop path, and read
no trained checkpoint. Use a normal hardware adapter environment rather than a
software-only Vulkan ICD.

In a fresh PowerShell console inside the owner's repository:

```powershell
$source = '53c56c6258527719927ab541f2c51d6ded95419f'
$repo = (git rev-parse --show-toplevel).Trim()
$runTag = [guid]::NewGuid().ToString('N')
$qualificationTree = Join-Path (Split-Path $repo -Parent) ('A-Life-frozen-hardware-' + $runTag)
git -C $repo fetch origin codex/frozen-gradient-hardware-fixture
if ($LASTEXITCODE -ne 0) { throw 'Source fetch failed' }
git -C $repo worktree add --detach $qualificationTree $source
if ($LASTEXITCODE -ne 0) { throw 'Isolated checkout failed' }
Set-Location $qualificationTree
if ((git rev-parse HEAD).Trim() -ne $source) { throw 'Wrong source revision' }

$runRoot = Join-Path $env:TEMP ('alife-frozen-hardware-synthetic-' + $runTag)
New-Item -ItemType Directory -Path $runRoot -ErrorAction Stop | Out-Null
$env:CARGO_TARGET_DIR = Join-Path $qualificationTree 'target\hardware-qualification'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_BUILD_JOBS = '2'
$env:ALIFE_DISPATCH_REQUIRE_HARDWARE = '1'
$env:ALIFE_SOFTWARE_PPO_CAPTURE_DIR = Join-Path $runRoot 'artifacts'
$env:ALIFE_FROZEN_ASSAY_STOP_FILE = Join-Path $runRoot 'stop-after-current-pair'
Write-Host "Synthetic output: $runRoot"
Write-Host "Graceful stop file: $env:ALIFE_FROZEN_ASSAY_STOP_FILE"

$buildJson = Join-Path $runRoot 'release-build.jsonl'
cargo test --release --locked -p alife_training --features gpu-tests --test software_ppo_controller --no-run --message-format=json 1> $buildJson 2> (Join-Path $runRoot 'release-build.stderr.log')
if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
$events = @(Get-Content -LiteralPath $buildJson | ForEach-Object { $_ | ConvertFrom-Json })
$artifacts = @($events | Where-Object { $_.reason -eq 'compiler-artifact' })
$candidates = @($artifacts | Where-Object {
    $_.target.name -eq 'software_ppo_controller' -and $_.executable -and
    [string]$_.profile.opt_level -eq '3' -and $_.features -contains 'gpu-tests'
})
if ($candidates.Count -ne 1) { throw 'Expected one exact optimized gpu-tests executable' }
$testArtifact = $candidates[0]
if ($testArtifact.profile.debug_assertions) { throw 'Expected release assertions profile' }
$boundExe = [string]$testArtifact.executable
$binaryHash = (Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant()
$libraryPaths = @($artifacts | Where-Object {
    $_.target.name -like 'alife_*' -and $_.target.kind -contains 'lib'
} | ForEach-Object { $_.filenames } | Where-Object { $_ -match '\.(rlib|rmeta)$' } | Sort-Object -Unique)
$libraryHashes = @($libraryPaths | ForEach-Object {
    $artifactPath = $_
    [ordered]@{ path = $artifactPath; sha256 = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$buildReceipt = [ordered]@{
    kind = 'hardware-fixture-build-before-execution'
    source_commit = (git rev-parse HEAD).Trim()
    source_tree = (git rev-parse 'HEAD^{tree}').Trim()
    rustc = ((rustc -Vv) -join "`n")
    cargo_artifact_log = $buildJson
    cargo_artifact_log_sha256 = (Get-FileHash -LiteralPath $buildJson -Algorithm SHA256).Hash.ToLowerInvariant()
    executable = $boundExe
    binary_sha256 = $binaryHash
    compiler_artifact = $testArtifact
    workspace_libraries = $libraryHashes
}
$buildReceipt | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $runRoot 'build-receipt-before-execution.json')

& $boundExe frozen_gradients_match_actual_controller_epochs_and_shared_restore --exact --ignored --test-threads=1 --nocapture 2>&1 | Tee-Object -FilePath (Join-Path $runRoot 'correctness.log')
if ($LASTEXITCODE -ne 0) { throw 'Hardware correctness failed; do not run the assay' }
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
```

The log must report `actual_controller_execution=required_hardware` with actual
non-CPU/non-Noop adapter identity and exact original/cache controller comparisons.
A missing admissible device is a failed qualification, not a software result.
The focused oracle covers active cached gradients, later fresh epochs, zero and
positive value coefficients, and owned/shared checkpoint continuation.

The build receipt is written before any device or training fixture executes.
It records the source/tree, compiler, selected Cargo artifact and binary/library
hashes. Both gates invoke that exact bound release executable; no intervening
Cargo selection or rebuild occurs. After correctness passes, the owner can run
the one bounded release assay in the same prepared console:

```powershell
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
& $boundExe frozen_gradient_assay::representative_actual_ppo_frozen_gradient_release_assay --exact --ignored --test-threads=1 --nocapture 2>&1 | Tee-Object -FilePath (Join-Path $runRoot 'release-assay.log')
if ($LASTEXITCODE -ne 0) { throw 'Hardware release assay failed' }
if ((Get-FileHash -LiteralPath $boundExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryHash) { throw 'Executable changed' }
Get-Content (Join-Path $env:ALIFE_SOFTWARE_PPO_CAPTURE_DIR 'frozen-gradient-release-assay.json')
```

The assay retains one warmup pair and five alternating measured pairs, with
original/cache receipts and captured words checked after each pair. Creating the
printed stop-file path from another console requests a gentle stop between
matched pairs; the current pair and its proof finish first. Keep the printed
directory, actual adapter metadata, source revision, binary/library hashes and
logs for parent review. No merge, default promotion or whole-training gain follows
from this prepared fixture.

## Applicability and remaining integration gates

Source audit at main `a9288027e1575821e83ac434c051116218795e21`
confirms that the default continuous host requests 512 trained decisions plus
one real bootstrap capture. Without capture gaps, the existing span builder
produces 256 ticks/no burn-in and 384 ticks/128 burn-in, with 256 loss rows each.
The actual controller call uses effective batch eight, two epochs, value
coefficient zero, no speech labels and no auxiliary objective. This matches the
assay's structural eligibility; the ordinary API still passes no cache request.
Capture gaps split spans, so eligibility depends on the sum of per-segment
ceil(loss rows / 256), not just the requested row count. The separate group path
uses effective batch 50 and is ineligible.

The software 19.06% paired controller reduction is not a production forecast.
Production uses continuing/truncated GAE, KL target .02 and entropy .01; the
measured assay uses terminal targets, KL 100 and entropy zero. Actual replay
varies, and the surrounding PPO timer also includes value prediction, replay
loading, reward/GAE construction and residual-critic fitting.

Any future activation belongs beside the policy-local snapshot cache and
controller call in `foundation_training_cycle/continuous_world.rs`, after full
targets/spans are constructed. Require explicit immutable/versioned input proof
without changing loader/error order: `PpoWindowSnapshotCache` retention is
optional, and every hit repeats measured host admission. Require a phase-total
device reservation covering live and offline resources, old/new replay scratch,
objective/rollback/readbacks and raw-cache bytes. The experiment's 2 MiB copy cap,
the continuous host RAM budget and the separate group auxiliary allowance do not
provide that reservation. Retention or admission failure must keep fresh replay
available. This source assessment changes no configuration, schedule or runtime
behavior and provides no physical-device qualification.
