# V4 bounded qualification correction

V3 is terminal, with setup passed in both jobs but zero completed fixtures.
V3-RESULTS.json preserves only its sanitized codes/locations and the parent's
listing-only observation. Raw native diagnostics remain private.

Runner: apart from new branch/plan/marker/work-directory identity, the only
changes are the two --list commands' --format argument: pretty replaces terse.
The strict parser, count/uniqueness requirements, exact ignored whitelist and
all proven setup functions are unchanged. No fallback accepts footerless output.

Reflex source: a private world-local recovery_reflex_channel_key match mirrors
the pinned core canonical key exactly: Locomotion 0, Orientation 1,
Manipulation 2, Vocal 3, Posture 4, SpeciesSpecific(id) 0x100 + u16::from(id).
Only the two inaccessible calls are replaced. Removing that helper and reversing
those calls restores the original E0624 source byte-for-byte. All other four
reflex files, including the 22 fixed fixture bodies, remain unchanged.
The finite witness checks all 261 enum values, exact keys, injectivity and u16
range. Therefore sorting and multiset equality preserve duplicate multiplicity
and accepted/rejected channel sets. No public API, cast, wildcard, guard removal,
new channel, source factor, scheduler, physical action or serialization change.
Future enum/core mapping changes fail the witness instead of silently widening.

The corrected full patch and source bindings are hash-bound separately from the
preserved original. The small incremental patch is also supplied in reflex-key-v2.
This is source-equivalent reasoning and synthetic checking, NOT native fixture
qualification. The unchanged 22 physical fixtures remain necessary on V4.
Boredom Rust stays byte-identical. Meal/GDS remain disabled.

UPDATE.json is the minimal UTF-8 publication map, with expected old blob hashes.
Create V4 only from dcace9456304e58e442a7423f934102eac1f4323. Keep inherited V2/V3
workflows unchanged and branch-ineligible. Parent reviews/publishes, verifies the
new prepared head and ledger, then adds one new V4 marker if proceeding.
No local native command, installer, world/source fixture, publication, PR,
marker, dispatch, deletion, target allocation or dependency download occurred.
