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
  negative chemistry anyway, giving zero distant prior. Now it uses observed
  bearing magnitude under fear, pain, or temperature urgency. For seed 96001,
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
  derivatives. Positive recurrent MotorProposal/Homeostatic weights and the
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
  reject negative recurrent excitation.
- **Ordinary Nano newborn pain:** ordinary New Game omitted the damage-emitter
  expression floor already applied to experimental Nano and Standard founders.
  Real damage .15 previously caused injury .040590316 with pain 0. Applying the
  same inherited floor to all New Game founders gives pain .120000005 while
  biochemical developmental expression remains 0; additional nutrition does
  not mature its chemistry. A real ordinary-world collision gives damage .12
  and pain .096. Quiet newborns remain pain-free. Stored organisms are not
  reinitialized.

## Compatibility and limits

No save, foundation, perception, decoder-width, address-map, or optimizer schema
is changed. Raw distance remains lane 2; physical features remain in their
original lanes. Cue code 24 (uploaded as 25) retains its Approach meaning:
`(1-contact) * clamp(abs(bearing_sin)+abs(bearing_cos),0,1)`. Avoid additionally
accepts this code and uses the bearing factor without the contact multiplier.
The ordinary WGSL and core validation agree. The legacy object-slot profile's
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
Compiler-produced prior/phenotype hashes can change; historical source-bound
evidence remains evidence for its original source. This patch supplies no
automatic migration of old exact runtime snapshots or qualification of old
policies under the new semantics. Continuing a source-bound experiment requires
its normal source admission and behavioral requalification, preserving its
complete accepted checkpoint and optimizer history.

The relevant architecture requirements are `AOA-INV-001`, `AOA-SENSE-001`,
`AOA-WORLD-001/003`, `AOA-DEV-001/003`, `AOA-BIO-006/007/015`,
`AOA-PRED-004`, `AOA-MOTOR-002/005`, and `AOA-OBS-002/003`. The tests establish
these narrow implementation corrections; long-term learning and physical GPU
behavior remain unqualified here.

## Validation

Focused reproduction, compatibility, aggregate Rust, static boundary, and
documentation checks are recorded with the PR. Physical GPU behavior is Unknown.
