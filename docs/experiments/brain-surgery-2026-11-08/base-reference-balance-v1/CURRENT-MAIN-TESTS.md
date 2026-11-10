# Current-main experimental-brain tests

After the CPU behavioral comparison, the following tests ran successfully offline on main c3aec16df33861f1316d127736f5f60eea327ede with `ingest-boost.patch` applied only to the research example and its two pinned support modules. Rust 1.99.1, CPU backend, development/test profile:

- `cargo test -p alife_core --test n2048_experimental_graph`: 3 passed. Includes experimental route/parameter budget checks, identity roundtrip and rejection as a default foundation, and changed-recipe/topology rejection.
- `cargo test -p alife_training --features cpu-backend --test cpu_experimental_graph`: 1 passed. Includes rejection of an exact CPU checkpoint across the experimental/default graph boundary.
- `cargo test -p alife_training --features cpu-backend --example probe_weight_space_care`: 1 passed. Tests the authored builder's signs and need-axis separation; it is not a separate end-to-end test of the two-weight candidate.

The first attempt to link the example test failed with a linker bus error when the temporary filesystem reached 100%. Rebuildable intermediate files were cleared after preserving and verifying the research executables; the retry passed. These focused CPU tests do not establish full workspace CI, GPU execution, migration of existing learned lives or functional learning. The cross-graph rejection is a useful guard, but also means this experimental brain cannot simply replace the default or reinterpret existing saved weights. The candidate remains unpromoted.
