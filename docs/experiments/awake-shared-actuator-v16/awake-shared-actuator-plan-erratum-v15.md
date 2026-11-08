# Source interpretation correction

The first source-plan message incorrectly said the hazard profile replaced the body event with damage only. That interpretation was retracted before implementation. `OutcomeProfile::with_body_event` explicitly carries forward `self.body_event.energy`; ordinary hazard therefore retains -.08 energy while adding physical damage. Quiet movement is -.04 body energy and .08 physical outcome cost; body-only reflex cost is .02*capacity. These fields were not assumed to share a unit or silently unified.

The original source-plan hash5dc1887ad7b5e80aee49b82d45b6ea9f728a8f5df1e1db8124015a539b2c1003 contained the mistaken statement; the corrected source-plan hash2b1c7292822a638832670f0259c68eb8833eac988a3858a41b0f082273f20b43 removes it. The correction was sent to the parent immediately after inspecting `with_body_event`.

V15 initial-contact candidate preserves existing effort constants and preempts voluntary translation. V16 explicitly scales the existing voluntary effort fields by actual allocated time only when a swept contact splits the motion; injury dose is unscaled, and remaining-time reflex effort uses the existing .02*capacity rate. Quiet/default/sleeping behavior remains the control. This is a distinct candidate accounting rule, not retroactive interpretation of the old implementation.
