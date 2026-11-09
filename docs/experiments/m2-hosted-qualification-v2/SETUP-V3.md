# Source-only V3 setup correction

V1 and V2 files/seals/archives are preserved unchanged in local history. This
named revision inherits the remotely verified V2 prepared tree, but publishes
only the changed/new files listed in UPDATE.json on a NEW V3 branch. V2 run
42575100065 is terminal and must not be retried. Its TOOLCHAIN cause is UNKNOWN.

The narrow change is the audited official dtolnay toolchain recipe in the normal
hosted rustup home, with private output and explicit rustup run commands. The
upstream composite action itself is not run unchanged because it tees output.
The exact public provenance and deliberate bounded deviations are recorded in
TOOLCHAIN-PROVENANCE.json. New failure codes distinguish discovery, home,
provenance, install, sysroot, version, Cargo and binary hashing. Generic resource
codes retain the current static setup stage. No stderr, discovered path, raw
version or exception text is exported. Compiler location projection and partial
test completion from V2 stay intact.

Rust variants, fixture lists and call bounds remain byte-identical. Boredom V2
six cases and world-reflex 22 cases are the only eligible selections. Meal/GDS
remain disabled; old meal bc source is historical, not the current locally
qualified ownership-bound V2 host. No PR, marker, dispatch, publication, native
build, native fixture, world tick or dependency allocation was done here.

Review UPDATE.json, FINAL-CHECK.json, STATIC-CHECK.json, SYNTHETIC-TESTS.json and
TRIGGER-AUDIT.json. Parent must verify the new prepared remote head and no marker
or prior V3 run, then add one new frozen V3 marker if proceeding. Never reuse V2.
