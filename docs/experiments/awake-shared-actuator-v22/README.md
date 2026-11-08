# V22 common bounded endpoints and opt-in actuator continuation

This update fixes bounded endpoint representation for ordinary movement and the separately opt-in shared contact-reflex actuator. It does not activate reflexes in ordinary CPU/GPU gameplay, change genes or reset acquired lives. No save format is changed.

## Physical correction

Movement duration is planned before coordinates are rounded. Ordinary movement, observed-goal steering and reflex movement now use the same endpoint rule: if representation exceeds the existing guard, try at most eight next-representable coordinate steps toward the start, keeping the original speed and duration. Re-resolve terrain and retain collision, support, water, carry and grip checks. The existing tolerance is not increased. An unrepresentable route fails closed. Huge-coordinate sub-step immobility is reported rather than invented movement.

This common correction is not a reflex intervention. Genuine preemption, swept clipping, collision/carry rejection and other changed voluntary execution still remain marked as altered. Late zero-time, zero-effort, zero-displacement responses qualify for ordinary credit only when the actual voluntary execution/body event remains equivalent. The CPU research fixture proves this with sealed outcomes; this change does not wire that research host into gameplay.

## Executed evidence

- Exact source 2e154178cbdc9a18c4c1c09572327ea7c8c81032, production helper source 1faacaf698f0f50adffa39141dd2f69f5b0b2280. The six extracted world files are byte-identical to that source. Translation-budget source is unchanged from the original PR315; it is not a seventh changed world file.
- Full research world suite: 636 passed, zero failed, three pre-existing ignored cost/soak tests. Earlier library/focused counts overlap and are not added again.
- Strict world lib/tests and cpu-backend training lib/tests Clippy passed; workspace formatting and diff checks passed.
- Six actual CPU continuation fixtures passed. A separate fresh half-speed fixture passed at two paired origins, X128 and X1000: the native selected move plus Eat really consumed the meal, path was 0.049987793 with full allocated time. Calibration was explicitly prepared in fresh fixture state, not learned adaptation or a forced sampled action.
- Ordinary/shared sealed decisions and complete checkpoints matched for the unchanged/late-exhausted cases. Memory/experience and learned-weight generation advanced once; founder optimizer sentinels were preserved. This is plumbing/continuation evidence, not sustained learned competence.

The evidence directory preserves inspected source, original logs, intermediate patches, manifests and hashes. No executables, checkpoints, neural banks or retained learned state are included. The earlier focused report remains historical; FINAL-QUALIFICATION.md supersedes its pending-check status only. Exact current-main integration and hosted CI are separate qualification steps and must be reported from their actual outcomes.

## Known limits and next iteration

The curved-terrain contact diagnostic deliberately preserves a real defect: a straight chord's hazard intersection can be outside the actual grounded surface after projection, although a later grounded point truly contacts it. Snapping the chord point onto terrain would invent pain. The separately identified V23 planner will use canonical terrain-triangle segments; V22 is not that result. Factual intervention-memory attribution, ordinary gameplay reflex activation, GPU numerical behavior and sustained autonomous self-care are also unqualified.

The original method reference remains [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/): targeted repair in experiment 4, whole 20,486-weight game networks in experiment 5, iterative feedback in experiment 6. We preserve each prediction, failure and measured correction; those game results are not A-Life validation.

## Exact current-main integration check

The extracted six world files were checked byte-for-byte against V22 source 2e154178 on the current PR315/main integration. Its independent world-library run passed291 tests, zero failed, two existing ignored. This count overlaps the earlier world suite and is not added to 636. The accompanying source receipt records the pre-build tracked patch/new-helper identity and rechecks all six world blobs afterward; intervening additions were documentation/evidence only. Hosted exact-head CI is pending. Current-main persistence fixes are retained rather than replaced with older research source.
