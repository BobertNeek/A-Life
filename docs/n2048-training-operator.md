# N2048 training operator

Use PowerShell 7 from the authoritative repository root. The guarded launcher prepares a **fidelity pilot**, not a training campaign. A separate training-only CLI now runs a bounded real-world cohort cycle; it has not passed the behavioral gates for an overnight campaign.

```powershell
Set-Location -LiteralPath 'D:\A life'
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Status
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Prepare
```

Before `Prepare`, the supervisor must release the Cargo/GPU lane. Do not run another Cargo command, game, GPU test, or training process alongside it. The launcher takes an exclusive file lock and rejects existing project processes; that lock only coordinates callers that respect it. It never kills processes.

Preparation builds `train_n2048_care` with `foundation-training`, runs exact serial gates (including the PPO GPU objective and selective merge), requires one executed passing test per gate, and runs the supported `--pilot` command for a fixed 32 ticks. Cargo uses existing default features and dependencies offline, two build jobs, and the development profile. This is correctness evidence, not a release-performance measurement. No source edits, downloads, legacy trainer fallback, or full test suites are performed.

Evidence lives in a fresh `target/founder-training/prepare-*` directory. `source/` records HEAD, the complete tracked binary diff, and hashes of nonignored new files under source/docs/scripts/assets directories. Artifact folders, Blender backups, and Python caches are excluded. Source identity is checked between phases and after the pilot. Each command has separate stdout/stderr logs. `status.json` is atomically published both in that directory and at `target/founder-training/status.json`.

`Prepared` means the listed gates and pilot succeeded for that exact source, executable, and receipt. The executable enforces replay tolerances; the launcher additionally rejects missing, negative, or nonfinite receipt metrics. Matching successful evidence is reused. `Failed` means stop and inspect the named phase and logs with the supervisor. Do not change tolerances, skip a gate, repeatedly retry, delete receipts, or modify source just to change its fingerprint. An interrupted `Running` receipt also needs supervisor diagnosis. Check status with the first command above.

`-Mode Campaign` deliberately fails. The CLI can run a single cohort and resume its actor/value optimizer state in another bounded cohort. For a focused operator check, choose fresh output directories and run:

```powershell
cargo run -p alife_game_app --features foundation-training --bin train_n2048_care -- --cycle target/founder-training/cycle-001 4
target/debug/train_n2048_care.exe --resume-cycle target/founder-training/cycle-001 target/founder-training/cycle-002 4
target/debug/train_n2048_care.exe --inspect-cycle target/founder-training/cycle-001
```

The cycle collects ordinary GPU decisions, streams Zstd-compressed binary replay records with a 4 GiB disk ceiling and 8 GiB process-memory check, predicts frozen values, calculates GAE across natural sleep gaps using actual world time, applies PPO in 256-tick waking windows with up to 128 burn-in ticks, exports a canonical N2048 asset, and verifies its admission into a fresh cohort. Structural synapses remain frozen replay context while inherited weights train. The first waking decision after each sleep must pass exact GPU production/replay parity. The sealed experience patch retains its existing JSON wire inside each binary record. The second command restores the exported actor, Adam state, and GPU value head, and increments the policy version. `--seed N` may change the world seed on resume while preserving the original inherited founder seed and graph; `cycle.json` records both. `--inspect-cycle` reads and checks sealed replay records, including finalized records from an interrupted collection. Both training commands fail rather than overwrite an output directory. `cycle.json` and its actor/value checkpoints are integration evidence, not a full campaign checkpoint or a promoted founder. The cycle accepts at most 36,000 waking decisions and fails if the organism dies.

`--food-after-world-tick N` in cycle mode temporarily moves the existing food resource to a distant legal world position, then returns it at world tick `N`. `scenario.json` records both positions, and `cycle.json` records the availability tick, first meal, and energy. It changes no physiology or policy. `--teacher-pilot OUTPUT [TICKS] --seed WORLD_SEED --founder-seed-base GENOME_SEED --food-position X Y` produces a source-bound compact demonstration replay in the same format as the cycle. The grounded teacher follows the nearest sensed object and stops at a sealed meal; the output is a feeding lesson, not a balanced care corpus.

Do not start an overnight campaign yet. A release 8,000-decision cycle completed 59,565 world ticks (49.6 simulated minutes) with 98 meals, energy 0.7925 initially / 0.2764 minimum / 0.8562 finally, five actor/value optimizer steps, and exact next-cohort admission. Collection took 45.1 wall minutes and PPO update 73.6 seconds. A two-decision offset-food teacher pilot approached, consumed, wrote two digest-checked replay records, and passed production/replay logit agreement below 1e-6. A short delayed-food cycle passed the world-owned move/return path. The required 20-minute food-free interval remains untested. Balanced grounded demonstrations, protected behavioral development/acceptance panels, campaign throughput, and a durable deadline/best-checkpoint coordinator still remain. The current development plan is [the training regimen](n2048-care-training-regimen-2026-09-21.md).
