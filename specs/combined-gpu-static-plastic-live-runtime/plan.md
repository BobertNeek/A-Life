# Plan

1. Add a backend mode that requests `GpuPlastic` and permits CPU-shadow-guarded
   static scoring.
2. Add an app-level smoke mode and CLI parser entry.
3. Reuse the existing live sequence: static report, proposal construction, live
   tick, sealed patch, post-seal plasticity, delta batch application.
4. Extend summary fields for combined mode, post-seal receipt, replay
   protection, and unsupported full-runtime gap.
5. Add CI-safe tests that pass under CPU fallback and assert full combined
   evidence when GPU dispatch is available.
6. Update product evidence docs.
7. Run focused GPU commands, full validation, R2 review, merge, and push.
