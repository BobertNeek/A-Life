# Recovery and handoff after rejected V32

The actor/private lineage is recoverable from a compact, exactly indexed set of
existing files. A complete **joint actor/critic** handoff is unavailable: this
supervised pilot never saved a matching critic or critic-version binding. No
critic is initialized or substituted to disguise that gap.

The detailed metadata-only manifest stays local at
`/workspace/language-recovery-handoff-v33/manifest.json`, with handoff notes in
the same directory. It records exact hashes/sizes, dependency and provenance
parents, original/source and final private state, saved Adam boundaries/gradients,
seed exclusions, successful cold receipts and command limits. No payload bundle,
archive, transfer or independent backup was created. Every original and failed
artifact stays in place; training remains stopped and V32 remains rejected.

| Selected existing file set | Files | Exact bytes |
|---|---:|---:|
| Original actor/private snapshot | 2 | 3,560,053 |
| Four current actor/private snapshots + known positive start | 11 | 6,904,067 |
| Four-arm recovery plus saved436→440 update states/gradients | 32 | 9,240,739 |
| Above plus compact negative-score/provenance evidence | 129 | 17,792,710 |
| All recorded private snapshots and scored/sleep traces | 366 | 298,668,828 |
| Compact evidence plus exact measured executable | 130 | 117,778,374 |

Sizes are sums of intact existing files, not a new package or estimated archive.
They exclude source checkout, OS/compiler/registry dependencies and the small
manifest itself. The minimum listed actor-history/private closure is9.24MB; its
compact evidence closure is17.79MB. No full actor/critic package size can be
honestly stated without the missing critic.

The original V24 recovery embeds compiler inputs, foundation data, full actor
Adam state and private host. V26 Untouched changes only its explicit optimizer
stage mask. The saved ZeroGradient4/Balanced4 controls and rejected MatchedFactual4
retain complete weights, moments, integer ages and configuration, including all
saved437..440 boundaries and real gradients. This preserves current Adam history;
it does not claim every pre436 gradient trajectory. Final/after4 byte-identical
files are indexed as aliases without deleting either. Original input/private
snapshots and current final states remain distinct.

GitHub holds source commits and selected source-only reports. Exact trained and
private inputs remain **workspace-only in the verified inventory**. Independent
payload copies are unknown. Redundant V23/V24 snapshots on one filesystem are
not independent backups. Trained-state upload authorization is still unresolved;
no retry or transfer was attempted.

## Safe verification

The new source-only
[`verify_language_recovery_handoff.py`](../../scripts/verify_language_recovery_handoff.py)
reads only manifest-selected files and performs no engine, neural, world,
optimizer or external-model call:

```sh
python scripts/verify_language_recovery_handoff.py /workspace/language-recovery-handoff-v33/manifest.json --profile actor_history
python scripts/verify_language_recovery_handoff.py /workspace/language-recovery-handoff-v33/manifest.json --profile compact_evidence --rescore
python scripts/verify_language_recovery_handoff.py /workspace/language-recovery-handoff-v33/manifest.json --profile full_recorded_evidence --rescore
```

All three commands pass. The full profile checks366 selected file hashes and191
previous native checkpoint identities directly from serialized JSON bytes. It
uses24 existing successful cold-recovery receipts covering176 checkpoints,
without rerunning native restoration. The compact rescore reproduces704 saved rows and the rejected result:
the two fresh seeds retain per-arm paired totals4/0 meals,4/0 timely latest
requested reports and1/6 premature declarations. Compressed scored streams are
only byte-hashed; no historical43GB decode scan is repeated.

The old full scorer requires all762 original paths; it is not a compact-closure
command. The old experimental reload CLI runs world/assessment steps and may
update Adam, so it is unsuitable here. The old test verifier would fail on its
create-new receipt guard in the existing output directory; no in-place retry or
payload copy is used to evade that guard.

A standalone native load-only CLI is not available. The documented native
constructor recipe is source-inspected and backed by prior successful cold
receipts; it is not newly executed. Cross-platform restoration or a bit-identical
future source rebuild is unverified. Native source is pinned to
`ad9dc5a8410a34bc8d3d0770ea7df09f9b37e3b0`, result source to
`1bab7e2d01e9f36a6d3a8b15deb726ad70f37f13`, and optional executable to SHA256
`a25a715602b1cdcce7f9b8adef8e72a2986dd149ff6f7a547a5b099f21345019`.

Only metadata/source files are added. The482148352-byte free-space floor remains
preserved. The safe next step is review or metadata verification; independent
backup needs the unresolved transfer authorization/destination, and joint-PPO
continuity needs matching critic/version state. No promotion, new training,
archive, PR/model-triggering route or automatic next experiment follows.
