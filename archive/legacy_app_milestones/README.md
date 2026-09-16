# Legacy app milestone archive

This directory preserves retired smoke runners and report aggregators from the
G21-G23, CA16-CA44A, and S07-S09 milestone sequence. They described old
playground, packaging, tutorial, comparison, and manual soak workflows. Several
command tables no longer matched the production executable.

The first snapshot came from commit `5c06e5a7af312124baef415ff4798033f2d85ae3`.
The follow-up snapshot from `130a0f108f1f5dc4a4388496f08d4c76e95c5615`
adds the unused CA22 ecological soak, CA25 curriculum authoring, CA26/CA27
semantic smoke wrappers, S02 interactive control panel, and four debug-only
presentation snapshots. It also retains the old placeholder-art manifest,
the CA25 example lesson, and the original FVR08 cutover test.

The active semantic provider implementation remains in `alife_semantic`. The
active game keeps its production playback state and scheduler without the old
fixture-control panel.

The active game uses the production voxel frontend, current runtime checks, and
the validators documented in `docs/DEVELOPMENT.md`. Files below this directory
are not part of a Cargo target, test gate, launcher, or package.

Do not restore a complete milestone module. If a historical check is useful,
port the smallest relevant assertion to the current production path and name
the supported command or interface directly.
