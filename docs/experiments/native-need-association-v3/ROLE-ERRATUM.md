# Additive drive-role clarification

The frozen V2 and V3 comparison intentionally has seven nonzero magnitudes and
hardcoded pressure operators for drive lanes 0–4, 6 and 7. Lanes 5, 8, 9 and 10
are forced to zero by both its recipe validator and pressure compiler. Its fixed
`reference-species22-drive-roles-atp-availability.v1` label is not a validation of
the actual phenotype's species-to-drive mapping.

In the accepted phase-3 phenotype, drive lane 9 is Extension0, bound to species 22
boredom. It is a real need: Awake emits +0.0002, PlayStimulation emits −0.2, its
receptor gain is +1 and biological value coefficient is −0.08. Praise/reward is
species 21 projected to endocrine Extension0 (hormone lane 9), a different ABI
array. Calling the observed drive-lane-9 increase reward was incorrect. Curiosity
is the separate drive lane 5/species 6 and has biological value coefficient zero.

The V2 raw drive intervals changed curiosity and boredom, while all seven axes
valued by the frozen reader remained exactly unchanged. Ignoring observed boredom
is an architectural omission of this reader, not evidence that boredom is an
informational or reward channel. These facts do not change the sealed V2 result,
V3 genes or V3 admission protocol.

A later version must validate or derive role bindings from the actual phenotype,
change the pressure operators and validator together, bind the mapping into its
identity, and test positive goal/regulatory/unbound roles separately from signed
need pressure. Adding a nonzero coefficient alone would not fix the hardcoded
zero pressure. It also needs genuine Play source admission: current observed
records admit Inspect only, so a weighted boredom lane alone cannot establish
learned toy utility. Native V3 remains an admission/recall check, not that broader
behavior test.

Development also matters. Newborn concentration paths can be expression-gated,
while explicit curiosity/boredom emitters use an expression floor of one. No
pressure change in this fresh context is not by itself broken world chemistry.
Any future preparation, action expansion or developmental context needs its own
prospectively frozen source, protocol, controls and honest raw consequences.
