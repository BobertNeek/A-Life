# Additive compile proof and report erratum

This supplements the frozen v2a boredom review without modifying it. No native
work or fixture was rerun, and no acquired memory bank was read.

The three *.compile-lines.txt files contain only exact Compiling/Finished lines
from the original logs, with one source-root prefix replacement. binding.json
records the original log hashes, compile paths, frozen source closure, prior
copy-time source/executable provenance and limits of the evidence chain.

The build log explicitly compiles core, GPU backend, world, runtime and training
from the isolated experiment source. Test-profile compiles are independently
captured in two separate logs: six crates for the boredom example and core/world
for the common Rest guards. Test executables were not separately frozen. Cargo
text does not embed a source-tree or executable digest; this is a retrospective
hash-bound provenance supplement, not an embedded compiler attestation.

Erratum: full arm, stream 29, 24-tick toy-high ends at boredom
0.05273296684026718 (0.052733 to six decimals), not 0.052666 as stated in
results.md. The public receipt and matrix analysis already carry the correct
value. No run, count or conclusion changes. Decision and outcome clocks remain
explicitly separate, as documented in v2a.

Only source hashes/metadata, copied provenance text, sanitized compile lines,
this explanation and the scalar erratum are included. No executable, checkpoint,
acquired bank, learned array or raw episode payload is included.
