//! A local survey, not a fabricated island: both axes use the same scale and the
//! relief samples the current world's surface. Crowded marks are never relocated.
use super::*;
use alife_world::{orchard::OrchardStage, TerrainSurface};
use image::{Rgba, RgbaImage};
const W: u32 = 288;
const H: u32 = 216;
const PAPER: [u8; 3] = [46, 74, 74];
const INK: [u8; 3] = [24, 47, 51];
const PORCELAIN: [u8; 3] = [253, 237, 194];
#[derive(Clone, Copy)]
struct MapBounds {
    center: Vec2,
    units: f32,
}
impl MapBounds {
    fn new(plots: &[(VoxelTileCoord, OrchardStage)]) -> Self {
        let points = plots
            .iter()
            .map(|(t, _)| Vec2::new(t.x as f32 + 0.5, t.z as f32 + 0.5));
        let (lo, hi) = points.fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        let (center, span) = if plots.is_empty() {
            (Vec2::ZERO, Vec2::ZERO)
        } else {
            ((hi + lo) * 0.5, hi - lo)
        };
        Self {
            center,
            units: (span.x / (W - 68) as f32)
                .max(span.y / (H - 68) as f32)
                .max(0.020),
        }
    }
    fn position(self, x: f32, y: f32) -> Vec2 {
        self.center + (Vec2::new(x, y) - Vec2::new(W as f32, H as f32) * 0.5) * self.units
    }
    fn pin(self, t: VoxelTileCoord) -> (i32, i32) {
        let p = (Vec2::new(t.x as f32 + 0.5, t.z as f32 + 0.5) - self.center) / self.units
            + Vec2::new(W as f32, H as f32) * 0.5;
        (p.x.round() as i32, p.y.round() as i32)
    }
}
pub(super) fn pick(
    plots: &[(VoxelTileCoord, OrchardStage)],
    normalized: Vec2,
) -> Option<VoxelTileCoord> {
    let bounds = MapBounds::new(plots);
    let p = (normalized + Vec2::splat(0.5)) * Vec2::new(W as f32, H as f32);
    plots
        .iter()
        .filter_map(|(tile, _)| {
            let (x, y) = bounds.pin(*tile);
            let distance = p.distance_squared(Vec2::new(x as f32, y as f32));
            (distance <= 13.0_f32.powi(2)).then_some((distance, *tile))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, t)| t)
}
fn rgba(c: [u8; 3]) -> Rgba<u8> {
    Rgba([c[0], c[1], c[2], 255])
}
fn shade(base: [u8; 3], value: f32) -> [u8; 3] {
    base.map(|n| (n as f32 + value).clamp(0., 255.) as u8)
}
fn dab(im: &mut RgbaImage, x: i32, y: i32, c: [u8; 3]) {
    if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
        im.put_pixel(x as u32, y as u32, rgba(c));
    }
}
fn disc(im: &mut RgbaImage, x: i32, y: i32, r: i32, c: [u8; 3]) {
    for j in -r..=r {
        for i in -r..=r {
            if i * i + j * j <= r * r {
                dab(im, x + i, y + j, c);
            }
        }
    }
}
pub(super) fn draw(
    surface: Option<&TerrainSurface>,
    plots: &[(VoxelTileCoord, OrchardStage)],
    selected: Option<VoxelTileCoord>,
) -> Image {
    let survey = MapBounds::new(plots);
    let mut im = RgbaImage::from_fn(W, H, |x, y| {
        let p = survey.position(x as f32, y as f32);
        let (tone, relief) =
            surface
                .and_then(|s| s.sample(p.x, p.y))
                .map_or((PAPER, 0.), |(h, g)| {
                    // Spaced contour intervals express actual elevation, not walkability.
                    let interval = (survey.units * 18.).max(0.22);
                    let edge = (h / interval).rem_euclid(1.);
                    let contour = (1. - (edge - 0.5).abs() * 13.).max(0.);
                    (
                        [67, 99, 92],
                        (-g[0] - g[1] * 0.55).clamp(-1., 1.) * 13. - contour * 13.,
                    )
                });
        let (gx, gy) = ((x + 20) % 28, (y + 22) % 28);
        let grid = if (gx == 0 && gy < 3) || (gy == 0 && gx < 3) {
            13.
        } else {
            0.
        };
        rgba(shade(tone, relief + grid))
    });
    // North needle and a fine instrument border. Neither implies an unobserved path.
    for y in 6..H - 6 {
        for x in [7, W - 8] {
            im.put_pixel(x, y, rgba([110, 141, 124]));
        }
    }
    for x in 7..W - 7 {
        for y in [6, H - 7] {
            im.put_pixel(x, y, rgba([110, 141, 124]));
        }
    }
    for y in 0_i32..14 {
        let wing = (y / 3).min(3);
        for x in -wing..=wing {
            dab(
                &mut im,
                22 + x,
                14 + y,
                if x < 0 { [172, 183, 154] } else { PORCELAIN },
            );
        }
    }
    for (tile, stage) in plots
        .iter()
        .filter(|(t, _)| Some(*t) != selected)
        .chain(plots.iter().filter(|(t, _)| Some(*t) == selected))
    {
        let (x, y) = survey.pin(*tile);
        let color = match stage {
            OrchardStage::Seed => [207, 158, 100],
            OrchardStage::Bud => [143, 214, 161],
            OrchardStage::Bloom => [247, 229, 197],
            OrchardStage::Ripe => [255, 162, 77],
        };
        disc(&mut im, x, y, 8, INK);
        match stage {
            OrchardStage::Seed => disc(&mut im, x, y + 2, 3, color),
            OrchardStage::Bud => {
                disc(&mut im, x - 3, y - 2, 3, color);
                disc(&mut im, x + 3, y - 4, 3, color);
                for d in -1..=4 {
                    dab(&mut im, x, y + d, color);
                }
            }
            OrchardStage::Bloom => {
                for (dx, dy) in [(-4, 0), (4, 0), (0, -4), (0, 4)] {
                    disc(&mut im, x + dx, y + dy, 3, color);
                }
                disc(&mut im, x, y, 2, [224, 162, 76]);
            }
            OrchardStage::Ripe => {
                disc(&mut im, x - 2, y + 2, 4, color);
                disc(&mut im, x + 3, y - 2, 3, color);
                disc(&mut im, x + 2, y - 5, 2, [144, 199, 141]);
            }
        }
        if Some(*tile) == selected {
            for k in 0..8 {
                for (dx, dy) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
                    dab(&mut im, x + dx * (12 - k), y + dy * 10, PORCELAIN);
                    dab(&mut im, x + dx * 12, y + dy * (10 - k), PORCELAIN);
                }
            }
        }
    }
    Image::from_dynamic(
        image::DynamicImage::ImageRgba8(im),
        true,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clustered_and_distant_beds_stay_on_the_same_isotropic_survey() {
        let plots = [
            (VoxelTileCoord::new(-88, 43), OrchardStage::Ripe),
            (VoxelTileCoord::new(-87, 43), OrchardStage::Seed),
            (VoxelTileCoord::new(463, -381), OrchardStage::Bud),
        ];
        let survey = MapBounds::new(&plots);
        for (tile, _) in plots {
            let (x, y) = survey.pin(tile);
            assert!((18..W as i32 - 18).contains(&x) && (18..H as i32 - 18).contains(&y));
            let round = survey.position(x as f32, y as f32);
            assert!(
                round.distance(Vec2::new(tile.x as f32 + 0.5, tile.z as f32 + 0.5))
                    <= survey.units * 0.72
            );
        }
        let (x, y) = survey.pin(plots[2].0);
        assert_eq!(
            pick(
                &plots,
                Vec2::new(x as f32 / W as f32 - 0.5, y as f32 / H as f32 - 0.5)
            ),
            Some(plots[2].0)
        );
        assert_eq!(pick(&plots, Vec2::splat(-0.49)), None);
        assert!(MapBounds::new(&[]).position(0., 0.).is_finite());
    }
}
