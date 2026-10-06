# Training CLI output preservation

The cycle CLI's error wrapper attempted to write `output/failure.json` even
when creating that output directory failed. If output named an existing source
checkpoint or a foreign destination, the rejected invocation could overwrite
its prior diagnostic. The warmup wrapper had the same problem with `failure.txt`.
Native CPU rejection fixtures reproduced both overwrites using the preserved
`de38cad9` binary; source inspection confirms the same wrappers on main `9a4e3b3`.
No real checkpoint was used as a writable reproduction fixture.

Failure reporting now belongs to the cycle/warmup operation after atomic
`create_dir` succeeds. An existing destination rejects before the operation or
its failure recorder runs. There is no check-then-create ownership assumption.
Errors still reach the CLI, and a later failure in the invocation's new output
retains the cycle JSON fields or warmup text diagnostic. Training, objective,
checkpoint identity, replay and promotion rules are unchanged.

Four CPU-only ownership tests cover an aliased source/output, an existing foreign
destination, a diagnostic after failure in a newly created output, and successful
completion without a failure diagnostic. They compare all sentinel file bytes,
including preexisting diagnostics. Native CLI verification additionally records
SHA256 inventories before and after each disposable fixture.

The feature-gated CLI builds offline. Six native CPU checks cover both modes:
existing source/output and foreign destinations retain identical SHA256 file
inventories, while new outputs retain nonempty diagnostics. Cycle JSON also
retains its seed and requested-decision fields. Formatting, core boundaries and
all 77 documentation assertions pass. All four ownership tests also pass through
the feature-enabled library harness and are registered in their own CI job.

This report does not establish GPU, learned behavior or continuation acceptance.
The separate objective/sensory continuation work and its owner remain untouched.
