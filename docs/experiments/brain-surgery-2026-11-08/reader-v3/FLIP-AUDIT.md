# All nine low-gain instantaneous bundle changes

No shadow command was executed. “Potentially useful,” “neutral-looking,” and “risk” below describe hypotheses from the observed scores/targets and actual OFF receipts, not measured alternate outcomes. None is an established behavioral benefit. Full logits/probabilities and selected candidate indices are in `nine-flip-audit.json`.

| Seed / step | Actual OFF bundle → gain1/16 shadow bundle | Evidence and conservative interpretation |
|---|---|---|
|2401 /41|NO_LOCOMOTION → H/Approach + NO_MANIPULATION|All selected candidates have port53=0 and identical logits. Normalization/representative arbitration adds H-directed movement. Ambiguous handling/energy cost, not H avoidance.|
|2401 /125|H/Approach + Idle → H/Approach + H/Play|H is already held. No selected candidate has a nonzero port. The source completes Play only for Ball/ActivityToy; H here is Food. Handling/action-validity risk from adding an unsupported play attempt. No shadow failure was executed.|
|2401 /206|H/Approach + H/Close → H/Approach + H/Play|Close logit .513564→.382256, input53=-.262616. OFF actually acquired H. Ambiguous handling substitution: Play calls hands-on preparation before checking the final play affordance, so selecting Play can still prepare/acquire an object. Do not count removal of Close as proof of avoiding pickup.|
|2401 /244|H/Approach + H/Play → H/Approach + H/Release|H is held; OFF Play actually failed without transfer. Release logit -.220071→-.107022. Potentially useful replacement of a failed action/freeing hands, but no safe Eat candidate is present and a carrying goal is unknown. Premature-release risk remains; benefit unmeasured.|
|2402 /55|H/Approach + Idle → H/Approach + Rest|Selected logits unchanged, both selected non-operation candidates port0. Neutral-looking posture redistribution, but Rest can matter physiologically, so not proven neutral.|
|2402 /98|H/Approach + H/Close → H/Approach|Close .402034→.289607. OFF actually placed S then acquired H. S/Eat is available with unchanged logit4.382791. This is the clearest potentially feeding-supporting direction: omission of the actual S-to-H swap. The shadow does not itself select S/Eat; retained S and later feeding were not measured.|
|2402 /107|H/Approach + H/Close → H/Approach|Close .414685→.295994. OFF actually acquired H. No S/Eat candidate is present. Could reduce unnecessary handling or lose useful acquisition; classify as unresolved handling risk, not a safety success.|
|2402 /148|NO_LOCOMOTION → H/Approach + NO_MANIPULATION|Same zero-port normalization/arbitration pattern as2401/41. Ambiguous added H movement, not demonstrated benefit.|
|2402 /174|H/Approach + Rest → H/Approach + Idle|Selected logits unchanged, both port0. Neutral-looking posture redistribution with possible rest opportunity cost. No alternative physiological outcome measured.|

Summary: three changes remove the explicit H/Close primitive, one adds H/Release, and five change only zero-port representatives. None selects Eat(S). Only the step98 frame even contains an S/Eat candidate among these nine. Its representative probability changes .3383604→.3385908 through normalization, but it is not selected in that same draw.

Correct IDs from frozen source:
-1=Idle,3=Rest.
-101=Approach,102=Flee,103=NO_LOCOMOTION,104=NO_POSTURE.
-210=Eat,211=legacy Grab,212=NO_MANIPULATION,213=Play.
-2201=explicit Close,2202=explicit Release.

Earlier prose describing212 as Play was inaccurate. The frozen historical files are unchanged; this appendix corrects the interpretation. `normalization-example.json` preserves the first exact counterexample.
