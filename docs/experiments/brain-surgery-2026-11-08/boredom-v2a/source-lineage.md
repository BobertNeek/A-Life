# Exact source lineage and common care coefficients

The immediate source baseline is NOT the accepted historical whole-v6/v12
operating point. Every matrix arm starts from
`drive_balance_candidate::construct_with_reference(true)`.

1. `world_swap_legacy_v12::construct()` calls
   `candidate_with_readouts(2026110901, 0.25, true, 12.0, true)`.
2. That starts with `care_candidate::whole_care`, whose Approach contact
   coefficient is -2.0, then multiplies all ActionCandidate genes by 0.25.
   It retains the existing target danger/valence readers, including Ingest
   exact-family valence, at the native memory gain cap 12.0.
3. `drive_balance_candidate` copies those exact gene coordinates into the
   protected N2048DriveBalanceCandidateV1 identity. Fresh motor birth dendrite
   output gain is 0.5, versus the historical 6.0. Existing/restored dendrites and
   accepted defaults are not migrated or overwritten.
4. The `true` argument then flips the nonzero ActionCandidate reference-motor
   rows for Approach and Inspect. This is the sign recalibration identified in
   the prior drive audit after lowering birth gain. Taste and distance scales
   are both 1.0; the separate ingest-geometry modification is NOT applied.
5. Only then does this experiment add the boredom module and/or consequence
   reader. Full, no-drive, no-memory and baseline all share steps 1–4.

Common care recurrent bridges retain need→reserve 2, reserve→temporal 2,
temporal→motor 4, tonic→reserve 8 and tonic→temporal -4. Whole-care scaffolding
also retains lateral inhibition -0.03 and the source's sparse recurrent support.
The newly added boredom pathway has its separate, declared four edges.

Effective ActionCandidate gene coefficients after the 0.25 scaling and reference
recalibration are below. These are static gene values, not observed action scores.
Reference multipliers include the number of selected need motors.

| Family | Feature lane | Each signal-motor coefficient | Reference-motor coefficient |
|---|---|---:|---:|
| Ingest | chemical 15 | 3 | -15 |
| Ingest | distance 2 | -1.5 | 7.5 |
| Approach | chemical 15 | 1 | -5 |
| Approach | distance 2 | -0.5 | 2.5 |
| Approach | contact 18 | -0.5 | 2.5 |
| Rest | intrinsic basis 23 | 0.5 | -0.5 |
| Avoid (two need motors) | chemical 15 | -2.5 | 25 |
| Inspect | physical color lanes 6, 7, 8 | 0.125 each | -0.625 each |
| Contact | chemical 15 | 0.5 | -2.5 |

Inherited MemoryContext totals are Ingest input 25 = -16, input 26 = -64,
input 32 = 32; Approach and Contact input 25 = -8, input 26 = -32;
Avoid input 25 = 16, input 26 = 128. Each total is divided by the actual count
of matching rows. The new Other reader adds only input 32 = 12 and input 34 =
-16, likewise normalized. These additive memory heads do not multiply their
source neuron's activation or current need. Exact action/target routing is
provided by the ABI; learned consequence values come only from actual receipts.

The full-vs-source result therefore concerns the added boredom module at this
already recalibrated 0.5-gain operating point. It does not compare 0.5 with 6.0,
or newly qualify the whole care policy. The hungry food fixtures begin with
reachable contact food. They do not establish distance navigation. The
unreachable-toy fixtures produce zero actual Play and do not show a search or
approach solution. Low-boredom full fixtures produce no Play during 24 ticks,
with 8/6 Rest selections and boredom ending near 0.104; low exploratory drive
remains an explicit limitation, not a desired prohibition on low-need play.

See exact-ablation-diff.json for all changed static synapse coordinates. The
no-drive arm removes all four new recurrent edges AND both new contact readout
rows. It is a module ablation, not an input-only lesion. The no-memory arm removes
only the 80 new consequence-reader rows; native memory storage, novelty,
inherited readers and ordinary local plasticity remain active.
