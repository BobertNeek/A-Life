# Landscape assets

`landscape.blend` retains the editable prop library. Use Blender 5.2.1 with
`scripts/export_landscape.py` to export it. The playable Island has its own sources
and overrides under `island/`; retain the reusable Highlands library.

After export, refresh registrations with `python scripts/register_approved_art.py`
and run the game app's `validate-production-assets` command. Exported geometry
and manifest digests must be updated together.
