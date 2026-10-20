# Founder sensing, readout signs, and newborn pain corrections — 2026-10-20

This implementation report starts from main
`bda3b65be5fa6f01865b850640adf2e8d01f4990`. It does not amend the controlling
architecture or qualify a founder. Tests run in the saved Linux cloud workspace,
without the user's PC, training episodes, seed allocation, or physical GPU work.

## Reproduction and corrections

`founder_brain_regressions` exercises real world observation extraction,
calibration, candidate enumeration, compiled founder neurons/readouts, and world
command execution. Its test-only numerical harness uses fresh zero mutable
banks, neutral receptor gains, the compiled encoder, recurrent cadence, bias,
leak, tanh and metabolic feedback equations, and all recurrent routes enabled.
There are no structural branches or memory contributions in these fixtures.
These are CPU source/arithmetic and physical-world tests, not GPU acceptance.

- **Distant avoidance:** production `GroundedTerrainVisionV1` contact calibration
  zeros source-specific chemistry and temperature. The old Avoid prior read
  negative chemistry anyway, giving zero distant prior. It now falls back to
  observed bearing magnitude before contact under fear, pain, or temperature
  urgency; at contact it retains the original chemical cue. For seed 96001,
  pain .98 and a visible object four units away, the prior changes from 0 to
  3.071232. Decoded Avoid beats Approach and the executed retreat increases
  separation; hunger reverses the decoded preference and approach reduces
  separation. Hidden source valence cannot change the candidate features.
  Contact still gates chemistry, temperature, and taste-supported ingestion.
  This is generic retreat under danger, not remote hazard recognition.
- **Distance:** seed 41 N512 motor activations are .016131–.033836 under the
  fixture. The old Approach score rises from .075800 to .078528 as normalized
  distance rises .25 to .5: derivative +.010914. Fresh procedural lane-2 readout
  coefficients now use the negative of the same generated magnitude; corrected
  scores are .070343 and .067614, derivative −.010914. N512/N1024 measured motors
  are positive; N2048 also has some negative motors. The tested weighted
  Approach/Avoid/Grab distance derivatives are negative for three founder seeds
  in each class, including N2048. Tests compare the analytic sum of
  `motor * weight` with a finite difference and trace `motor * distance` weight
  derivatives. A separately re-extracted near snapshot and recurrent evaluation
  also prefer proximity after upstream sensory changes. Positive recurrent
  MotorProposal/Homeostatic weights and the
  +.05 motor bias remain intact.
- **Readout identity:** Play formerly overwrote angular-speed lane 20 with 1.
  At angular speed 1 it has the same Contact family and features as Grab, so
  no learned weight can distinguish them. Play now uses the existing Other
  family; Grab retains Contact. All 24 features remain physical, with no action
  flag replacing proprioception. Angular speeds 0, .5 and 1 are covered. The
  stationary seed-96001 fixture decodes Play .113169 and Grab .067798.
- **Foundation and optimizer signs:** procedural initialization is distinct from
  foundation replacement. Negative mathematical decoder coefficients now survive
  replacement and phenotype validation. The full trainer uploads Adam sign
  policy 0 for every Decoder coordinate; only Recurrent LateralInhibition uses
  policy 1 and Recurrent Homeostatic/MotorProposal policy 2. The separate Nano
  readout fitter and founder branch merger also permit signed decoder weights.
  No recurrent sign rule is relaxed. Exact candidate regressions retain a
  −.375 distance coefficient, preserve every other supplied weight bit, and
  reject negative recurrent excitation. SpeechPayload is also a mathematical
  readout on MotorProposal: signed speech coefficients are covered by exact
  foundation admission and Adam metadata, not constrained as recurrent excitation.
- **Ordinary Nano newborn pain:** ordinary New Game omitted the damage-emitter
  expression floor already applied to experimental Nano and Standard founders.
  Real damage .15 previously caused injury .040590316 with pain 0. Applying the
  same inherited floor to all New Game founders gives pain .120000005 while
  biochemical developmental expression remains 0; additional nutrition does
  not mature its chemistry. A real ordinary-world collision gives damage .12
  and pain .096. This single collision does not cross the existing .55 emergency
  threshold. Quiet newborns remain pain-free. Stored organisms are not
  reinitialized.

## Compatibility and limits

The grounding layout is `[bearing sine, bearing cosine, distance, ...]`:
distance is lane 2, not lane 1. N2048 has 16 candidate motors per family and
24 candidate features, giving 3,072 ActionCandidate and 1,024 SpeechPayload
synapses. The synthetic curriculum really clamps selected-family encoded inputs
to +.75 in movement, survival, and speech mechanics stages, and supervises the
selected family's 16 motor targets to +.75. Its 24 synthetic candidate features
share `action_sign`, with ±.8 candidate targets. This is a positive synthetic
supervision bias, not evidence that all live motors are positive or that family
identity is lost. Other content/speech targets can be signed. This correction
does not rewrite the curriculum or overwrite learned coefficients; signed
per-family and speech readouts remain available to learning.

No save, foundation, perception, decoder-width, address-map, or optimizer schema
is changed. Raw distance remains lane 2; physical features remain in their
original lanes. Cue code 24 (uploaded as 25) retains its original Approach-only
meaning: `(1-contact) * clamp(abs(bearing_sin)+abs(bearing_cos),0,1)`. Avoid
retains its original bound metadata: chemical lane 15, inversion enabled, no
reach requirement. Only when contact lane 18 equals zero does this exact Avoid
configuration use bearing magnitude instead of unavailable chemical valence.
The ordinary WGSL and core arithmetic agree. This preserves foundation decoder
hashes; the actual retained Nano package, which failed admission with the first
cue-plan proposal, is regression-tested against the final implementation. The legacy object-slot profile's
unrestricted chemistry is retained; the corrected non-leaking reproduction uses
the production terrain-vision profile.

Accepted foundation assets, checkpoint backups, learned matrices, and Adam
moments/ages are not rewritten or reset. Foundation replacement deliberately
preserves a trained distance coefficient of either sign; this is not a blanket
inversion of learned policies. Fresh procedural initialization alone guarantees
the tested new sign disposition. Existing trained policies may retain distance
bias and require further learning or separately qualified replacement.

Play's move to Other changes its readout coordinates and memory-family context.
Older Play learning does not automatically transfer from Contact. Old Play
replay frames carrying the overwritten physical lane are not silently converted.
Foundation prior metadata and its bound hashes are preserved. Fresh procedural
phenotype hashes change with their distance coefficients; no automatic migration
of old exact procedural runtime snapshots is supplied. Historical source-bound
evidence remains evidence for its original source, and old policies are not
qualified under the changed Play and non-contact Avoid semantics. Continuing a
source-bound experiment requires its normal source admission and behavioral
requalification, preserving its complete accepted checkpoint and optimizer
history.

The concrete Other-family opportunities in the current world enumerator are
targetless Vocalize and no-posture Hold, plus object Play after this correction.
Vocalize has intrinsic lane 22 and basis lane 23; Hold has lane 21 and basis lane
23. Both have zero bearing. Play retains the observed object's nonzero bearing
and all physical descriptors. Gesture and Write are also ABI-compatible with
Other but are not generated as object opportunities here. A future additional
object operation sharing Other would need its own distinguishable encoding.
The separate N2048 speech-payload head has a distinct decoder-head tag and is
executed only for a selected Vocalize kind, not for arbitrary Other candidates.

Motor channel routing uses ActionKind: Interact/Write are manipulation,
Vocalize is vocal, and Hold/Rest/Inspect are posture. Tests select simultaneous
Other-family Play, Vocalize and Hold through the real factorized bundle builder,
validate the joint receipt, round-trip Play's decision family, and execute a
real manipulation activation with Completed phase and positive play stimulation.
Memory queries retain Other plus the actual Play ActionId; Other's existing
memory index additionally partitions by ActionId. Historical Contact records
retain Contact and are not relabelled to Play's new family.

The vocabulary teacher and both action evaluators map token 13 to Other while
token 9 stays Contact. The teacher still approaches before contact and chooses
the explicit Play primitive at contact; a CPU fixture covers these choices and
the replay candidate's family/physical-feature round-trip. Grab demonstrations,
reproduction contact checks, and social-recognition Contact criteria retain
their existing acquisition/contact meanings. No family is used to route Play
into vocal output or posture. These software tests do not qualify on-policy GPU
activation or transfer of old Play learning.

The relevant architecture requirements are `AOA-INV-001`, `AOA-SENSE-001`,
`AOA-WORLD-001/003`, `AOA-DEV-001/003`, `AOA-BIO-006/007/015`,
`AOA-PRED-004`, `AOA-MOTOR-002/005`, and `AOA-OBS-002/003`. The tests establish
these narrow implementation corrections; long-term learning and physical GPU
behavior remain unqualified here.

## Validation

Focused reproduction, compatibility, aggregate Rust, static boundary, and
documentation checks are recorded with the PR. Physical GPU behavior is Unknown.
