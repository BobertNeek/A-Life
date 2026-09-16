# Legacy P25 GPU diagnostic archive

This directory preserves the retired P25 static-forward parity implementation,
WGSL shader, timing report, and test source. The snapshot came from commit
`130a0f108f1f5dc4a4388496f08d4c76e95c5615`.

P25 was an early three-pass diagnostic with synchronous activation readback and
a CPU parity helper. It did not run the complete causal neural transaction and
never owned production brain authority. The production runtime now uses the
closed-loop GPU pipeline and bounded selection and learning receipts.

Files here are not Cargo targets, production bundle assets, shader ABI entries,
or performance gates. If a historical assertion is still useful, port it to
the current closed-loop runtime instead of restoring the P25 path.
