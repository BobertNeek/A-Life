# Body turning during rest: source assessment, not a new rule

Source: merged main 6afad15b68820d56e3a419404ecb74c1938aca29, tree f836215d2b0bb839a5afa7bf511a87c03e9eebc2. The checked local rest-motion-main-publication-v1 worktree has identical source bytes. PR303 is merged and is not held for this assessment.

## Conclusion

Do not automatically forbid all body yaw during Rest. A small postural adjustment can coexist with physical rest. The current code does allow full recovery while turning, but that observation alone is not proof of a biological defect or of cognitive sleep while turning. The existing one-event recovery magnitude is a separate inherited-chemistry calibration question. No yaw rule, extra life, or native test was introduced here.

## What the source establishes

In alife_world/src/headless.rs:
- HEAD_SWIVEL_STEP is 20 degrees. A TurnLeft/Right changes body_yaw by that amount times the adapted intensity, wrapped to [-pi, pi). It synchronizes the carried pose. Actual commanded turns can be smaller because embodiment/proprioceptive gain scales intensity.
- A carried-pose obstruction rejects the turn before body_yaw changes. It still reports physical energy cost .01. This makes requested Turn and cost insufficient evidence of actual rotation.
- LookLeft/Right changes head_yaw only, clamped at a head limit. LookCenter recenters it. HoldGaze changes neither. Head movement must not be mistaken for body locomotion.
- Successful body Turn reports physical energy_cost .01; head Look reports .005. Both use OutcomeProfile::look(), whose actual BodyEventDelta.energy is -.002. Actual translation uses -.04. These are signed event inputs before inherited metabolic turnover/organ allocation, not measured joules.
- Those look/turn costs do not scale with realized angular change. Even a tiny turn, a head movement clamped at its limit, and a blocked body turn do not provide a reliable angular-work measure through cost alone.
- All these orientation outcomes report zero organism translation. The merged PR303 helper therefore deliberately leaves their Rest recovery unchanged.

At 20 world ticks per second, the full nominal command is up to 20 degrees per 50 ms, but this is a source-defined bound, not measured living behavior. Repeated high-rate turning and an occasional restful adjustment should not be treated as equivalent without further evidence.

## Physical recovery and cognitive sleep are different

BodyState::apply_event in alife_core/src/biochemistry.rs computes recovery from sleep_recovery times metabolic turnover, and sets body.sleeping whenever recovery is positive. The graph uses that flag for Awake/Sleeping emitters and sleep metabolic regulation. Rest currently contributes a full sleep_recovery event, so this boolean also marks a restful interval; it does not prove the cognition controller has entered sleep.

The CPU host (alife_training/src/cpu/host.rs) evaluates the SleepController from homeostasis and neural affordability, seals SleepPhase, and skips ordinary action selection when the old or new cognitive phase is non-awake. A positive body.sleeping flag alone is not that phase gate in the ordinary selection loop.

The GPU staged tick calls scheduled_tick_with_organism. That scheduler separately evaluates homeostatic entry, consolidation progression, and pressure/fatigue wake release. Its biology input can include recovery for a previous resting/sleeping body, but the cognitive phase remains explicit and independently sealed. Existing advance_biology_once boundaries prevent a second biology advance at the same tick. Full Turn+Rest therefore should not be described simply as an asleep brain choosing to turn.

Rest recovery can still affect later cognitive entry indirectly by lowering fatigue/pressure. The inherited waking recipe removes direct sleep-to-ATP manufacture, but its large recovery gains and slow accumulation were already investigated in the separate chemistry candidate. No chemistry parameters are changed by this assessment.

## Missing measurement and smallest honest receipt design

HeadlessMotorChannelReceipt currently stores the adapted command, phase, transfers, selected chemistry doses, optional observation, and PhysicalActionOutcome. None retains before/after body yaw or measured angular displacement. measured_motor_channel_observation returns a measured observation only for the locomotion channel. Inferring rotation from TURN_LEFT/RIGHT would be dishonest for blocked, zero-intensity, clamped, or future modified operations.

If a follow-up needs angular evidence, the smallest local design is:
1. Capture actual actor body_yaw and head_yaw immediately before and after each executed channel at the world boundary.
2. Store a small measured record with wrapped signed body_delta_radians and head_delta_radians alongside that channel's receipt. Keep head and body deltas separate. For the current bounded turn operation, signed shortest-angle difference avoids the +/-pi representation discontinuity.
3. Preserve each segment. Derive total absolute body angular travel from the segments when needed, rather than using final net yaw; opposite turns must not erase the evidence. Future reflex rotations need their own measured segment too.
4. Retain zero actual body delta for blocked turns and zero actual head/body deltas for no-op gaze. Preserve costs, rollback, and one biology advance.

This is evidence instrumentation, not yet a sleep rule. HeadlessMotorChannelReceipt is presently non-serialized; adding a local measured field need not migrate organism saves. Propagating it into canonical neural/learning evidence would be a separate, explicitly versioned interface decision, not an implicit reinterpretation of old records.

Before changing recovery, choose a biologically defensible distinction between occasional low-effort adjustment and sustained locomotor turning. The present code lacks an angle-sensitive energetic model, so no numerical cutoff or blanket prohibition is justified by this source inspection alone. Compare stationary Rest, small/full realized body turns, gaze-only motion, blocked turns, and opposing rotations in bounded native fixtures after a rule is justified. Do not use requested actions or event cost as a substitute for measured rotation.
