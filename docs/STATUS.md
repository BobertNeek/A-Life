# Current status

This describes source integration as inspected on 21 October 2026. It does not
report a new hardware run or playtest. Dated measurements and their source
revisions are linked in [Evidence](EVIDENCE.md).

## Production game

`production-voxel` restores a selected save and a required GPU neural runtime.
The world constructs grounded perception and unscored legal candidates; WGSL
selects actions and applies neural learning after world execution. The renderer
consumes tick-bound positions, births, and retirement events. It cannot change
world truth.

The player surfaces include camera/follow, creature selection and naming, care
tools, speech, needs/inspector views, pause, speed, save, and load. Load replaces
the runtime and reconstructs the scene. Save can include the real GPU checkpoint;
exact restoration is distinct from genetic rebuild or a learned-founder clone.

World/runtime lifecycle and archive hooks exist for reproduction, development,
ageing, and death. The complete autonomous family lifecycle, retained learning,
and visible save/load continuity still need observation through the current
application on supported hardware.

## Cognition and training

- Signed action and speech readouts survive foundation compilation and offline
  training. Recurrent routes retain their inherited sign restrictions.
- Play and Grab use distinct decoder families while retaining physical feature
  lanes. Older learned Play coordinates and replay have different semantics.
- Waking Oja normalization follows local third-factor credit times credited
  eligibility; zero local credit leaves fast weights unchanged.
- Terrain vision gates object chemistry by contact. Noncontact Avoid uses spatial
  bearing under danger urgency; this alone does not establish learned hazard
  recognition or avoidance under combined drives.
- The runtime coordinates episodic memory, topology, sleep, and individual neural
  state. Batched training captures private organism rows and streams cohort replay.
- Language teaching uses spatial world speech and perception. A local SLM can
  optionally translate player/creature speech. Its developmental-prior service is
  available to offline training, but is not wired into live gameplay cognition.
- Offline native CPU training and native GPU training preserve actor/value
  weights, moments, and integer optimizer ages. A successful optimizer update or
  checkpoint handoff does not establish learned-care competence.

The foundation trainer and production organism loop are the execution paths.
The former synthetic evolution, active-battery, and Era 1 runners are retired.

## Known limits and unmeasured behavior

- Current-source rendered controls, complete lifecycle, and fresh-process
  restoration need hardware observation; source integration is not a playtest.
- Dynamic overlays and some VFX triggers still derive from the launch snapshot.
  Sleep feedback follows the living individual's authoritative sleep phase and
  displayed pose, clears on waking/removal/load, and pauses with playback.
- Creature LOD remains selected by graphics profile rather than camera distance.
- Production admission supports N512, N1024, and N2048. Fresh games use the
  built-in Nano512 founder. N2048 care training remains experimental; N4096 is a
  research class. Enum values and population caps do not establish hardware
  capacity or useful behavior.
- The user target of fifty creatures at 20 FPS on i7-3770K / 32 GB /
  RTX 3050 8 GB at 1080p remains unmeasured for the current source.

This is a research alpha. A public release claim needs a working packaged game,
observed care and learning, retained individual state, practical performance,
and the user's release decision.

## Historical experiments

EI0 measured bounded lifecycle/archive/save behavior and a GPU challenge battery
on its original source. EI1 recorded 2,640 descendant receipts and 52,800 GPU
dispatches but failed its learning comparison. These historical results do not
certify current gameplay or impose promotion gates on ordinary fixes. The exact
reports, blockers, hardware, and source revisions remain in
[Evidence](EVIDENCE.md).
