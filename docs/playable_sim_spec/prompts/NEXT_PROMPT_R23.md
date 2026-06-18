You are executing the playable-sim review gate R23.

Use only:
- docs/playable_sim_spec/review_gates/R23_feature_complete_rc_review.md
- docs/playable_sim_spec/plan_manifest.json
- docs/playable_sim_spec/GAME_PHASE_PROGRESS.md
- the completed G01-G23 implementation and release-candidate docs

Do not start G24.
Do not add new runtime features.
Do not hide release-candidate gaps.
Do not create P37.

Run the R23 review checklist, produce the required R23 review receipt, and stop. If the verdict is PASS, say that G24 may proceed only after explicit user authorization. If the verdict is FIX_REQUIRED or BLOCKER, include the exact fix prompt and do not proceed to G24.
