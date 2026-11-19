# Native acceptance, next week only

The offscreen cameras are *art studies*, not the game's orthographic camera. This
recipe is for a separately booked RTX 3050 window. It has not been run.

1. Use two clean checkouts with no user profiles or caches: the comparison
   shoreline checkpoint `810e4f872680dc569affa6c8d2e3c31f79caa86c`, then
   the weekly candidate. Record both SHAs. Do not use a simulator/training
   checkout. Confirm adapter and idle session; stop if either is unavailable.
2. In each checkout's own new PowerShell session, prepare an unmistakably new
   profile (the parent of `-UiSettings` scopes all writable player saves):

   ```powershell
   $qa = Join-Path $env:TEMP ("qualia-island-art-" + [guid]::NewGuid())
   New-Item -ItemType Directory -Path $qa -ErrorAction Stop | Out-Null
   $env:ALIFE_GRAPHICS_CAPTURE_DIR = Join-Path $qa "captures"
   $env:ALIFE_GRAPHICS_CAPTURE_MAX_FRAMES = "1"  # use F12 for subsequent views
   $argsQA = @{
       NewGame = $true; Seed = 20261119; Population = 1
       UiSettings = (Join-Path $qa "ui.json")
       Profile = "MinSpecComfort1080p"; Resolution = "1920x1080"
       GraphicsBackend = "vulkan"; RequireGpu = $true; BuildProfile = "release"
   }
   .\scripts\run_production_voxel_frontend.ps1 @argsQA -PreviewCommand
   .\scripts\run_production_voxel_frontend.ps1 @argsQA -RecordPerformance
   ```

3. Allow the authored chunks to finish streaming. Capture the nursery encounter
   view, the player's normal orthographic bank view near (x=345,z=115), and a
   wide bank-to-hill view. Press F12 to save both pixels and the renderer's JSON
   camera transform, focus, terrain binding and view-height receipt. For an A/B
   use one copied **baseline F12 JSON receipt** at a time: set
   `$env:ALIFE_ISLAND_CAPTURE_POSE = (Resolve-Path "baseline-view.json").Path`
   in the candidate session before launch. The optional lock applies only to
   that terrain binding and native orthographic lens, before normal safety
   constraints, and only when a capture directory is set. Compare the emitted
   camera and projection receipts; do not infer a match from the framing.
   Remove that environment variable and restart for free camera/contact tests. Do not label the 86 cm **perspective Blender** plate an in-game view.
4. Pan the same shore outward through the 150 m near-chunk cutoff and 350 m
   middle/far handoff; record visible gaps, jumping edges and ungrounded foliage.
   Inspect rock and reed contacts at maximum stock encounter zoom. Watch a
   creature actually walk across and a toy roll through the bank in the scratch
   profile. A still frame or mesh proof alone cannot pass contact.
5. For cost, use a separate warmed run per build with the same profile, save,
   camera receipt and resolution. No screenshots or camera interactions during
   timing. Keep the renderer's CPU/GPU frame-time distribution, 1% lows and
   adapter string; mark any unavailable counter unknown. Check minimum settings
   separately to catch changed shadow or fog budgets rather than conflating
   quality settings. Keep the original recordings and compare before promoting.

The default and this recipe use V1: its binding must match the original save.
The northern V2 alternate remains an isolated generated map with a separate
restore test, not an unqualified playable switch. Do not overwrite V1 to stage V2.

For the new material pass, use `4bf90a3438bc160665580ba711c44c1c96526879`
as the additional isolated predecessor. Port each setup to its own writable
profile; compare the same V1 camera receipt before/after. Verify that the
first ordinary terrain/trees appear even if the 76 near habitat chunks stream
later. Inspect the junction between meadow and dry sand, wet/rough glints,
reed roots and the real gap through each grass colony. At the 150 m
handoff check hue as well as silhouette: normal + albedo atlas are 2688×2352;
the less sensitive AO/roughness pack is 672×588. Check cutout foliage shimmer
and alpha overlap against the original on both min-spec and normal profiles.
Record driver allocated memory rather than equating PNG bytes with VRAM.

For the optional marine-day artwork in the weekly candidate, compare from both
the low bank and a high orthographic pan. The inside of the dome, not a camera
overlay, should fill only empty sky. Check an island-to-non-island load, look
across the original water horizon, and zoom the camera across the globe; a
visible rim, sky occluding land or any sun shadow from the sky is a failure.
The background has one unlit draw; profile occupied habitats and dense cutouts
with and without the candidate, not an empty sky-only benchmark.

For grove and mineral integration also compare against weekly `c6a3e18`.
Use the actual stone at x=254.68 z=-177.68, the 1.03 m eye at x=258.40
z=-170.00, and the 144 m vertical-span guardian lock at x=247 z=-184.
A small physical overhang is expected on the downhill side: the original
flat-foot boulder contacts the slope uphill and this art never moves its
solid proxy. Fail if the whole rock is airborne, UVs flash on a handoff,
a hidden stone disables plants, or the mineral normal maps bind to foliage.
Pan from close shale into the guardian shoulder; the lowland should not
acquire a repeating stripe or an outlined polygon grid. Keep the
horizon/cove as a control rather than claiming a different sky.

For the new spruce meshes use the previous geometry from
`16090b4e4a25458696ef74ff461804f553ad3d62` versus the weekly branch. Save
the ordinary native orthographic creature-height and guardian receipts looking through the
stand around **x=-162,z=-38**. A close comparison for pine 434 is from x=-149.04,z=-38.54
toward x=-151.27,z=-32.78; the low study is perspective, so judge actual scale in the
normal gameplay lens separately. Verify the ordinary tree remains visible before the pack
loads, the same trunk still stops the toy, all three LODs show the same needle material,
and broadleaves/stones keep their own. Pan through the 100 m/240 m prop transitions in a
dense clump. Profile one shared pine surface and the three mipped needle maps; the source
and candidate have the same per-visible-tree draw count, not a measured frame-time result.

**Studio-only pause (19 November, 04:34 UTC).** All six branch-specific automated
validation workflows are manual-only. Do not dispatch them, run game/PC checks,
or open a draft PR that would start the general CI run this week. Keep their
previously completed artifacts. A separate Blender-only studio render can
produce art; this does not imply a new validation pass. At the next agreed
test window, dispatch the narrow asset/runtime checks deliberately and record
the exact checked-out release before proceeding with interactive acceptance.

For the later cutout-spruce pass, read the fixed under-canopy / guardian art board
as an art reference, not game screenshots. In the next agreed play slot inspect
needle coverage from 3 to 8 metres, middling stands around 100 m, the 240 m
handoff and an actual far guardian span above 350 m. Check clipped shadows,
grazing sunlight and moving-camera shimmer on the real GPU. Record fragment and
shadow work as well as triangle count. The new crowns in
the workbench do not change tree transforms or their inherited solid trunks.
