# Passive v3 build provenance addendum

This addendum follows revision 3 without changing its frozen source, executable or measurements. The public v3 results are observations of the recorded executable. Retained build logs establish recompilation of world, runtime and training source, but do not establish that the core dependency was rebuilt from this copied worktree. Reuse of a shared Cargo target with preserved source timestamps can allow stale dependency reuse. No contemporaneous dependency-fingerprint archive remains to resolve that historical uncertainty. This is an attribution limit, not evidence that a specific different core was used.

The [bounded provenance inventory](reader-v3/build-provenance-addendum-v1/BUILD-PROVENANCE.json) preserves log hashes and project compilation lines. [Details](reader-v3/build-provenance-addendum-v1/README.md) distinguish intended-source and executable hashes from a hermetic build proof. No raw build logs, private state or new creature exposures are published here. All prior archive bytes remain unchanged.

The next four-life OFF-versus-low-gain comparison will refresh copied source timestamps, verify unchanged content, and retain the actual dependency compilation evidence before freezing its executable. It uses fresh seeds and the same tested moving-Rest correction in both arms; it does not rerun closed v3 lives.

The original method remains [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/): targeted repair, whole-policy construction, and iterative measured revision. Build provenance is part of making that iteration reproducible.
