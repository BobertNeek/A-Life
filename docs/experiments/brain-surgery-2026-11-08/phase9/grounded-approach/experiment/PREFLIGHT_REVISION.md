# Prospective six-interval revision, before any physical exposure

Original fixed eight-slot source is commit
`ebd4b6244aa4158341cfac5d72e96f3968ebe140`, tree
`74079511ef3a74feb19a5bf0931f6d714d628e56`. Its zero-interval preflight failed.

The diagnostic shows the blocked scene has five observed grounded slots, while
the four object-candidate reservations are all occupied by its four nearby cage
objects. Peer entity2 is not one of the offered target candidates. The other
three scenes each offer ActionId101 / ActionKind::Move / familyApproach for peer
or toy entity2. This is not a mistaken ActionKind lookup, invisible target or a
measured failed action. It is a missing legal observed-target candidate under the
frozen candidate budget.

Before executing any world interval, revise only execution accounting:
- Preserve all eight initial setups, geometry, pressure, genotype and sensing.
- Report both blocked slots unavailable and execute zero intervals for them.
- Run the remaining six unchanged peer/toy/passive slots exactly once.
- Do not move/remove obstacles, change attention/profile, replace the target,
  manufacture a command/candidate, loosen admission, or spend the unused slots.
- No measured failed-Approach comparison can be claimed from this revision.

The first two compilation failures were fixture-only issues: JSON macro recursion
(solved by splitting the diagnostic object) and a conflicting DriveChannel alias
(solved by using BiochemicalDriveChannel). Both occurred before tests/intervals.
The third invocation established the missing-candidate failure; the fourth added
cold candidate diagnostics and preserved the same failure. None ran biology.

The final fixture retains a create-new marker, six actual intervals maximum and
two unspent closed slots. Body measurements are still prescribed observations,
not neural-origin evidence; the opt-in store remains empty/Unknown.
