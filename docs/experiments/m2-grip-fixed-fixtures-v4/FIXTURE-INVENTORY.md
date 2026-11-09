# V4 f041-only correction and diagnostic

Execute exactly one unchanged test name, once:
- f041: grounded_scene_pair_scores_and_cold_continuation
- Package alife_training, test target grip_options_host, features cpu-backend.

Retained historical contracts: f001-f040, with pinned V2/V3 receipts.
Not executed or passed here: f042-f046 (GPU fixtures remain pending).
This is not a46/46 batch. Source-audited attempt dose is a conservative upper
bound, not an observed completion count.
