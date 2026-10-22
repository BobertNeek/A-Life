# Sealed training artifact reuse

The CPU care, mixed GPU care, and legacy held-food schedulers reuse exact-byte
sealed artifacts in a lane-parent `.training-artifacts` directory. This removes
physical duplication of repeated source snapshots and cumulative CPU receipt
ancestry without changing checkpoint names, JSON encoding, receipt hashes,
optimizer ages, replay data, or native validation.

The store uses ordinary filesystem hard links, not symlinks or external manifest
pointers. New objects are published by linking the already sealed file, avoiding
a second temporary payload copy. Duplicate files are replaced atomically only
after both copies pass SHA256 verification. Store objects and their aliases are
read-only. Filesystem hard-link limits roll over to another verified object.
Existing read-only files that are not already the selected object are left alone.

Only native-accepted, hash-pinned files enter the store. Mutable ledgers, logs,
partial attempts and dispatch histories do not. CPU retained receipt blobs are
verified against their digest filenames. Corrupt objects, unsafe paths and
unsupported filesystems produce an error with the accepted checkpoint retained;
a partial optimization is recoverable and rerunning it is idempotent. The
existing disk/memory/PC-activity guards and seed reconciliation still apply.

A checkpoint remains readable if the store's own directory entries disappear:
each checkpoint has its own hard-link references. Copying or archiving a checkpoint
normally produces self-contained files. Do not edit a sealed alias in place or
clear its read-only flag: aliases share an inode. Native admission still verifies
content hashes before reuse, rather than treating read-only permissions as proof.

This is duplicate elimination, not unlimited retention or a whole-disk quota.
Unique learning snapshots and replay still occupy space. Directory entries also
have a cost, and exporting an archive can materialize duplicate bytes. No old
learning, original individual archive, replay or recovery package is deleted by
this change. `logical_bytes_linked` reports replaced logical payload bytes, not a
measurement of freed physical disk space.

## Deployment and verification

Finish/reconcile the current attempt and prepare a new plan from the actual latest
accepted checkpoint. Do not edit an old plan's source/helper pins to make it pass.
No trained brain or actor/value Adam state is reset. CPU plans also pin the new
storage helper. The mixed GPU scheduler retains its clean-checkout/commit check.

- `python -B -m unittest discover -s scripts -p test_training_artifact_store.py -v`
- `python -B -m unittest discover -s scripts -p test_cpu_care_serial.py -v`
- `python -B scripts/check_training_artifact_native.py PATH_TO_train_cpu_care`

The native check creates only a synthetic development fixture, compares every
payload hash before/after reuse, and reruns native admission/restoration on both
checkpoint copies. CI exercises Python reuse on Linux and Windows, and native
CPU admission on Linux. None of these tests claims production throughput gains.
