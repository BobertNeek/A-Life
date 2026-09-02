# Legacy app milestone archive

This directory preserves retired smoke runners and report aggregators from the
G21-G23, CA16-CA44A, and S07-S09 milestone sequence. They described old
playground, packaging, tutorial, comparison, and manual soak workflows. Several
command tables no longer matched the production executable.

The snapshot came from commit `5c06e5a7af312124baef415ff4798033f2d85ae3`.
It also retains the old placeholder-art manifest and the original FVR08
cutover test that consumed the retired packaging report. The two production
launcher and package assertions remain active in the game crate.

The active game uses the production voxel frontend, current runtime checks, and
the validators documented in `docs/DEVELOPMENT.md`. Files below this directory
are not part of a Cargo target, test gate, launcher, or package.

Do not restore a complete milestone module. If a historical check is useful,
port the smallest relevant assertion to the current production path and name
the supported command or interface directly.
