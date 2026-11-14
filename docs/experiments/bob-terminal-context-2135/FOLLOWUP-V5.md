# V5: preserve enabled terminal heads across cold state

Cumulative follow-up against draft commit `a451394fcdd06aaaf61c6359728a45840451d654`. V1 through V4 packets remain immutable. V4's exact two-head asset plus construction genome, finite gain domain, LOCAL create-only retention, and typed ATP admission are unchanged.

The canonical CPU checkpoint now selects host schema 2 when the optional terminal head is absent and schema 3 when present. The optional field retains its original skip-if-None serialization, field order, and all other state. Default-off serialization is unchanged. This host version is distinct from terminal recipe schema 2.

Before restoring any World or resident state, the new loader accepts only `(host schema 2, no head)` or `(host schema 3, head present)`. It rejects missing heads, unknown versions and prototype `(2, head present)` checkpoints. Existing recipe, parent and genome validation still follows; a schema 3 envelope does not authorize an invalid recipe. Old host loaders reject schema 3 even though their permissive serde structs ignore the unfamiliar head field, preventing silent head loss.

Prototype V1 through V4 enabled checkpoints advertised schema 2. They are not automatically migrated or accepted by V5. Preserve their original bytes and original source/recipe provenance. No qualified migration is provided, and editing their version alone is not established as a valid migration. Ordinary legacy schema 2 checkpoints without a head remain supported. No old checkpoint or experimental life was rewritten.

## Local checks

21 combined context, head, ATP admission and schema component tests passed, including default-off wire projection equality, schema/head consistency, enabled recipe cold roundtrip with extreme finite gain, and permissive old-reader projection followed by the original version gate. Strict component Clippy passed. The tests deliberately use a minimal serde envelope projection, not a fabricated claim of full host cold restoration. Host source wiring was checked to use these exact selection/validation functions before restoration. No Cargo or World execution was performed.

## Remaining remote qualification

Compile the full host and run full canonical checkpoint tests: default-off exact bytes against the original host on the same prepared full-parent fixture; enabled schema 3 cold roundtrip preserving all resident, neural, lifetime, eligibility, optimizer and head state; inconsistent schema/head rejection before World restore; old executable rejection of a real enabled schema 3 checkpoint. The V4 native admission and actual informative-consumption behavioral gates remain outstanding. Do not interpret component success as demonstrated learned choice. Bank capacity greater than 64 and conservative tariff remain prototype limitations. All raw evidence stays LOCAL and outside source publication.
