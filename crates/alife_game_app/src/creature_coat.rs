//! Deterministic palette selection for editable creature-part authoring.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureCoatPalette {
    pub primary: [u8; 3],
    pub secondary: [u8; 3],
    pub accent: [u8; 3],
    pub iris: [u8; 3],
}

const COAT_PALETTES: [CreatureCoatPalette; 16] = [
    CreatureCoatPalette {
        primary: [226, 58, 42],
        secondary: [62, 22, 88],
        accent: [244, 176, 34],
        iris: [34, 196, 218],
    },
    CreatureCoatPalette {
        primary: [24, 184, 202],
        secondary: [20, 48, 102],
        accent: [238, 82, 48],
        iris: [238, 190, 34],
    },
    CreatureCoatPalette {
        primary: [214, 178, 32],
        secondary: [68, 30, 94],
        accent: [20, 156, 142],
        iris: [34, 96, 224],
    },
    CreatureCoatPalette {
        primary: [196, 48, 138],
        secondary: [48, 28, 104],
        accent: [32, 184, 174],
        iris: [236, 174, 36],
    },
    CreatureCoatPalette {
        primary: [42, 168, 92],
        secondary: [30, 42, 94],
        accent: [232, 74, 52],
        iris: [236, 184, 28],
    },
    CreatureCoatPalette {
        primary: [226, 88, 28],
        secondary: [62, 24, 116],
        accent: [26, 172, 190],
        iris: [220, 204, 36],
    },
    CreatureCoatPalette {
        primary: [60, 112, 226],
        secondary: [78, 22, 74],
        accent: [232, 132, 32],
        iris: [42, 206, 158],
    },
    CreatureCoatPalette {
        primary: [190, 64, 212],
        secondary: [24, 68, 82],
        accent: [224, 174, 36],
        iris: [36, 190, 216],
    },
    CreatureCoatPalette {
        primary: [216, 54, 82],
        secondary: [26, 58, 112],
        accent: [28, 182, 142],
        iris: [238, 180, 32],
    },
    CreatureCoatPalette {
        primary: [28, 178, 154],
        secondary: [86, 26, 76],
        accent: [228, 80, 42],
        iris: [224, 194, 30],
    },
    CreatureCoatPalette {
        primary: [224, 146, 26],
        secondary: [34, 54, 116],
        accent: [188, 42, 136],
        iris: [34, 188, 202],
    },
    CreatureCoatPalette {
        primary: [118, 84, 224],
        secondary: [22, 74, 70],
        accent: [226, 96, 32],
        iris: [218, 190, 30],
    },
    CreatureCoatPalette {
        primary: [202, 72, 44],
        secondary: [32, 82, 96],
        accent: [218, 170, 34],
        iris: [48, 186, 214],
    },
    CreatureCoatPalette {
        primary: [44, 154, 218],
        secondary: [90, 22, 62],
        accent: [226, 138, 28],
        iris: [42, 204, 152],
    },
    CreatureCoatPalette {
        primary: [188, 38, 104],
        secondary: [20, 66, 108],
        accent: [30, 176, 134],
        iris: [230, 184, 30],
    },
    CreatureCoatPalette {
        primary: [74, 184, 52],
        secondary: [72, 24, 96],
        accent: [222, 70, 38],
        iris: [32, 174, 216],
    },
];

pub fn resolve_creature_coat_palette(palette_family: u8) -> CreatureCoatPalette {
    COAT_PALETTES[usize::from(palette_family) % COAT_PALETTES.len()]
}
