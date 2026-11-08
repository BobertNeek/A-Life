# V21 bounded candidate endpoint correction

This preserves a measured failed prerepair and the following finite correction, separately from V20 and the later common-planner experiment.

Source 11d323c982c37604bf451e6aabf65b3d70122dfc, tree ef6e52d1d1aa216e89d22c0de68c145c7977fac1. Native prerepair: 0 passed, 1 expected failure. Native corrected focused world tests: 25 passed, 0 failed. The signed-coordinate/speed test includes 24 nested cases. No additional CPU credit, full-world, Clippy, GPU or gameplay qualification is claimed here.

The correction contracts a rounded endpoint by at most eight next-representable coordinate steps toward the start when the existing movement guard would reject it. It preserves the original speed cap and planned duration, re-resolves terrain, and keeps collision, grip and carry validation. It does not raise a tolerance or reset learned state. Candidate-only correction remains explicitly altered for credit.

The X128 prerepair reproduced an ordinary endpoint above the declared cap and a rejected research candidate. The corrected suite also admits the X1000 half-speed meal while retaining rollback, blocked-move, terrain and signed-coordinate controls. The ordinary/candidate discrepancy motivates the separately versioned V22 common planner; this V21 result is not its evidence. Curved-ground prefix handling and factual memory during genuine intervention remain unresolved.

The focused-results directory is copied byte-for-byte from its safe manifest-bound packet, including original scalar failure logs and patches. Only executable hashes are included, never executables, checkpoints, learned arrays or retained lives. Historical notes and files remain unchanged.

Method reference: [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), especially targeted repair (experiment 4), whole 20,486-weight game networks (experiment 5) and iterative feedback (experiment 6). These inspire iteration; they do not validate A-Life behavior.
