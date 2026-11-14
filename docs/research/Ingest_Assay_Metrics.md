# Ingest execution metrics in evolve_pursuit

`failed_ingests` is retained unchanged for historical comparability: it counts a
selected Ingest candidate whenever the **whole joint bundle** reports failure.
It is not a count of failed Ingest executions. Another channel or an aversive
meal can make this legacy field disagree with successful consumption.

The additive `ingest_execution` object reports `successes`, `failures`,
`unknown`, and `consumed_receipts`. Each selected Ingest is matched to exactly
one Manipulation channel receipt from the sealed outcome. Typed evidence must
validate and match its selected primitive and commanded target. Missing legacy,
mismatched, invalid or ambiguous evidence counts as unknown, never as failure.
The evidence's execution success is independent of shared bundle success.
`consumed_receipts` is a subset with validated `Consumed` physical contact;
a successful `Touch`/preparation is not a completed meal. These count execution
phases, not nutrient quantity, isolated hunger relief or learned action value.

No policy, world execution, physiology, plasticity, weights or sampling changes
are made. Old result files lack the new object; do not treat absence as zeros or
retroactively relabel the historical `failed_ingests` field.
