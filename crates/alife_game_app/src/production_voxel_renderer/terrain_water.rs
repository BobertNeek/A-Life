//! Static, world-aligned island shore color and mipmapped water normals.
//! All depth samples come from the selected collision surface. No world writes,
//! extra water geometry, per-frame texture updates, or custom shaders.
use super::*;
use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    pbr::UvChannel,
    prelude::Meshable,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const SHORE_SIZE: u32 = 1024;
const RIPPLE_SIZE: u32 = 128;
const SHORE_PADDING: f32 = 80.0;
const RIPPLE_METRES: f32 = 32.0;

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn water_color(x: f32, z: f32, ground: Option<f32>, shore_distance: f32) -> [f32; 4] {
    let depth = (-ground.unwrap_or(-12.0)).max(0.0);
    let near_shore = 1.0 - smooth(8.0, 45.0, shore_distance);
    let shallow = (1.0 - smooth(0.3, 10.0, depth)) * near_shore;
    let deep = Vec3::new(0.008, 0.055, 0.180);
    let turquoise = Vec3::new(0.035, 0.250, 0.300);
    let mut color = deep.lerp(turquoise, shallow);
    // Quiet, irregular wave bands; fade to the deep color at texture edges.
    let bands = (x * 0.61 + z * 0.37 + (z * 0.043).sin()).sin() * (x * 0.19 - z * 0.31).sin();
    color *= 1.0 + bands * shallow * 0.07;
    let broken = smooth(-0.5, 0.7, (x * 0.33 + (z * 0.19).sin()).sin());
    let foam = smooth(0.0, 0.07, depth)
        * (1.0 - smooth(0.16, 0.85, depth))
        * (0.35 + 0.45 * broken)
        * near_shore;
    color = color.lerp(Vec3::new(0.72, 0.78, 0.73), foam);
    [color.x, color.y, color.z, 1.0]
}

fn distances_to_dry_land(
    ground: &[Option<f32>],
    width: usize,
    step: bevy::prelude::Vec2,
) -> Vec<f32> {
    // Two bounded chamfer passes. This presentation mask prevents an existing
    // shallow sea-floor channel from drawing a long colored ribbon offshore.
    let height = ground.len() / width;
    let diagonal = step.length();
    let mut distances: Vec<_> = ground
        .iter()
        .map(|h| {
            if h.is_some_and(|h| h >= 0.0) {
                0.0
            } else {
                f32::INFINITY
            }
        })
        .collect();
    for reverse in [false, true] {
        for index in 0..distances.len() {
            let i = if reverse {
                distances.len() - 1 - index
            } else {
                index
            };
            let (x, y) = (i % width, i / width);
            let mut distance = distances[i];
            let horizontal = if reverse {
                (x + 1 < width).then_some(i + 1)
            } else {
                (x > 0).then(|| i - 1)
            };
            if let Some(other) = horizontal {
                distance = distance.min(distances[other] + step.x);
            }
            let vertical = if reverse {
                (y + 1 < height).then_some(i + width)
            } else {
                (y > 0).then(|| i - width)
            };
            if let Some(other) = vertical {
                distance = distance.min(distances[other] + step.y);
                if x > 0 {
                    distance = distance.min(distances[other - 1] + diagonal);
                }
                if x + 1 < width {
                    distance = distance.min(distances[other + 1] + diagonal);
                }
            }
            distances[i] = distance;
        }
    }
    distances
}

fn image_with_mips(size: u32, mut pixels: Vec<[f32; 4]>, normal: bool, repeat: bool) -> Image {
    let mut data = Vec::with_capacity((size * size * 4 * 4 / 3) as usize);
    let mut width = size;
    let mut levels = 0;
    loop {
        for p in &pixels {
            let rgb = if normal {
                Vec3::new(p[0], p[1], p[2]) * 0.5 + Vec3::splat(0.5)
            } else {
                let s = Color::linear_rgb(p[0], p[1], p[2]).to_srgba();
                Vec3::new(s.red, s.green, s.blue)
            };
            data.extend(
                rgb.to_array()
                    .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8),
            );
            data.push(255);
        }
        levels += 1;
        if width == 1 {
            break;
        }
        let next = width / 2;
        let mut smaller = Vec::with_capacity((next * next) as usize);
        for y in 0..next {
            for x in 0..next {
                let mut average = Vec3::ZERO;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let p = pixels[((y * 2 + dy) * width + x * 2 + dx) as usize];
                        average += Vec3::new(p[0], p[1], p[2]) * 0.25;
                    }
                }
                if normal {
                    average = average.normalize();
                }
                smaller.push([average.x, average.y, average.z, 1.0]);
            }
        }
        pixels = smaller;
        width = next;
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        if normal {
            TextureFormat::Rgba8Unorm
        } else {
            TextureFormat::Rgba8UnormSrgb
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = levels;
    image.data = Some(data);
    let address = if repeat {
        ImageAddressMode::Repeat
    } else {
        ImageAddressMode::ClampToEdge
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: address,
        address_mode_v: address,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

pub(super) fn island_water(
    surface: &alife_world::TerrainSurface,
    size: bevy::prelude::Vec2,
    center: Vec3,
) -> (Mesh, StandardMaterial, [Image; 2]) {
    let span = bevy::prelude::Vec2::new(
        (surface.width - 1) as f32 * surface.spacing,
        (surface.depth - 1) as f32 * surface.spacing,
    ) + bevy::prelude::Vec2::splat(SHORE_PADDING * 2.0);
    let origin = bevy::prelude::Vec2::new(surface.origin_x, surface.origin_z)
        - bevy::prelude::Vec2::splat(SHORE_PADDING);
    let mut ground = Vec::with_capacity((SHORE_SIZE * SHORE_SIZE) as usize);
    for y in 0..SHORE_SIZE {
        for x in 0..SHORE_SIZE {
            let world_x = origin.x + (x as f32 + 0.5) / SHORE_SIZE as f32 * span.x;
            let world_z = origin.y + (y as f32 + 0.5) / SHORE_SIZE as f32 * span.y;
            ground.push(surface.height(world_x, world_z));
        }
    }
    let distances = distances_to_dry_land(&ground, SHORE_SIZE as usize, span / SHORE_SIZE as f32);
    let shore = ground
        .iter()
        .zip(distances)
        .enumerate()
        .map(|(i, (&ground, distance))| {
            let world_x = origin.x + (i as u32 % SHORE_SIZE) as f32 / SHORE_SIZE as f32 * span.x;
            let world_z = origin.y + (i as u32 / SHORE_SIZE) as f32 / SHORE_SIZE as f32 * span.y;
            water_color(world_x, world_z, ground, distance)
        })
        .collect();
    let mut normals = Vec::with_capacity((RIPPLE_SIZE * RIPPLE_SIZE) as usize);
    for y in 0..RIPPLE_SIZE {
        for x in 0..RIPPLE_SIZE {
            let u = x as f32 / RIPPLE_SIZE as f32 * std::f32::consts::TAU;
            let v = y as f32 / RIPPLE_SIZE as f32 * std::f32::consts::TAU;
            let nx = 0.09 * (u * 4.0 + v * 2.0).cos() + 0.035 * (u * 9.0 - v * 3.0).cos();
            let ny = 0.045 * (u * 4.0 + v * 2.0).cos() - 0.012 * (u * 9.0 - v * 3.0).cos();
            let n = Vec3::new(nx, ny, 1.0).normalize();
            normals.push([n.x, n.y, n.z, 1.0]);
        }
    }
    let mut mesh = Mesh::from(
        bevy::prelude::Plane3d::default()
            .mesh()
            .size(size.x, size.y),
    );
    let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        unreachable!()
    };
    let uv0: Vec<_> = positions
        .iter()
        .map(|p| {
            [
                (p[0] + center.x - origin.x) / span.x,
                (p[2] + center.z - origin.y) / span.y,
            ]
        })
        .collect();
    let uv1: Vec<_> = positions
        .iter()
        .map(|p| {
            [
                (p[0] + center.x) / RIPPLE_METRES,
                (p[2] + center.z) / RIPPLE_METRES,
            ]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);
    mesh.generate_tangents().expect("valid water plane UVs");
    (
        mesh,
        StandardMaterial {
            perceptual_roughness: 0.38,
            reflectance: 0.35,
            normal_map_channel: UvChannel::Uv1,
            ..default()
        },
        [
            image_with_mips(SHORE_SIZE, shore, false, false),
            image_with_mips(RIPPLE_SIZE, normals, true, true),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_detail_stays_bounded_and_preserves_the_sea_plane() {
        let terrain = alife_world::island_terrain();
        let (mesh, _, images) = island_water(
            terrain.surface(),
            bevy::prelude::Vec2::splat(10000.0),
            Vec3::ZERO,
        );
        assert_eq!(mesh.indices().unwrap().len(), 6);
        assert_eq!(mesh.count_vertices(), 4);
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        assert!(positions.iter().all(|p| p[1] == 0.0));
        assert!(
            images
                .iter()
                .map(|i| i.data.as_ref().unwrap().len())
                .sum::<usize>()
                < 6 * 1024 * 1024
        );
        for image in images {
            let size = image.width();
            assert_eq!(image.texture_descriptor.mip_level_count, size.ilog2() + 1);
            let expected = (0..=size.ilog2())
                .map(|level| (size >> level).pow(2) as usize * 4)
                .sum::<usize>();
            assert_eq!(image.data.as_ref().unwrap().len(), expected);
        }
        let deep = water_color(0.0, 0.0, None, f32::INFINITY);
        let shallow = water_color(0.0, 0.0, Some(-2.0), 2.0);
        let foam = water_color(0.0, 0.0, Some(-0.15), 0.5);
        assert!(deep[2] > deep[1] && shallow[1] > deep[1] && foam[0] > shallow[0]);
    }

    #[test]
    fn shore_tint_does_not_expose_a_submerged_channel_far_from_land() {
        let open_water = water_color(0.0, 0.0, None, f32::INFINITY);
        assert_eq!(water_color(0.0, 0.0, Some(-1.8), 100.0), open_water);
        assert!(water_color(0.0, 0.0, Some(-1.8), 3.0)[1] > open_water[1]);
    }
}
