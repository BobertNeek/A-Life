#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureVisualBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl CreatureVisualBounds {
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    pub fn include(&mut self, other: Self) {
        for axis in 0..3 {
            self.min[axis] = self.min[axis].min(other.min[axis]);
            self.max[axis] = self.max[axis].max(other.max[axis]);
        }
    }

    pub fn is_valid(self) -> bool {
        self.min.into_iter().chain(self.max).all(f32::is_finite)
            && (0..3).all(|axis| self.min[axis] <= self.max[axis])
    }

    pub fn corners(self) -> [[f32; 3]; 8] {
        [
            [self.min[0], self.min[1], self.min[2]],
            [self.min[0], self.min[1], self.max[2]],
            [self.min[0], self.max[1], self.min[2]],
            [self.min[0], self.max[1], self.max[2]],
            [self.max[0], self.min[1], self.min[2]],
            [self.max[0], self.min[1], self.max[2]],
            [self.max[0], self.max[1], self.min[2]],
            [self.max[0], self.max[1], self.max[2]],
        ]
    }
}

pub fn grounded_root_height(
    surface_height: f32,
    clearance: f32,
    bounds: CreatureVisualBounds,
    scale: [f32; 3],
    rotation_columns: [f32; 9],
) -> f32 {
    if !surface_height.is_finite()
        || !clearance.is_finite()
        || !bounds.is_valid()
        || !scale.into_iter().all(|axis| axis.is_finite() && axis > 0.0)
        || !rotation_columns.into_iter().all(f32::is_finite)
    {
        return surface_height.max(0.0) + clearance.max(0.0);
    }

    let min_y = bounds
        .corners()
        .into_iter()
        .map(|corner| {
            let scaled = [
                corner[0] * scale[0],
                corner[1] * scale[1],
                corner[2] * scale[2],
            ];
            rotation_columns[1] * scaled[0]
                + rotation_columns[4] * scaled[1]
                + rotation_columns[7] * scaled[2]
        })
        .fold(f32::INFINITY, f32::min);
    surface_height + clearance.max(0.0) - min_y
}

#[cfg(test)]
mod tests {
    use super::{grounded_root_height, CreatureVisualBounds};

    const IDENTITY_COLUMNS: [f32; 9] = [
        1.0, 0.0, 0.0, // X column
        0.0, 1.0, 0.0, // Y column
        0.0, 0.0, 1.0, // Z column
    ];

    #[test]
    fn upright_grounding_places_actual_scaled_foot_minimum_on_surface() {
        let bounds = CreatureVisualBounds::new([-0.42, -0.71, -0.24], [0.42, 1.02, 0.35]);
        let surface = 0.64;
        let clearance = 0.012;
        let scale = [1.08, 1.08, 1.08];
        let root_y = grounded_root_height(surface, clearance, bounds, scale, IDENTITY_COLUMNS);

        let world_min_y = root_y + bounds.min[1] * scale[1];
        assert!((world_min_y - (surface + clearance)).abs() <= 1.0e-5);
    }

    #[test]
    fn side_sleeping_grounding_uses_rotated_width_instead_of_upright_height() {
        let bounds = CreatureVisualBounds::new([-0.46, -0.72, -0.25], [0.46, 1.01, 0.34]);
        let quarter_turn_z_columns = [
            0.0, 1.0, 0.0, // X column
            -1.0, 0.0, 0.0, // Y column
            0.0, 0.0, 1.0, // Z column
        ];
        let root_y = grounded_root_height(0.44, 0.012, bounds, [1.0; 3], quarter_turn_z_columns);

        assert!((root_y - (0.44 + 0.012 + 0.46)).abs() <= 1.0e-5);
        assert!(root_y.is_finite());
    }

    #[test]
    fn visual_bounds_merge_rejects_no_axis_and_remains_finite() {
        let mut bounds = CreatureVisualBounds::new([-0.2, -0.4, -0.1], [0.3, 0.5, 0.2]);
        bounds.include(CreatureVisualBounds::new(
            [-0.7, -0.3, -0.5],
            [0.1, 1.1, 0.4],
        ));
        assert_eq!(bounds.min, [-0.7, -0.4, -0.5]);
        assert_eq!(bounds.max, [0.3, 1.1, 0.4]);
        assert!(bounds.is_valid());
    }
}
