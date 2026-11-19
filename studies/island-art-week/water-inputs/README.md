# Exact reference water, not a painted replacement

These two PNGs are losslessly encoded level-zero pixels from the real
`production_voxel_renderer::terrain_water::island_water` builder on the default
island. The hosted CPU-only export at commit
`d5003d02921f15d7e97dc144bfc7655aea2ef459` ran
`terrain_water::tests::export_review_water` (Actions run 44440606818). The PNG
round trip compared equal to every source RGB byte. Production still builds its
own mipmapped textures; Blender reads this study fixture, not vice versa.
`water.json` carries the actual sampler coordinate scale and material parameters.
Its terrain binding version is the serialization schema version, not the
opt-in ridge map version.

- Shore colour, sRGB: `ea8e124488e4abb4276957672c52a5b45fdf98e8d0ea82d362442175e52eddd9`
- Ripple normals, linear: `d4936b7a685919ed151cddd0141412064db9f2b5482db88ba60c9c6615e99519`

For changes, call the same ignored offline test with `QUALIA_REVIEW_WATER_DIR`
pointing to a new directory. Encode the two PPM files as lossless PNG as in the
export workflow. Use the same inputs on both sides of a coastal A/B unless
testing a water change. Blender lighting, tangent handling and the BRDF remain
an approximation of Bevy; these are never native gameplay frames.
