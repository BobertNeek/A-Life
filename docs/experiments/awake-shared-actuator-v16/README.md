# Awake shared actuator: isolated world factor V16

This source-only package implements an explicit world API for sharing one locomotor interval between voluntary motion and acute external-contact withdrawal. It does not activate normal awake CPU/GPU callers or claim sealed learning/credit integration. Existing ordinary and sleeping-body APIs remain unchanged.

## Source and evidence

Accepted baseline: main c775c8c92e8f5e625f8a3e861ef79200199b54df, tree da127f8d53fd615f3df18e01a5cbff8c2232738c, identical to local base f83be325d47b435c9e8889d70ea64496f2b382b4.

Behavioral test source: 820041ba4985b5fb8d479f7679281667f32a6845, tree 0e597d5485faba7413d366bf2521a991921981c0. All 15 focused awake controls and the full world suite passed: 623 passed, 0 failed, 3 existing ignores. Full world includes the six arithmetic controls. The three ignores are manual endpoint-cost comparison, manual mating-query cost, and extended headless soak; none was executed or counted as a pass.

Final source: 82d8ac4b096f65469a4a80ac578e75285c205942, tree 02516c654c4efce2cb69d1d4bbbdd5c51b54754f. The only subsequent change consumes the optional mutable borrow at its final use, removing a Clippy needless_option_as_deref diagnostic. The exact two-line delta is included. Final strict Clippy/fmt and focused retest are recorded separately; do not transfer source identities silently.

## Physical behavior

Initial damaging contact receives priority over voluntary translation. New swept contact executes the lawful prefix, then spends only remaining time on withdrawal. Slow commands use actual expressed/calibration rates, not distance-as-time and not a speed retune. Free-space movement uses spatial distance for elapsed time. Existing terrain walking uses ground-plane distance, identified explicitly in the receipt; reported path lengths remain spatial. Injury is not duration-scaled. Existing effort profiles are allocated by actual segment time, with one aggregate biology advance.

The selected bundle remains unchanged. Per-channel receipts describe only executed voluntary motion; involuntary displacement is separate, while body velocity and aggregate consequences include both. Blocked attempts, exact-center/unknown-normal fallback, opposed normals, carried object ownership, thermal endpoint/intermediate exposure, zero-net motion without sleep recovery, rollback, and JSON restore/no residual latch have fixed controls.

## Preserved failures and limits

The early compile failure was non-Ord entity sorting. A slow-command fixture then used the wrong expected rate; correcting its expectation did not retune production physiology. Fresh effector/proprioceptive values are expressed/calibration gains, not acquired learning. Later diagonal/vertical tests reproduced a genuine candidate XZ-time bug before the metric correction; both failed assertions are retained. The strict Clippy failure and final borrow-only repair are retained.

The earlier mistaken statement that a hazard profile overwrote effort with damage-only was retracted before this implementation; the explicit erratum is included. No generalized escape, learned adaptation, recurrent motor reconciliation success, cognitive wake, or automatic aroused neural write is claimed. Internal late-action injury is not yet an awake trigger. Ordinary sealed CPU/GPU integration must preserve the separate physical evidence, one credit/biology interval, rollback, retry, and checkpoint contracts before activation.

Only source, source identities/hashes, finite test logs and scalar reports are included. No executable, generated save, checkpoint, bank, or learned array is included.

Final 82d8ac4 validation completed: 15 focused tests passed; strict world all-targets Clippy and workspace fmt passed. No further native process remains.
