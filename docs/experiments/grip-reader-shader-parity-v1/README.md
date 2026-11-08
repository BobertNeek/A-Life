# Protected N2048 grip-reader shader parity

This is an opt-in **synthetic, isolated numerical fixture** for the protected
N2048 episodic/grip-reader ABI. It executes the unchanged production memory and
decoder-eligibility WGSL entry points on **SwiftShader Device (Subzero), CPU
Vulkan**, and compares against `CpuOrganismState.advance` and `stage_selection`.
Selection is seeded and contextual inputs are constructed explicitly. This is
not hardware qualification, learned-behavior evidence, policy retuning, full
memory retrieval, recurrent execution, or end-to-end runtime qualification.

## Results

Each successful revision contains 15 cases covering valid Close/Release,
ordinary Other, legacy numeric action 2202, cross-profile rejection, zero
evidence, signed contextual inputs, and the new ±12 versus legacy ±0.5 caps.
Each checks 738 feature values, 122,880 decoder eligibility rows, and 61,440
memory eligibility rows. Final fixtures also check 15 host cross-profile
rejections and 11 schema3 wrong-profile shader rejections.

- Initial successful fixture: maximum absolute logit error `5.9604645e-8`;
  exact numerical feature and eligibility agreement.
- Final fixture: exact numerical agreement for logits, features, and eligibility.
  A fixed synthetic `+0.125` lifetime-weight control on lane53 rows makes Close
  and Release visibly sign-sensitive, with explicit causal assertions. No
  existing policy weights or acquired creature state were changed.

## Provenance and limits

The tested source commit is `92312cfb5a6841e28be2d54cbd550aea8fa64f39`, based on
protected revision9 `a402c1da3c7ee3334db72429230beb016b887ab0`. This documentation
commit adds only this evidence package and does not alter that tested code.

The initial successful log is preserved unchanged. Its source is explicitly
**reconstructed**, not a contemporaneously hashed source capture. The preserved
`initial-to-final-fixture.patch` reconstructs the final source exactly. The
original executable was **not separately retained**, and no original executable
hash is claimed. The final executable was retained locally and hashed; it is
not included here.

- Final fixture source SHA256:
  `704c87de286dbdb164e2276bbaae3c58ee352c3fb9db973fc8d73815177f67ca`
- Final executable SHA256:
  `675d3befe2a6c9b6b2abbab27b1830ffd396c6619afb85d123a174ca30865ebc`

All ten original package payloads, including `manifest.json` and `README.txt`,
are byte-preserved. The manifest records source, backend, executable, and log
bindings. This package contains no executable, caches, checkpoints, learned
arrays, or project assets. The fixture's arrays are fixed synthetic controls.

## Checks and opt-in execution

Recorded checks passed: 10 CPU organism tests including this fixture, 8 core
ABI tests, 3 GPU upload tests, strict training Clippy, workspace formatting,
static core boundaries, and diff checks. The full workspace suite was not run.

The new test is explicitly ignored and feature-gated under the CPU backend.
The existing fixed 18-suite qualifier inventory and integration binaries are
unchanged. Ordinary CI does not acquire a new device requirement. Production
WGSL, runtime behavior, defaults, and retained lives are unchanged.

To rerun on an explicitly selected CPU Vulkan ICD:

```sh
VK_ICD_FILENAMES=/path/to/cpu-vulkan-icd.json RUST_TEST_THREADS=1 \
  cargo test -p alife_training --features cpu-backend,gpu-tests --lib \
  software_vulkan_matches_cpu_grip_reader_logits_features_and_eligibility \
  -- --ignored --nocapture
```

No device run was repeated for this documentation-only addition.
