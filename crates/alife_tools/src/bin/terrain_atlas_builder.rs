use std::{env, error::Error, fs, path::PathBuf};

use alife_world::persistence::PortableAssetDigest;
use image::{imageops::FilterType, DynamicImage, ImageFormat, Rgba, RgbaImage};
use serde::Deserialize;

const TERRAIN_GENERATION_SCHEMA: &str = "alife.fvr11.terrain_material_generation.v1";
const OUTPUT_FILES: [&str; 3] = [
    "terrain_albedo_atlas.png",
    "terrain_normal_atlas.png",
    "terrain_orm_atlas.png",
];

#[derive(Debug, Deserialize)]
struct TerrainGenerationConfig {
    schema: String,
    tile_size: u32,
    gutter: u32,
    grid_columns: u32,
    grid_rows: u32,
    source_prompt: String,
    slots: Vec<TerrainMaterialSlot>,
}

#[derive(Debug, Deserialize)]
struct TerrainMaterialSlot {
    id: String,
    normal_strength: f32,
    roughness: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: terrain_atlas_builder <source-sheet.png> <generation-config.json> <output-directory>"
                .into(),
        );
    }

    let source_path = PathBuf::from(&args[0]);
    let config_path = PathBuf::from(&args[1]);
    let output_directory = PathBuf::from(&args[2]);
    let source = image::open(&source_path)?.into_rgba8();
    let config: TerrainGenerationConfig = serde_json::from_str(&fs::read_to_string(&config_path)?)?;
    validate_config(&config, &source)?;

    let cell_size = config.tile_size + config.gutter * 2;
    let atlas_width = config.grid_columns * cell_size;
    let atlas_height = config.grid_rows * cell_size;
    let mut albedo = RgbaImage::new(atlas_width, atlas_height);
    let mut normal = RgbaImage::new(atlas_width, atlas_height);
    let mut orm = RgbaImage::new(atlas_width, atlas_height);

    for (index, slot) in config.slots.iter().enumerate() {
        let column = index as u32 % config.grid_columns;
        let row = index as u32 / config.grid_columns;
        let x0 = source.width() * column / config.grid_columns;
        let x1 = source.width() * (column + 1) / config.grid_columns;
        let y0 = source.height() * row / config.grid_rows;
        let y1 = source.height() * (row + 1) / config.grid_rows;
        let crop = image::imageops::crop_imm(&source, x0, y0, x1 - x0, y1 - y0).to_image();
        let tile = make_wrapped_tile(&crop, config.tile_size, 8.min(config.tile_size / 4));
        let normal_tile = derive_normal_map(&tile, slot.normal_strength);
        let orm_tile = derive_orm_map(&tile, slot.roughness);

        extrude_into_atlas(&mut albedo, &tile, column, row, config.gutter);
        extrude_into_atlas(&mut normal, &normal_tile, column, row, config.gutter);
        extrude_into_atlas(&mut orm, &orm_tile, column, row, config.gutter);
        println!("slot={index:02} id={} column={column} row={row}", slot.id);
    }

    fs::create_dir_all(&output_directory)?;
    for (file_name, atlas) in OUTPUT_FILES.iter().zip([albedo, normal, orm].into_iter()) {
        let output_path = output_directory.join(file_name);
        DynamicImage::ImageRgba8(atlas).save_with_format(&output_path, ImageFormat::Png)?;
        let size = fs::metadata(&output_path)?.len();
        let digest = PortableAssetDigest::for_file(&output_path)?;
        println!(
            "output={} dimensions={}x{} bytes={} digest={}",
            output_path.display(),
            atlas_width,
            atlas_height,
            size,
            digest.0
        );
    }

    Ok(())
}

fn validate_config(
    config: &TerrainGenerationConfig,
    source: &RgbaImage,
) -> Result<(), Box<dyn Error>> {
    if config.schema != TERRAIN_GENERATION_SCHEMA {
        return Err(format!("unsupported terrain generation schema: {}", config.schema).into());
    }
    if config.tile_size == 0
        || config.gutter == 0
        || config.grid_columns == 0
        || config.grid_rows == 0
    {
        return Err("terrain atlas dimensions and gutter must be positive".into());
    }
    if config.source_prompt.trim().is_empty() {
        return Err("terrain source prompt must be recorded".into());
    }
    let expected_slots = (config.grid_columns * config.grid_rows) as usize;
    if config.slots.len() != expected_slots {
        return Err(format!(
            "terrain config has {} slots; expected {expected_slots}",
            config.slots.len()
        )
        .into());
    }
    if source.width() < config.grid_columns || source.height() < config.grid_rows {
        return Err("terrain source sheet is smaller than its configured grid".into());
    }
    for slot in &config.slots {
        if slot.id.trim().is_empty()
            || !(0.0..=2.0).contains(&slot.normal_strength)
            || !(0.0..=1.0).contains(&slot.roughness)
        {
            return Err(format!("invalid terrain material slot: {}", slot.id).into());
        }
    }
    Ok(())
}

fn make_wrapped_tile(source: &RgbaImage, size: u32, blend: u32) -> RgbaImage {
    let mut tile = image::imageops::resize(source, size, size, FilterType::Lanczos3);
    let blend = blend.min(size / 2);
    if blend == 0 {
        return tile;
    }

    for inset in 0..blend {
        let opposite = size - 1 - inset;
        let strength = 0.5 * (blend - inset) as f32 / blend as f32;
        for y in 0..size {
            let left = *tile.get_pixel(inset, y);
            let right = *tile.get_pixel(opposite, y);
            tile.put_pixel(inset, y, mix_rgba(left, right, strength));
            tile.put_pixel(opposite, y, mix_rgba(right, left, strength));
        }
    }
    for inset in 0..blend {
        let opposite = size - 1 - inset;
        let strength = 0.5 * (blend - inset) as f32 / blend as f32;
        for x in 0..size {
            let top = *tile.get_pixel(x, inset);
            let bottom = *tile.get_pixel(x, opposite);
            tile.put_pixel(x, inset, mix_rgba(top, bottom, strength));
            tile.put_pixel(x, opposite, mix_rgba(bottom, top, strength));
        }
    }
    tile
}

fn derive_normal_map(source: &RgbaImage, strength: f32) -> RgbaImage {
    let width = source.width();
    let height = source.height();
    RgbaImage::from_fn(width, height, |x, y| {
        let left = luminance(source.get_pixel((x + width - 1) % width, y));
        let right = luminance(source.get_pixel((x + 1) % width, y));
        let up = luminance(source.get_pixel(x, (y + height - 1) % height));
        let down = luminance(source.get_pixel(x, (y + 1) % height));
        let nx = -(right - left) * strength;
        let ny = -(down - up) * strength;
        let nz = 1.0_f32;
        let inverse_length = (nx * nx + ny * ny + nz * nz).sqrt().recip();
        Rgba([
            signed_normal_channel(nx * inverse_length),
            signed_normal_channel(ny * inverse_length),
            signed_normal_channel(nz * inverse_length),
            255,
        ])
    })
}

fn derive_orm_map(source: &RgbaImage, roughness: f32) -> RgbaImage {
    let width = source.width();
    let height = source.height();
    RgbaImage::from_fn(width, height, |x, y| {
        let center = luminance(source.get_pixel(x, y));
        let neighbors = [
            luminance(source.get_pixel((x + width - 1) % width, y)),
            luminance(source.get_pixel((x + 1) % width, y)),
            luminance(source.get_pixel(x, (y + height - 1) % height)),
            luminance(source.get_pixel(x, (y + 1) % height)),
        ];
        let neighbor_average = neighbors.into_iter().sum::<f32>() / neighbors.len() as f32;
        let cavity = (neighbor_average - center).max(0.0);
        let ambient_occlusion = (0.82 + center * 0.16 - cavity * 0.38).clamp(0.48, 1.0);
        let varied_roughness = (roughness + (0.5 - center) * 0.08).clamp(0.04, 1.0);
        Rgba([
            unit_channel(ambient_occlusion),
            unit_channel(varied_roughness),
            0,
            255,
        ])
    })
}

fn extrude_into_atlas(atlas: &mut RgbaImage, tile: &RgbaImage, column: u32, row: u32, gutter: u32) {
    assert_eq!(tile.width(), tile.height());
    let size = tile.width();
    let cell = size + gutter * 2;
    let origin_x = column * cell;
    let origin_y = row * cell;
    assert!(origin_x + cell <= atlas.width());
    assert!(origin_y + cell <= atlas.height());

    for cell_y in 0..cell {
        let source_y = (cell_y as i64 - i64::from(gutter)).clamp(0, i64::from(size - 1)) as u32;
        for cell_x in 0..cell {
            let source_x = (cell_x as i64 - i64::from(gutter)).clamp(0, i64::from(size - 1)) as u32;
            atlas.put_pixel(
                origin_x + cell_x,
                origin_y + cell_y,
                *tile.get_pixel(source_x, source_y),
            );
        }
    }
}

fn mix_rgba(origin: Rgba<u8>, opposite: Rgba<u8>, opposite_weight: f32) -> Rgba<u8> {
    let origin_weight = 1.0 - opposite_weight;
    Rgba(std::array::from_fn(|index| {
        (f32::from(origin[index]) * origin_weight + f32::from(opposite[index]) * opposite_weight)
            .round()
            .clamp(0.0, 255.0) as u8
    }))
}

fn luminance(pixel: &Rgba<u8>) -> f32 {
    (f32::from(pixel[0]) * 0.2126 + f32::from(pixel[1]) * 0.7152 + f32::from(pixel[2]) * 0.0722)
        / 255.0
}

fn signed_normal_channel(value: f32) -> u8 {
    unit_channel(value * 0.5 + 0.5)
}

fn unit_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn striped_tile(size: u32) -> RgbaImage {
        RgbaImage::from_fn(size, size, |x, y| {
            Rgba([(x * 23) as u8, (y * 19) as u8, ((x + y) * 11) as u8, 255])
        })
    }

    #[test]
    fn wrapped_tile_matches_opposite_outer_edges() {
        let tile = make_wrapped_tile(&striped_tile(8), 8, 2);
        for offset in 0..8 {
            assert_eq!(tile.get_pixel(0, offset), tile.get_pixel(7, offset));
            assert_eq!(tile.get_pixel(offset, 0), tile.get_pixel(offset, 7));
        }
    }

    #[test]
    fn derived_maps_have_expected_packed_channels() {
        let tile = make_wrapped_tile(&striped_tile(8), 8, 2);
        let normal = derive_normal_map(&tile, 0.6);
        let orm = derive_orm_map(&tile, 0.82);

        assert_eq!(normal.dimensions(), (8, 8));
        assert_eq!(orm.dimensions(), (8, 8));
        assert!(normal.pixels().all(|pixel| pixel[2] >= 128));
        assert!(orm.pixels().all(|pixel| pixel[2] == 0));
        assert!(orm.pixels().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn atlas_gutter_extrudes_tile_edges_and_corners() {
        let tile = RgbaImage::from_fn(2, 2, |x, y| {
            Rgba([(x * 100) as u8, (y * 100) as u8, 40, 255])
        });
        let mut atlas = RgbaImage::new(4, 4);
        extrude_into_atlas(&mut atlas, &tile, 0, 0, 1);

        assert_eq!(atlas.get_pixel(0, 0), tile.get_pixel(0, 0));
        assert_eq!(atlas.get_pixel(3, 0), tile.get_pixel(1, 0));
        assert_eq!(atlas.get_pixel(0, 3), tile.get_pixel(0, 1));
        assert_eq!(atlas.get_pixel(3, 3), tile.get_pixel(1, 1));
    }
}
