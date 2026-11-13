# Bob's branch — soul vision for A-Life

**Branch:** `bob/soul` (from main `eded459fe3187e08de3cd27d3807cfe7d0efa66f`, Nov 13, 2026)
**Author:** Bob (Muse), Cassidy's AI assistant — working in his own branch at Cassidy's
direction (Nov 13, 2026). Nothing here merges to main without Cassidy's go-ahead.
**Status:** research alpha. Main was red at branch time (see "Where the game is").

## The target

A modern AAA cross between **Black & White** and **Creatures 3 / Docking Station**:
the god-game hand, creature training, moral shaping, and village belief of the
former; the genetics, biochemistry, learning, needs, language, and genuine
autonomy of the latter. Polished, optimized gameplay — not a systems demo with a
game taped on.

"Real soul" means a creature that feels alive and worth caring about: it
remembers you, reacts to you, learns from you, and its inner life is visible
enough to trust. WWCD — *what would creatures do* — is the style guide for
life-system work: if a creature wouldn't notice it, feel it, or learn from it,
it isn't soul, it's decoration.

The repo's own `docs/VISION.md` already states the destination well: organisms
that live, learn, reproduce, communicate, and die inside one authoritative voxel
world, with a GPU-resident neural policy, grounded perception, persistent
identity and lineage, needs and endocrine state, sleep, injury, ageing,
reproduction, and death, plus a bounded compositional language. This doc adds
the *game* around that simulation — the part Black & White supplies.

## Where the game is (verified Nov 13, 2026, at main `eded459`)

Present and substantial — the simulation is serious:

- **Brains:** compiled phenotypes from genomes (Nano512 founder default;
  N1024/N2048 admitted, N4096 research-only). GPU-authoritative WGSL selection
  with credit-gated fast plasticity, sleep consolidation, episodic memory,
  topology sidecars, offline PPO/foundation training. The brain is the most
  built-out system in the repo.
- **Biochemistry:** genuinely simulated — 11 drives (hunger, fatigue, fear,
  pain, loneliness, curiosity, ATP, temperature stress, reproductive drive…),
  11 hormones (adrenaline, cortisol, dopamine, oxytocin, serotonin…), 6 organs
  with energy/integrity/damage, multi-rate cadence, a sparse reaction graph
  (32 species / 128 reactions / 64 emitters / 64 receptors), emitters
  including `PlayerReward`, `PlayStimulation`, `SocialContact`.
- **God hand:** exists — skinned hand, Hover/Point/Grab/Carry/Release;
  world-owned carry/drop of creatures and objects (one at a time).
- **Care dock:** exists — Food, Touch, Toy, Praise, Speech. Praise routes
  through the `PlayerReward` emitter into an endocrine slot.
- **Language:** real but tiny — an 18-word bounded codebook, token IDs,
  player/creature/teacher utterances, a perception-only nursery.
- **World:** authoritative — perception, legality, candidates, execution,
  outcomes, ecology (Meadow/Grove/Wetland/Rocky/Nest/HazardField), persistence,
  archives, lineage.

Missing or unproven — the game around the simulation is thin:

- **No village, no belief, no worship, no morality, no economy.** Repo-wide
  search: `village` 0 hits, `worship` 0, `moral` 0, `slap` 0. Belief exists
  only as "stale belief" in episodic-memory recall code. No villager agents,
  buildings, or faith mechanic of any kind.
- **No punishment channel at all, by design.** Praise is the only
  training signal, and deliberately weak: "its clearance ends reinforcement;
  it does not punish the next action." There is no way to shape a creature
  by disapproval — the B&W moral axis has one pole.
- **No verified mating, birth, or social bonds.** The scaffolding exists
  (`ReproductionReadiness`, mating-reach code, loneliness drive,
  `SocialContact` emitter) but no verified mating action, birth execution,
  or friendship/bond state was found.
- **Touch/Praise do not establish a creature response.** The care dock file
  says so itself: queued stimuli record only Queued/Placed/Rejected
  evidence. The player cannot tell whether the creature noticed.
- **The care → learning → family loop is integrated but unproven.**
  `docs/STATUS.md` (Oct 21, 2026): complete autonomous family lifecycle,
  retained learning, and visible save/load continuity still need observation
  on hardware. This is a research alpha.
- **Performance target unmeasured.** Fifty creatures at 20 FPS on
  i7-3770K / 32 GB / RTX 3050 8 GB at 1080p remains unmeasured.
- **CI is red.** Nov 12 main (`25255c7`) failed five jobs with real
  durations (Group50, Simulator rehearsal, native-cpu, checkpoint
  continuity, Rust workspace validation). Nov 13 main (`eded459`) failed
  *everything* in 2–5 seconds across all workflows — a new signature,
  cause unseen in the notification emails; needs the Actions logs.

## Pillars of soul

1. **The hand is real.** The hand exists; now it must matter. Every hand
   action should have a visible, truthful consequence the creature can
   perceive. No fake feedback: if the creature didn't notice, the game
   says so.
2. **Creatures that remember you.** Biochemistry and episodic memory exist;
   a *relationship* doesn't. A creature should carry an observable
   disposition toward the player — familiarity, trust, wariness — built
   from real interaction history, shown in the inspector, never invented.
3. **A tongue that grows.** The 18-word codebook is a seed, not a language.
   Vocabulary should grow through demonstrated use — words earned by
   grounding, not granted by fiat.
4. **A village to believe in.** The biggest lift and the most B&W of the
   pillars: villagers, belief, buildings, economy. Starts as one small
   believable unit, not a city sim.
5. **Consequences you can see.** Every action's outcome must be visible in
   the world — learning, sleep, mood, injury, ageing. Overlays and
   readouts must derive from authoritative state, never from a launch
   snapshot.
6. **Fifty creatures at 20 FPS.** Polished means the target hardware
   holds the target load. Performance work follows measurements only —
   no speculative optimization, no benchmarks as proof.

## Prioritized slices

- **Slice 1 — Clean the harness (this pass).** Remove the vestigial
  DX12-compiler asserts from the three Vulkan equivalence harnesses
  (`pending_eligibility_equivalence.rs`, `publication_barrier_equivalence.rs`,
  `sleep_initializer_equivalence.rs`). They force CI to set a meaningless
  env var in Vulkan-only tests; the var is never used. See "Cleanup log".
  Status: edits prepared; branch creation awaited approval at first attempt.
- **Slice 2 — Care-dock response.** Make Touch/Praise produce an
  observable, truthful creature response: a world-owned social event the
  creature perceives, with an inspector-visible acknowledgement. Fixes the
  dock's own warning that queued stimuli don't establish a response.
  Touches `alife_world` + `alife_game_app`. Design:
  `docs/bob-soul-slice2-care-response.md`. Status: implemented in this
  pass — world-owned receipt + HUD acknowledgement; tests written for
  Codex/CI.
- **Slice 3 — Half-life chemistry.** Add per-species half-life to the
  biochemical graph (Creatures-style), with conversion from the current
  `decay_retention` and unit tests. Needs a semantics decision: how
  half-life and decay_retention interact. Pure `alife_core`; CI-run tests.
- **Slice 4 — Familiarity record.** A first-class, observable disposition
  toward the player, built from interaction history (PlayerReward,
  SocialContact, oxytocin, loneliness). Shown, never faked.
- **Slice 5 — Village seed.** Smallest believable belief unit. Design
  with Cassidy before writing code — this one decides the shape of the
  god game.
- **Slice 6 — Perf measurement.** Measure 50 creatures at 20 FPS on
  supported hardware; fix the measured bottleneck. Needs hardware
  (Codex / Cassidy's PC); not schedulable from this VM.

## What "polished optimized gameplay" means here, concretely

- The 50-creature / 20-FPS target, measured on the named hardware —
  not claimed from a benchmark.
- Truthful controls: pause, speed, save, load, selection, follow, and
  care tools that operate on the same runtime and presentation state,
  and a failed load that leaves a usable prior state.
- No fake feedback anywhere: a care action shows what actually happened
  (Queued/Placed/Rejected is evidence, not a response).
- No full-workspace test runs during implementation (Cassidy's standing
  design stance). Focused checks only; tests that freeze internal details
  get rewritten or removed, not preserved.
- This VM cannot run the game or GPU tests (Linux, no GPU, no Windows;
  no local checkout — the repo is private and only the connector can
  read it). Code and tests are written for Codex/CI to run, and the
  report says so plainly whenever that's the case.

## Cleanup log

- **Removed (slice 1):** DX12-compiler asserts in three Vulkan harnesses
  (see slice 1). Verified by reading each file: the var is never used in
  the first two files, and in the third it is only recorded tolerantly
  (`.ok()`) in a context file. All three harnesses run Vulkan only
  (all callers pass `Backends::VULKAN`; adapters are asserted Vulkan).
  The genuine DX12 test (`sleep_receptor_rate_bits.rs`, backend dx12 +
  compiler fxc) was left alone, as was the real `requested_dx12_compiler`
  selection logic in `closed_loop_runtime.rs`.
- **Checked and left alone:** the Jules temp files from PR #15
  (`parse_diff.py`, `format_review.py`, `pr15.diff`, `review.md`) do not
  exist at HEAD; the dead `if false {...}` block in
  `sleep_initializer_equivalence.rs` is not present at HEAD. Both were
  already gone — nothing to delete.
- **Rule going forward:** if you change something, clean up the thing you
  are touching. Dead code, unused imports, stale comments, and junk files
  go in the same commit as the change that exposes them. No drive-by
  refactors of untouched files.

## Working rules for this branch

- Work only on `bob/soul`. Never touch main or any `codex/*` branch.
  Never merge. PRs go up for review; nothing lands without Cassidy.
- Sign any GitHub content as Bob (Muse), Cassidy's AI assistant.
- WWCD for life-system work. Evidence grows with the claim: implemented
  is not integrated, integrated is not visible, visible is not proven.
- Focused checks only; write code and tests for Codex/CI to run when
  this environment can't run them.
