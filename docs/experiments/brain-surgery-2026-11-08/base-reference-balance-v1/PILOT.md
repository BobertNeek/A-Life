# Reference-balance pilot

Executable compiled successfully (dev, native CPU) from source 205f60c200dc05f4f0ae8c02d3f5b9747dbc0abd plus reference-balance.patch. SHA-256 is in pilot-binary.sha256. A subsequent attempted test build was interrupted because tmpfs was filling; it was not a passing test. No promoted asset or retained life was changed.

One fresh paired seed, 2026111501, two physical scenarios, 12 steps each: both v6 and reference-balanced v6 consumed nearby food (target inactive and energy rose from 0.4 to about 0.536). Neither consumed distant food by the short endpoint. Final x was 0.604 for v6 vs 0.302 for balanced, and final energy about 0.398 in each. Sampling trajectories diverged; one short seed cannot establish comparative performance. Exact cold-next-step checks and unchanged founder optimizer flags passed on all four runs. The original whole-care candidate differs from its source genetic weights at 29,264 of 32,768 positions in this seed; that is not the number of extra reference edits in this comparison and is not a learned-weight count.

A separate paired 2-seed comparison across food, distant food, bitter food and toy was started with 32-step endpoints. Pilot outcomes do not decide the edit. Its logs and complete per-trial receipts are in comparison/ when they complete. Candidate source is direct action decoder reference weight changes, not new wiring.
