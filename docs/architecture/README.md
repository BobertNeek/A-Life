# A-Life architecture authority

The single normative architecture is:

`ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`

It supersedes every earlier A-Life architecture specification. Earlier documents remain historical design lineage only and cannot narrow, amend, or override v2.0.

The normative categories remain exactly:

1. LOCKED GOAL
2. LOCKED CAPABILITY
3. LOCKED INVARIANT
4. LOCKED INTERFACE
5. REFERENCE MECHANISM
6. TUNABLE DEFAULT
7. DEFERRED CAPABILITY
8. RESEARCH

The controlling document defines conflict precedence among those categories. Do not infer architecture from current Rust types, engines, processor placement, GPU layouts, brain-size classes, constants, adapters, tests, fixtures, or other implementation choices unless a v2.0 requirement explicitly locks the relevant semantics.

## Package contents

- `ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md` is the canonical source.
- The matching DOCX and PDF are publication copies.
- `requirement_registry.csv` contains all 365 stable `AOA-*` requirements.
- `compliance_matrix_template.csv` and `compliance_report_template.md` are blank report templates.
- `diagrams/` contains the DOT, SVG, and PNG forms of all eight architecture diagrams.
- `verification/` records publication checks. It is not codebase compliance.
- `SHA256SUMS` records the adopted files after promotion.

## Compliance and implementation status

Never add current codebase pass/fail status to the controlling architecture. Record compliance in a dated report that cites the relevant `AOA-*` IDs, repository commit, evidence, and uncertainty.

The current implementation maps in `../ARCHITECTURE.md` and `../REFERENCE.md` are non-normative derived documents. They can describe what exists, but they cannot define the target.

## Historical lineage

The v1.1 brain specification, its compliance matrix, and its recovery plan remain in the repository with explicit supersession notices. They may explain past decisions and evidence, but v2.0 controls every conflict.
