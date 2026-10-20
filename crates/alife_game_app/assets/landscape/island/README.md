# Island assets

`island-assets.blend` and `island-terrain.blend` retain the base authoring sources.
`props.json` binds runtime placement and collision. Exported GLBs, source models
and production manifest digests must be kept together.

When rebuilding the base library, apply species overrides afterward: connected
Fern from `scripts/build_fern_fronds.py`, CoastalShrub from
`scripts/build_coastal_leaf_sprays.py`, Broadleaf from
`scripts/build_broadleaf_canopy.py`, and Conifer from
`scripts/build_conifer_leader.py`. The Fern and CoastalShrub exporters target
different species; older shrub exports must precede the CoastalShrub override.
The Conifer exporter reuses the retained NumPy geometric derivation with exact
input/output hashes. Use the compatible Blender version recorded in each exporter.

Refresh registrations with `python scripts/register_approved_art.py`, then run
the game app's `validate-production-assets` command. Historical audits and render
receipts remain in Git history; the removed snapshot validators are not current
gates. Inspect new exports visually before shipping them.
