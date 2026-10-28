# Explicit counterfactual training attempts

A continuous training death retires normal continuation, even when the last
retained checkpoint predates that death. To try again from that acquired
baseline, create an explicitly labeled **new training attempt**:

```text
export_native_training_handoff --inspect-current TERMINAL_CURRENT
export_native_training_handoff --begin-training-attempt TERMINAL_CURRENT NEXT_NONCE NEW_FILE
export_native_training_handoff --inspect-current NEW_FILE
```

The output must be a new file. The source remains terminal and unchanged.
The operation preserves the full handoff, actor/value Adam moments and ages,
compiler inputs, exact private world/biology/recurrent/lifetime state, sampler,
world seed, saved tick, prior same-individual counters and seed cursor. It does
not train, advance time, reserve a nonce, consume a nonce, restore a dead body,
or credit unsaved updates. It requires one living **preterminal saved snapshot**.
This is a counterfactual branch from that snapshot; the original trajectory died.

The caller selects `NEXT_NONCE` from the admitted pending tail, or a fresh unused
nonce when no tail remains, respecting the recorded historical-freshness limit.
`training_attempt_origin` binds that intended first dispatch to the original
Current digest, full terminal disposition/event, saved world/individual identity,
head/private hashes and optimizer ages. Prior compact origins persist. No prior
brain or private-state archive is embedded. The usual native WAL consumes the
nonce only at real dispatch; a different first nonce is rejected before native
initialization. Later same-individual checkpoints and recovery retain the origin.

The external owner validates and atomically publishes `NEW_FILE` through the
existing single-current publisher. No new request or gameplay flag is needed;
the ordinary `--resident-continuous STORE REQUEST` path restores the explicitly
branched private world. Fresh-cohort/serial paths reject this branch rather than
silently replacing its private life. A later death again retires continuation
and preserves this attempt's origin.

These source/CPU contracts do not demonstrate successful GPU restore, improved
care, terminal-transition credit, or retained training from an unsaved window.
