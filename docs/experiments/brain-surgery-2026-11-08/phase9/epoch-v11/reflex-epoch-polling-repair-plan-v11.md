# Repeated disposable regression: real worker polling repair

Source270cee772c15afc132595b561d8f556177ec92d0/tree83771fe78566b074f89db2f283e3ba0c73ec667e, parent frozenf8e71f109a8792e54376e1fb8c07763fe195e2ea. One test-only file changes: wait for checkpoint_durability to return from the active real worker instead of unwrapping its legitimate None phase; use a new local fixture-directory label to preserve all first-run assets. Patch SHA2565481e326d4edb07357aab07288d808068f9c66f095fa557d14ad86620162f373.

Production/candidate logic, world recipe, seed31118, assertions and one-decision/two-interval bounds are unchanged. This is a repeated disposable regression after a fixture-construction failure, not a new behavioral cohort. First run actual SwiftShaderCpu/Vulkan evidence and its executable061fbf24/sourcef8e71f1/log remain frozen.

The real checkpoint lifecycle takes checkpoint_durability into its immutable worker in checkpoint_poll.rs; Finalizing returns report.durability and queues a follow-up GPU capture. The next poll, not the same Finalizing return, moves that returned Option into the follow-up worker. The repaired test uses real polls until Some at tick1, preserving all dirty/new-generation, old/new actual-asset restore and final publication assertions. It does not fabricate or substitute a published snapshot.

Compile a distinct frozen executable and record its hash before the exact same single runtime test. Host-only gate/recovery tests may run from it without world allocation. Preserve any further failure separately; do not claim later milestones that were not reached. No additional runtime allocation or awake motor-budget experiment is part of this run.
