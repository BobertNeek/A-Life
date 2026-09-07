//! Ground the displayed pose, including its articulated parts, on displayed terrain.
use super::*;
use bevy::camera::primitives::Aabb;

#[derive(Resource)]
pub(super) struct RenderedTerrainSurface {
    stride: f32,
    quads: BTreeMap<VoxelTileCoord, [[f32; 3]; 4]>,
}

impl RenderedTerrainSurface {
    pub(super) fn from_meshes<'a>(meshes: impl IntoIterator<Item = &'a Mesh>, stride: f32) -> Self {
        let mut quads = BTreeMap::new();
        for mesh in meshes {
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            for quad in positions.chunks_exact(4) {
                let tile = VoxelTileCoord::new(
                    (quad[0][0] + stride * 0.5 - 0.5).round() as i32,
                    (quad[0][2] + stride * 0.5 - 0.5).round() as i32,
                );
                quads.insert(tile, [quad[0], quad[1], quad[2], quad[3]]);
            }
        }
        Self { stride, quads }
    }

    fn height(&self, position: Vec3) -> Option<f32> {
        let cell = |coordinate: f32| {
            ((coordinate + self.stride * 0.5 - 0.5) / self.stride).floor() as i32
                * self.stride as i32
        };
        let [a, b, c, d] = *self
            .quads
            .get(&VoxelTileCoord::new(cell(position.x), cell(position.z)))?;
        let u = ((position.x - a[0]) / self.stride).clamp(0.0, 1.0);
        let v = ((position.z - a[2]) / self.stride).clamp(0.0, 1.0);
        // Match the terrain mesh's two triangles, including corner relief.
        Some(if v >= u {
            a[1] * (1.0 - v) + b[1] * (v - u) + c[1] * u
        } else {
            a[1] * (1.0 - u) + c[1] * v + d[1] * (u - v)
        })
    }
}

pub(super) fn ground_creatures(
    surface_mesh: Res<RenderedTerrainSurface>,
    mut roots: bevy::prelude::Query<
        (
            &mut Transform,
            &Fvr04ProductionCreatureVisualMarker,
            &Children,
        ),
        With<ProductionCreatureAssemblyRoot>,
    >,
    parts: bevy::prelude::Query<(&Transform, &Aabb), Without<ProductionCreatureAssemblyRoot>>,
) {
    for (mut root, visual, children) in &mut roots {
        let mut lowest = f32::INFINITY;
        let mut surface = surface_mesh
            .height(root.translation)
            .unwrap_or(visual.surface_height);
        for child in children.iter() {
            let Ok((part, aabb)) = parts.get(*child) else {
                continue;
            };
            let bounds = CreatureVisualBounds::new(aabb.min().to_array(), aabb.max().to_array());
            for corner in bounds.corners() {
                let point =
                    root.rotation * (root.scale * part.transform_point(Vec3::from_array(corner)));
                lowest = lowest.min(point.y);
                if let Some(height) = surface_mesh.height(root.translation + point) {
                    surface = surface.max(height);
                }
            }
        }
        root.translation.y = if lowest.is_finite() {
            surface + 0.04 - lowest
        } else {
            grounded_root_height(
                surface,
                0.04,
                visual.local_bounds,
                root.scale.to_array(),
                bevy::math::Mat3::from_quat(root.rotation).to_cols_array(),
            )
        };
    }
}
