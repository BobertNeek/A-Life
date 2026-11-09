# Contacted Eat kinematic cue V2, source-only

The completed baseline diagnostic retained both frames (18,332 and 19,233 bytes), captured and self-matched both cues, and still returned strict and graded null. The consumed source differs only in Manipulation relative-velocity lane 3 among remaining graded hard gates. This candidate changes that role-specific association gate, not perception or physical execution.

## Semantics

Only EntityEat with both observed targets inside the original reach limit and in exact observed contact ignores relative velocity in association. All raw observations remain stored. Uncontacted Eat and every Move/Inspect retain the V1 velocity threshold. Chemistry, operation/payload/intensity/duration, owner, all drive/proprioception gates, coordination, target equality graph, contact and reach remain. Appearance, temperature and terrain use the unchanged attenuation; whole-event signed values retain the unchanged event-count denominator.

Nonbinding velocity is chosen rather than adding an arbitrary fitted penalty: source and query are already contacted/reached, and the verified entity Eat executor resolves current position and hand preparation, not remembered velocity. Motion still affects sensing and physical feasibility. The association does not certify feasibility, ownership, grip, safety, or future consumption. Grip and future consumption remain Unknown.

## Ownership and compatibility

FreshMealCueMemoryV2 has its own schema, owner and derived lineage, with new host inner/outer/private-token domains and sampler policy. V1 source storage and readers are reused literally as the private storage/control layer. V2 never accepts standalone V1 bytes. No acquired-life import, migration, default selector change, profile change or global numeric ID allocation is introduced. Existing V1 source files remain frozen except the new child-module declaration in visible_cue_joint.rs. Legacy host Rust type names remain the inherited local overlay API; wire/policy domains explicitly distinguish the candidate.

Full source frames are deliberately retained. Compact witnesses are deferred because the observed MISS was not a storage failure. A future projection would need a new sealed cue commitment or independently verifiable inclusion proof; self-consistent query hashes alone do not prove correspondence to the original committed frame.

## Verification state

Source written and static closure checked only. No Rust compile, Rust tests, new life, native fixture, GitHub write, or activation has occurred. The new runner, reconstruction inputs, source archive, output protocol and workflow are sealed for publication; activation is still absent. The test preserves one setup, 240 prep ticks, two acquisition/consumption intervals; pure assertions and V1/V2/cold readouts add no world steps. The comparison is explicitly a matched engineering regression, not held-out validation.

The overlay and patch use repository-relative paths; the producer-local /tmp path is not part of execution identity. SOURCE-CLOSURE.json records exact base hashes. PROPOSED-HOSTED.json describes publication prerequisites.

Contacted EntityEat association can admit acquisition or other non-consumption source events. It is not a meal-confirmation classifier. Every report includes source sequence and its original consumed flag independently; signed value remains whole-event value.

Publish exactly REMOTEFILES.json entries. Large inherited inputs are read/reused from their verified source_artifact or pinned Git blob, not copied into this tmpfs stage. Create an additions-only prepared commit directly on accepted main, then the sole fresh activation child when coordinated. No PR or legacy workflow mutation is part of the packet.
