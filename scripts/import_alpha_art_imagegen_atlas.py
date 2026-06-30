#!/usr/bin/env python3
"""Import generated source sheets into the committed alpha_art_v1 pack.

The source sheets are generated outside the repository. This importer keeps the
grid map and post-processing in source control so the committed PNGs are not a
mysterious one-off artifact. The preferred path uses a terrain-only sheet and a
chroma-keyed sprite sheet; the old single-atlas path is retained only for
reproducibility of rejected visual attempts.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "alife_game_app" / "assets" / "alpha_art_v1"
MANIFEST = OUT / "alpha_art_manifest.json"
SIZE = 128
ART_DIRECTION = "production-alpha-imagegen-ground-tiles-v33"


# Crop boxes are in source-atlas pixels. Terrain crops deliberately use the
# interior of each generated tile so the runtime never receives baked dark
# atlas gutters or card shadows.
TERRAIN_CROPS: dict[str, tuple[int, int, int, int]] = {
    "terrain_safe_grass": (70, 70, 290, 290),
    "terrain_soil_path": (372, 70, 592, 290),
    "terrain_resource_grove": (670, 70, 892, 292),
    "terrain_hazard_pressure": (974, 70, 1196, 292),
    "terrain_stone_rough": (70, 370, 290, 590),
    "terrain_water": (372, 370, 592, 590),
    "terrain_sand": (670, 370, 892, 592),
}


SPRITE_CROPS: dict[str, tuple[int, int, int, int]] = {
    "creature_idle": (18, 606, 318, 802),
    "creature_hurt": (320, 606, 620, 802),
    "selection_ring": (630, 600, 930, 810),
    "food_sprout": (932, 606, 1232, 810),
    "hazard_crystal": (18, 826, 318, 1012),
    "rock_cluster": (320, 826, 620, 1012),
    "prop_grass_tuft": (630, 826, 930, 1012),
    "prop_pebble_cluster": (932, 826, 1232, 1012),
    "prop_leaf_patch": (18, 1010, 318, 1252),
    "prop_mushroom_cluster": (320, 1010, 620, 1252),
    "prop_warning_shard": (630, 1010, 930, 1252),
    "food_bloom": (932, 1010, 1232, 1252),
}


DERIVED_SPRITES: dict[str, str] = {
    "creature_moving": "creature_idle",
    "creature_eat": "creature_idle",
    "creature_sleep": "creature_idle",
    "creature_signal": "creature_hurt",
    "selection_pulse": "selection_ring",
    "hazard_glow": "hazard_crystal",
    "prop_warning_shard": "prop_warning_shard",
}

TERRAIN_GRID: dict[str, tuple[int, int]] = {
    "terrain_safe_grass": (0, 0),
    "terrain_soil_path": (1, 0),
    "terrain_resource_grove": (2, 0),
    "terrain_hazard_pressure": (3, 0),
    "terrain_stone_rough": (0, 1),
    "terrain_water": (1, 1),
    "terrain_sand": (2, 1),
}

SPRITE_GRID: dict[str, tuple[int, int]] = {
    "creature_idle": (0, 0),
    "creature_hurt": (1, 0),
    "selection_ring": (2, 0),
    "food_sprout": (3, 0),
    "hazard_crystal": (0, 1),
    "rock_cluster": (1, 1),
    "prop_grass_tuft": (2, 1),
    "prop_pebble_cluster": (3, 1),
    "prop_leaf_patch": (0, 2),
    "prop_mushroom_cluster": (1, 2),
    "prop_warning_shard": (2, 2),
    "food_bloom": (3, 2),
}


def resize_tile(source: Image.Image, box: tuple[int, int, int, int]) -> Image.Image:
    tile = source.crop(box).convert("RGBA")
    tile = tile.resize((SIZE, SIZE), Image.Resampling.LANCZOS)
    return tile


def grid_box(
    source: Image.Image,
    columns: int,
    rows: int,
    column: int,
    row: int,
    margin: int,
) -> tuple[int, int, int, int]:
    left = round(source.width * column / columns)
    right = round(source.width * (column + 1) / columns)
    top = round(source.height * row / rows)
    bottom = round(source.height * (row + 1) / rows)
    return (
        min(max(0, left + margin), source.width),
        min(max(0, top + margin), source.height),
        min(max(0, right - margin), source.width),
        min(max(0, bottom - margin), source.height),
    )


def background_mask(cell: Image.Image) -> Image.Image:
    """Return an alpha mask that removes generated atlas backgrounds."""

    rgb = cell.convert("RGB")
    width, height = rgb.size
    bg = rgb.getpixel((0, 0))
    px = rgb.load()
    mask = Image.new("L", rgb.size, 0)
    mask_px = mask.load()

    # The preferred sprite sheet uses a flat magenta chroma key. The fallback
    # branch also works for older dark-card generated atlases.
    is_magenta_key = bg[0] > 190 and bg[1] < 80 and bg[2] > 170
    for y in range(height):
        for x in range(width):
            r, g, b = px[x, y]
            if is_magenta_key:
                distance = abs(r - bg[0]) + abs(g - bg[1]) + abs(b - bg[2])
                magenta_spill = r > 110 and b > 95 and g < 125 and (r - g) > 45 and (b - g) > 35
                if distance > 105 and not magenta_spill:
                    mask_px[x, y] = 255
            elif (
                max(r, g, b) > 46
                or (r > 42 and r > g * 1.18 and r > b * 1.18)
                or sum(abs(channel - bg_channel) for channel, bg_channel in zip((r, g, b), bg)) > 22
            ):
                mask_px[x, y] = 255

    mask = mask.filter(ImageFilter.MedianFilter(3)).filter(ImageFilter.GaussianBlur(0.35))
    return mask


def sprite_from_cell(source: Image.Image, box: tuple[int, int, int, int], pad: int = 12) -> Image.Image:
    cell = source.crop(box).convert("RGBA")
    alpha = background_mask(cell)
    cell.putalpha(alpha)
    bbox = alpha.getbbox()
    if bbox is None:
        raise ValueError(f"no visible sprite found for crop {box}")

    left = max(0, bbox[0] - pad)
    top = max(0, bbox[1] - pad)
    right = min(cell.width, bbox[2] + pad)
    bottom = min(cell.height, bbox[3] + pad)
    sprite = cell.crop((left, top, right, bottom))
    sprite.thumbnail((SIZE - 10, SIZE - 10), Image.Resampling.LANCZOS)

    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    x = (SIZE - sprite.width) // 2
    y = (SIZE - sprite.height) // 2
    out.alpha_composite(sprite, (x, y))
    return out


def draw_variant(asset_id: str, base: Image.Image) -> Image.Image:
    img = base.copy()
    draw = ImageDraw.Draw(img, "RGBA")
    if asset_id == "creature_moving":
        draw.line([(18, 83), (4, 89)], fill=(180, 255, 255, 125), width=3)
        draw.line([(24, 95), (7, 101)], fill=(180, 255, 255, 90), width=2)
    elif asset_id == "creature_eat":
        draw.ellipse((90, 72, 111, 91), fill=(90, 210, 76, 215))
        draw.ellipse((103, 66, 114, 77), fill=(226, 82, 105, 230))
    elif asset_id == "creature_sleep":
        draw.line([(91, 30), (108, 30), (94, 45), (111, 45)], fill=(235, 255, 255, 190), width=3)
        draw.line([(103, 18), (117, 18), (106, 30), (120, 30)], fill=(235, 255, 255, 150), width=2)
    elif asset_id == "creature_signal":
        for radius, alpha in [(30, 95), (42, 55)]:
            draw.arc(
                (64 - radius, 45 - radius // 2, 64 + radius, 45 + radius // 2),
                start=205,
                end=335,
                fill=(255, 238, 132, alpha),
                width=2,
            )
    elif asset_id == "selection_pulse":
        draw.ellipse((14, 38, 114, 92), outline=(127, 255, 220, 150), width=3)
    elif asset_id == "hazard_glow":
        glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        g = ImageDraw.Draw(glow, "RGBA")
        g.ellipse((22, 24, 106, 112), fill=(255, 45, 32, 40))
        glow = glow.filter(ImageFilter.GaussianBlur(7))
        glow.alpha_composite(img)
        img = glow
    return img


def save_png(path: Path, img: Image.Image) -> None:
    img.save(path, "PNG", optimize=True)


def update_manifest() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    manifest["art_direction"] = ART_DIRECTION
    for entry in manifest["entries"]:
        path = ROOT / entry["relative_path"]
        if not path.exists():
            continue
        with Image.open(path) as img:
            entry["width"], entry["height"] = img.size
        entry["file_size_bytes"] = path.stat().st_size
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def import_terrain_sheet(terrain_source: Image.Image) -> dict[str, Image.Image]:
    generated: dict[str, Image.Image] = {}
    for asset_id, (column, row) in TERRAIN_GRID.items():
        generated[asset_id] = resize_tile(
            terrain_source,
            grid_box(terrain_source, 4, 2, column, row, margin=4),
        )
    return generated


def import_sprite_sheet(sprite_source: Image.Image) -> dict[str, Image.Image]:
    generated: dict[str, Image.Image] = {}
    for asset_id, (column, row) in SPRITE_GRID.items():
        generated[asset_id] = sprite_from_cell(
            sprite_source,
            grid_box(sprite_source, 4, 3, column, row, margin=8),
            pad=18,
        )

    return generated


def import_from_split_sheets(terrain_source: Image.Image, sprite_source: Image.Image) -> dict[str, Image.Image]:
    generated = import_terrain_sheet(terrain_source)
    generated.update(import_sprite_sheet(sprite_source))
    return generated


def import_from_legacy_single_atlas(source: Image.Image) -> dict[str, Image.Image]:
    generated: dict[str, Image.Image] = {}
    for asset_id, box in TERRAIN_CROPS.items():
        generated[asset_id] = resize_tile(source, box)

    for asset_id, box in SPRITE_CROPS.items():
        generated[asset_id] = sprite_from_cell(source, box)

    return generated


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, help="legacy single source atlas")
    parser.add_argument("--terrain-source", type=Path, help="preferred terrain-only source sheet")
    parser.add_argument("--sprite-source", type=Path, help="preferred chroma-keyed sprite source sheet")
    parser.add_argument(
        "--terrain-only",
        action="store_true",
        help="replace only committed terrain PNGs; keep the current sprite sheet crops",
    )
    args = parser.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)

    if args.terrain_only:
        if not args.terrain_source or args.sprite_source or args.source:
            parser.error("--terrain-only requires only --terrain-source")
        generated = import_terrain_sheet(Image.open(args.terrain_source).convert("RGB"))
    elif args.terrain_source and args.sprite_source:
        generated = import_from_split_sheets(
            Image.open(args.terrain_source).convert("RGB"),
            Image.open(args.sprite_source).convert("RGB"),
        )
    elif args.source:
        generated = import_from_legacy_single_atlas(Image.open(args.source).convert("RGB"))
    else:
        parser.error("provide --terrain-source and --sprite-source, or legacy --source")

    for asset_id, base_id in DERIVED_SPRITES.items():
        if asset_id in generated:
            continue
        if base_id not in generated:
            continue
        generated[asset_id] = draw_variant(asset_id, generated[base_id])

    for asset_id, image in generated.items():
        save_png(OUT / f"{asset_id}.png", image)

    update_manifest()
    print(f"Imported {len(generated)} assets into {OUT}")


if __name__ == "__main__":
    main()
