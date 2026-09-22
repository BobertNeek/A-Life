# N2048 training operator

Use PowerShell 7 from the authoritative repository root. This launcher prepares a **pilot**, not a training campaign. Its own checks and pilot have not been run merely because these files exist.

```powershell
Set-Location -LiteralPath 'D:\A life'
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Status
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Prepare
```

Before `Prepare`, the supervisor must release the Cargo/GPU lane. Do not run another Cargo command, game, GPU test, or training process alongside it. The launcher takes an exclusive file lock and rejects existing project processes; that lock only coordinates callers that respect it. It never kills processes.

Preparation builds `train_n2048_care` with `foundation-training`, runs exact serial gates (including the PPO GPU objective and selective merge), requires one executed passing test per gate, and runs the supported `--pilot` command for a fixed 32 ticks. Cargo uses existing default features and dependencies offline, two build jobs, and the development profile. This is correctness evidence, not a release-performance measurement. No source edits, downloads, legacy trainer fallback, or full test suites are performed.

Evidence lives in a fresh `target/founder-training/prepare-*` directory. `source/` records HEAD, the complete tracked binary diff, and hashes of nonignored new files under source/docs/scripts/assets directories. Artifact folders, Blender backups, and Python caches are excluded. Source identity is checked between phases and after the pilot. Each command has separate stdout/stderr logs. `status.json` is atomically published both in that directory and at `target/founder-training/status.json`.

`Prepared` means the listed gates and pilot succeeded for that exact source, executable, and receipt. The executable enforces replay tolerances; the launcher additionally rejects missing, negative, or nonfinite receipt metrics. Matching successful evidence is reused. `Failed` means stop and inspect the named phase and logs with the supervisor. Do not change tolerances, skip a gate, repeatedly retry, delete receipts, or modify source just to change its fingerprint. An interrupted `Running` receipt also needs supervisor diagnosis. Check status with the first command above.

`-Mode Campaign` deliberately fails. The executable does not yet expose an eight-hour campaign, cohort scheduler, compact replay, protected acceptance runs, or resumable campaign checkpoints. Existing `initial.alife-foundation`, lineage files, and `pilot.json` are **not** actor/critic/optimizer/RNG/curriculum checkpoints. Do not resume from them or claim a trained/promoted founder. The current development plan is [the training regimen](n2048-care-training-regimen-2026-09-21.md): pin the genetic foundation for each life/cohort and update between cohorts while preserving ordinary lifetime learning.
