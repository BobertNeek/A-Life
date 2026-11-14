# V6: production edition and per-run evidence protection

Narrow follow-up against `21b2700d79dd854bf6663d5cd8c06cba87012aa8`.

The native terminal reader now uses nested conditionals instead of a Rust 2024 let-chain. Production uses edition 2021; the replacement retains the same successful-preparation and failed-contact evidence semantics. The previous component qualification used edition 2024 and did not establish production-edition compatibility.

Every newly claimed LOCAL evidence run now creates and syncs its own `.gitignore` containing `*` before returning to any raw evidence writer. An existing outer ignore file is preserved but is no longer trusted to cover the run. A failed protection write fails admission, leaves the directory claimed, and does not silently retry. This is accidental-publication protection, not permission to upload evidence. No path spelling, absolute-path, Git-ancestor or preexisting-root restrictions were added. Existing source, previous evidence and outer ignore contents remain untouched.

Validation: all 21 combined native-reader/context/head/schema/admission component tests and strict Clippy passed with `--edition=2021`; all 4 evidence tests and strict Clippy passed with `--edition=2021`. The new regression starts with a non-covering outer ignore and checks the run-local ignore, create-only rejection, and unchanged outer ignore, source and prior checkpoint. Tests use synthetic marker bytes, not a new creature life. V1 through V5 remain immutable. No Cargo or World runs. Full-host compilation, full cold restoration, replay compatibility and learned behavioral qualification remain separate remote gates; this patch makes no new behavioral claim.
