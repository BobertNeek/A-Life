You are executing the playable-sim review gate R13.

Use only:
- docs/playable_sim_spec/review_gates/R13_retrospective_product_boundary_review.md
- docs/playable_sim_spec/plan_manifest.json
- docs/playable_sim_spec/GAME_PHASE_PROGRESS.md
- the completed G01-G13 implementation and docs

Do not start G14.
Do not implement new runtime features.
Do not modify alife_core unless a release-blocking dependency leak is found.
Do not create P37.

Run the R13 review checklist, produce the required R13 review receipt, and stop. If the verdict is PASS, say that G14 may proceed only after explicit user authorization. If the verdict is FIX_REQUIRED or BLOCKER, include the exact fix prompt and do not proceed to G14.
