# Sealed experience version consistency

The directional-withdrawal research exposed a pre-existing validation gap in ordinary V11/V12 experience patches. A context-bearing patch could relabel only its header to the older ABI, remove the two outer optional context fields, retain newer nested evidence, and select the older validation/digest branch. This is malformed evidence, not a valid legacy save or a creature learning outcome.

A fresh current-main regression reproduced the gap: the intended rejection failed for a downgraded header. The narrow repair moves the existing header/pre-action/decision/outcome ABI-equality check before the header-dependent branch. No wire field, version constant, valid digest, learned record, weight, optimizer, world behavior or default neural recipe changes. Valid old formats continue through the existing explicit migration path.

The regression checks V11 and V12, each of the four version fields, and rejection by validation, causal digest and packing. Unchanged valid JSON bytes/digests and existing literal legacy migration fixtures remain controls. The research V13 reflex is not included in this patch.

This is a preservation correction arising from the iterative edit–measure–revise method, not brain-policy efficacy. Main methodological reference: [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), experiments 4–6.

## Local qualification

Rust 1.99.1, offline, a fresh worktree and a new isolated target directory: 81 tests passed across core library, three-phase experience, neural decision, packing, scaffold and V11/V12 contracts. One existing library test remained ignored. Strict core all-target Clippy, workspace formatting and diff whitespace checks passed. The prerepair rejection fixture failed as intended; the repaired fixture and existing legacy migration/round-trip controls passed. Hosted exact-head integration remains pending.
