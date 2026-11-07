# Held-food repertoire check, read only

The current ObjectSlots repertoire includes GRAB (action 211), family Contact, kind Interact for every selected object. In headless.rs, execute_grab toggles the target's carried_by ownership: choosing the same legal object already held releases it. There is no separate Drop candidate. The targetless NO_MANIPULATION candidate does not release anything.

All six object action families share the same 24 grounded object features. Lane 18 is generic physical contact, not a possession flag; lanes 19/20 are linear/angular proprioceptive speed. Ownership is sensed through the global tactile_contact[2] grip channel, with carried pose/relative velocity and held-object visibility/contact reservation. A recurrent policy can use that evidence, but the current negative Contact fear/pain readout does not itself distinguish grabbing from releasing a feared target.

Native Eat/Play can place an activity-held item before switching to a new legal target. Deliberate Grab possession is protected from automatic replacement. None of these operations require a new hidden hazard label, and this experiment changes none of them.

The frozen v11 reader-on/cap 12 journal gives concrete evidence. It acquired A at index 48 and successfully placed A when selecting Eat(B) at 49. It acquired A again at 72, carried it through 73–80, selected Avoid four times during those held rows and never selected the available Contact(A) release. Contact(A) scores were approximately -1.83 to -1.90. At 81 it consumed A while its locomotion choice was Avoid. The held target follows its grip, so fleeing does not discard it. This does not establish broken physical rules.

The v12 analysis will separately report possession duration, explicit release candidates and selections, automatic placement, final held state, and safe B engagement. No release action, candidate feature, genetic weight, world rule or stopping criterion is added to the predeclared factor.

Source references: candidate_enumerator.rs OBJECT_ACTIONS and enumerate_grounded_candidates_with_focus; grounding.rs GroundedObjectSlotV1::candidate_features; grounded_sensing.rs proprioception construction; headless.rs active_contact nomination, tactile_contact[2], execute_grab, sync_carried_pose; headless/hands.rs prepare_hands_on and activity_placement.
