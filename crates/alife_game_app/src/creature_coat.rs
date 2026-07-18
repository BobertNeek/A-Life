//! App-local deterministic creature coat baking and bounded display-asset cache.

use std::{cmp::Ordering, collections::BTreeMap};

use alife_world::CreaturePartSources;
use image::RgbaImage;
use thiserror::Error;

use crate::{CreaturePartSlot, ProductionFrontendProfileId};

pub const CREATURE_COAT_ATLAS_SIZE: u32 = 256;
pub const CREATURE_COAT_SOURCE_MASK_SIZE: u32 = 64;
pub const CREATURE_COAT_RGBA_BYTES: usize =
    CREATURE_COAT_ATLAS_SIZE as usize * CREATURE_COAT_ATLAS_SIZE as usize * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureCoatKey {
    pub part_sources: CreaturePartSources,
    pub palette_family: u8,
    pub fur_pattern: u8,
    pub marking_density: u8,
}

impl CreatureCoatKey {
    pub const fn new(
        part_sources: CreaturePartSources,
        palette_family: u8,
        fur_pattern: u8,
        marking_density: u8,
    ) -> Self {
        Self {
            part_sources,
            palette_family,
            fur_pattern,
            marking_density,
        }
    }

    fn ordering_tuple(self) -> (u16, u16, u16, u16, u16, u8, u8, u8) {
        (
            self.part_sources.head.0,
            self.part_sources.torso.0,
            self.part_sources.arms.0,
            self.part_sources.legs.0,
            self.part_sources.tail.0,
            self.palette_family,
            self.fur_pattern,
            self.marking_density,
        )
    }

    fn inherited_coat_distance(self, other: Self) -> u32 {
        u32::from(self.part_sources.head.0.abs_diff(other.part_sources.head.0))
            + u32::from(
                self.part_sources
                    .torso
                    .0
                    .abs_diff(other.part_sources.torso.0),
            )
            + u32::from(self.part_sources.arms.0.abs_diff(other.part_sources.arms.0))
            + u32::from(self.part_sources.legs.0.abs_diff(other.part_sources.legs.0))
            + u32::from(self.part_sources.tail.0.abs_diff(other.part_sources.tail.0))
            + u32::from(self.palette_family.abs_diff(other.palette_family)) * 8
            + u32::from(self.fur_pattern.abs_diff(other.fur_pattern)) * 4
            + u32::from(self.marking_density.abs_diff(other.marking_density))
    }
}

impl Ord for CreatureCoatKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ordering_tuple().cmp(&other.ordering_tuple())
    }
}

impl PartialOrd for CreatureCoatKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureCoatSemanticRegion {
    Head,
    Eyes,
    Lids,
    Hair,
    Teeth,
    Tongue,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    TailBack,
}

impl CreatureCoatSemanticRegion {
    pub const ALL: [Self; 12] = [
        Self::Head,
        Self::Eyes,
        Self::Lids,
        Self::Hair,
        Self::Teeth,
        Self::Tongue,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
        Self::TailBack,
    ];

    pub const fn atlas_cell(self) -> [u32; 2] {
        match self {
            Self::Head => [0, 0],
            Self::Eyes => [1, 0],
            Self::Lids => [2, 0],
            Self::Hair => [3, 0],
            Self::Teeth => [0, 1],
            Self::Tongue => [1, 1],
            Self::Torso => [2, 1],
            Self::LeftArm => [3, 1],
            Self::RightArm => [0, 2],
            Self::LeftLeg => [1, 2],
            Self::RightLeg => [2, 2],
            Self::TailBack => [3, 2],
        }
    }

    pub const fn mask_color(self) -> [u8; 3] {
        match self {
            Self::Head => [230, 92, 88],
            Self::Torso => [64, 166, 184],
            Self::LeftArm | Self::RightArm => [244, 177, 76],
            Self::LeftLeg | Self::RightLeg => [95, 177, 104],
            Self::TailBack => [154, 108, 180],
            Self::Eyes => [238, 238, 224],
            Self::Lids => [184, 80, 96],
            Self::Hair => [114, 84, 145],
            Self::Teeth => [235, 222, 188],
            Self::Tongue => [213, 92, 126],
        }
    }

    fn from_mask_color(color: [u8; 3], atlas_cell: [u32; 2]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|region| region.mask_color() == color && region.atlas_cell() == atlas_cell)
    }

    const fn is_body(self) -> bool {
        matches!(
            self,
            Self::Head
                | Self::Torso
                | Self::LeftArm
                | Self::RightArm
                | Self::LeftLeg
                | Self::RightLeg
                | Self::TailBack
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureCoatAtlasRegion {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    TailBack,
}

impl CreatureCoatAtlasRegion {
    pub const ALL: [Self; 7] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
        Self::TailBack,
    ];

    pub const fn slot(self) -> CreaturePartSlot {
        match self {
            Self::Head => CreaturePartSlot::Head,
            Self::Torso => CreaturePartSlot::Torso,
            Self::LeftArm => CreaturePartSlot::LeftArm,
            Self::RightArm => CreaturePartSlot::RightArm,
            Self::LeftLeg => CreaturePartSlot::LeftLeg,
            Self::RightLeg => CreaturePartSlot::RightLeg,
            Self::TailBack => CreaturePartSlot::TailBack,
        }
    }

    const fn semantic(self) -> CreatureCoatSemanticRegion {
        match self {
            Self::Head => CreatureCoatSemanticRegion::Head,
            Self::Torso => CreatureCoatSemanticRegion::Torso,
            Self::LeftArm => CreatureCoatSemanticRegion::LeftArm,
            Self::RightArm => CreatureCoatSemanticRegion::RightArm,
            Self::LeftLeg => CreatureCoatSemanticRegion::LeftLeg,
            Self::RightLeg => CreatureCoatSemanticRegion::RightLeg,
            Self::TailBack => CreatureCoatSemanticRegion::TailBack,
        }
    }
}

pub const fn join_cover_atlas_region(slot: CreaturePartSlot) -> CreatureCoatAtlasRegion {
    match slot {
        CreaturePartSlot::Head => CreatureCoatAtlasRegion::Head,
        CreaturePartSlot::Torso => CreatureCoatAtlasRegion::Torso,
        CreaturePartSlot::LeftArm => CreatureCoatAtlasRegion::LeftArm,
        CreaturePartSlot::RightArm => CreatureCoatAtlasRegion::RightArm,
        CreaturePartSlot::LeftLeg => CreatureCoatAtlasRegion::LeftLeg,
        CreaturePartSlot::RightLeg => CreatureCoatAtlasRegion::RightLeg,
        CreaturePartSlot::TailBack => CreatureCoatAtlasRegion::TailBack,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureCoatSourceInput {
    pub semantic_microdetail: RgbaImage,
    pub anatomy: RgbaImage,
}

impl CreatureCoatSourceInput {
    pub const fn new(semantic_microdetail: RgbaImage, anatomy: RgbaImage) -> Self {
        Self {
            semantic_microdetail,
            anatomy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureCoatSourceMasks {
    pub head: CreatureCoatSourceInput,
    pub torso: CreatureCoatSourceInput,
    pub arms: CreatureCoatSourceInput,
    pub legs: CreatureCoatSourceInput,
    pub tail: CreatureCoatSourceInput,
}

impl CreatureCoatSourceMasks {
    pub fn new(
        head: CreatureCoatSourceInput,
        torso: CreatureCoatSourceInput,
        arms: CreatureCoatSourceInput,
        legs: CreatureCoatSourceInput,
        tail: CreatureCoatSourceInput,
    ) -> Result<Self, CreatureCoatError> {
        for (label, input) in [
            ("head", &head),
            ("torso", &torso),
            ("arms", &arms),
            ("legs", &legs),
            ("tail", &tail),
        ] {
            validate_source_input_dimensions(label, input)?;
        }
        Ok(Self {
            head,
            torso,
            arms,
            legs,
            tail,
        })
    }

    fn iter(&self) -> [(&'static str, &CreatureCoatSourceInput); 5] {
        [
            ("head", &self.head),
            ("torso", &self.torso),
            ("arms", &self.arms),
            ("legs", &self.legs),
            ("tail", &self.tail),
        ]
    }
}

fn validate_source_input_dimensions(
    label: &'static str,
    input: &CreatureCoatSourceInput,
) -> Result<(), CreatureCoatError> {
    for (map_kind, mask) in [
        (
            CreatureCoatSourceMapKind::SemanticMicrodetail,
            &input.semantic_microdetail,
        ),
        (CreatureCoatSourceMapKind::Anatomy, &input.anatomy),
    ] {
        if mask.dimensions()
            != (
                CREATURE_COAT_SOURCE_MASK_SIZE,
                CREATURE_COAT_SOURCE_MASK_SIZE,
            )
        {
            return Err(CreatureCoatError::InvalidMaskDimensions {
                label,
                map_kind,
                width: mask.width(),
                height: mask.height(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureCoatPalette {
    pub primary: [u8; 3],
    pub secondary: [u8; 3],
    pub accent: [u8; 3],
    pub iris: [u8; 3],
}

impl CreatureCoatPalette {
    pub fn primary_secondary_value_contrast(self) -> f32 {
        let value = |color: [u8; 3]| f32::from(*color.iter().max().unwrap_or(&0)) / 255.0;
        (value(self.primary) - value(self.secondary)).abs()
    }

    pub fn minimum_saturation(self) -> f32 {
        let saturation = |color: [u8; 3]| {
            let maximum = f32::from(*color.iter().max().unwrap_or(&0));
            let minimum = f32::from(*color.iter().min().unwrap_or(&0));
            if maximum <= f32::EPSILON {
                0.0
            } else {
                (maximum - minimum) / maximum
            }
        };
        saturation(self.primary).min(saturation(self.secondary))
    }
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

pub fn resolve_creature_coat_palette(key: CreatureCoatKey) -> CreatureCoatPalette {
    COAT_PALETTES[usize::from(key.palette_family) % COAT_PALETTES.len()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureCoatAtlas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub palette: CreatureCoatPalette,
    populated_regions: [bool; 7],
    channel_pixel_counts: [usize; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureCoatAnatomicalChannel {
    Primary,
    Belly,
    Muzzle,
    InnerEar,
    HandsFeet,
    KeratinSkin,
    SecondaryMarking,
}

impl CreatureCoatAnatomicalChannel {
    pub const ALL: [Self; 7] = [
        Self::Primary,
        Self::Belly,
        Self::Muzzle,
        Self::InnerEar,
        Self::HandsFeet,
        Self::KeratinSkin,
        Self::SecondaryMarking,
    ];

    pub const fn mask_color(self) -> [u8; 3] {
        match self {
            Self::Primary => [248, 248, 248],
            Self::Belly => [232, 176, 72],
            Self::Muzzle => [226, 112, 128],
            Self::InnerEar => [238, 86, 154],
            Self::HandsFeet => [72, 174, 218],
            Self::KeratinSkin => [64, 52, 72],
            Self::SecondaryMarking => [84, 92, 214],
        }
    }

    fn from_mask_color(color: [u8; 3]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|channel| channel.mask_color() == color)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureCoatSourceMapKind {
    SemanticMicrodetail,
    Anatomy,
}

impl CreatureCoatAtlas {
    pub fn region_has_nonzero_color(&self, region: CreatureCoatAtlasRegion) -> bool {
        self.populated_regions[region_index(region)]
    }

    pub fn channel_pixel_count(&self, channel: CreatureCoatAnatomicalChannel) -> usize {
        self.channel_pixel_counts[channel as usize]
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CreatureCoatError {
    #[error("{label} {map_kind:?} mask must be 64x64 RGBA8, got {width}x{height}")]
    InvalidMaskDimensions {
        label: &'static str,
        map_kind: CreatureCoatSourceMapKind,
        width: u32,
        height: u32,
    },
    #[error("{label} semantic mask contains unknown occupied color {color:?} at {x},{y}")]
    UnknownSemanticColor {
        label: &'static str,
        color: [u8; 3],
        x: u32,
        y: u32,
    },
    #[error("{label} semantic mask contains region {region:?} owned by another source")]
    SemanticRegionOwnedByWrongSource {
        label: &'static str,
        region: CreatureCoatSemanticRegion,
    },
    #[error("{label} semantic mask is missing required region {region:?}")]
    MissingSourceSemanticRegion {
        label: &'static str,
        region: CreatureCoatSemanticRegion,
    },
    #[error(
        "{label} anatomy occupancy differs from semantic/microdetail at {x},{y}: semantic={semantic_occupied}, anatomy={anatomy_occupied}"
    )]
    AnatomyOccupancyMismatch {
        label: &'static str,
        x: u32,
        y: u32,
        semantic_occupied: bool,
        anatomy_occupied: bool,
    },
    #[error("{label} anatomy mask contains unknown occupied color {color:?} at {x},{y}")]
    UnknownAnatomyColor {
        label: &'static str,
        color: [u8; 3],
        x: u32,
        y: u32,
    },
    #[error("{label} anatomy mask contains channel {channel:?} owned by another source")]
    AnatomicalChannelOwnedByWrongSource {
        label: &'static str,
        channel: CreatureCoatAnatomicalChannel,
    },
    #[error("creature coat input is missing anatomical channel {0:?}")]
    MissingAnatomicalChannel(CreatureCoatAnatomicalChannel),
    #[error("creature coat input is missing required semantic region {0:?}")]
    MissingSemanticRegion(CreatureCoatAtlasRegion),
}

pub fn bake_creature_coat(
    key: CreatureCoatKey,
    masks: &CreatureCoatSourceMasks,
) -> Result<CreatureCoatAtlas, CreatureCoatError> {
    for (label, input) in masks.iter() {
        validate_source_input_dimensions(label, input)?;
    }
    let mut semantic_pixels: Vec<Option<(CreatureCoatSemanticRegion, u8)>> =
        vec![None; (CREATURE_COAT_SOURCE_MASK_SIZE.pow(2)) as usize];
    let mut anatomy_pixels: Vec<Option<(CreatureCoatAnatomicalChannel, u8)>> =
        vec![None; (CREATURE_COAT_SOURCE_MASK_SIZE.pow(2)) as usize];
    let mut found_anatomical_channels = [false; 7];
    for (label, input) in masks.iter() {
        let mask = &input.semantic_microdetail;
        let required = required_regions_for_source(label);
        let mut found = BTreeMap::new();
        for (x, y, pixel) in mask.enumerate_pixels() {
            let anatomy = input.anatomy.get_pixel(x, y);
            let semantic_occupied = pixel[3] > 0;
            let anatomy_occupied = anatomy[3] > 0;
            if semantic_occupied != anatomy_occupied {
                return Err(CreatureCoatError::AnatomyOccupancyMismatch {
                    label,
                    x,
                    y,
                    semantic_occupied,
                    anatomy_occupied,
                });
            }
            if !semantic_occupied {
                continue;
            }
            let color = [pixel[0], pixel[1], pixel[2]];
            let cell = [
                (x * 2 + 1) * 4 / (CREATURE_COAT_SOURCE_MASK_SIZE * 2),
                (y * 2 + 1) * 3 / (CREATURE_COAT_SOURCE_MASK_SIZE * 2),
            ];
            let semantic = CreatureCoatSemanticRegion::from_mask_color(color, cell)
                .ok_or(CreatureCoatError::UnknownSemanticColor { label, color, x, y })?;
            if !required.contains(&semantic) {
                return Err(CreatureCoatError::SemanticRegionOwnedByWrongSource {
                    label,
                    region: semantic,
                });
            }
            found.insert(semantic, ());
            let index = (y * CREATURE_COAT_SOURCE_MASK_SIZE + x) as usize;
            let anatomy_color = [anatomy[0], anatomy[1], anatomy[2]];
            let channel = CreatureCoatAnatomicalChannel::from_mask_color(anatomy_color).ok_or(
                CreatureCoatError::UnknownAnatomyColor {
                    label,
                    color: anatomy_color,
                    x,
                    y,
                },
            )?;
            if !allowed_anatomical_channels_for_source(label).contains(&channel) {
                return Err(CreatureCoatError::AnatomicalChannelOwnedByWrongSource {
                    label,
                    channel,
                });
            }
            if semantic.is_body() {
                found_anatomical_channels[channel as usize] = true;
            }
            anatomy_pixels[index] = Some((channel, anatomy[3]));
            match semantic_pixels[index] {
                Some((existing, detail)) if existing == semantic => {
                    semantic_pixels[index] = Some((semantic, detail.max(pixel[3])));
                }
                Some(_) => {
                    return Err(CreatureCoatError::UnknownSemanticColor { label, color, x, y });
                }
                None => semantic_pixels[index] = Some((semantic, pixel[3])),
            }
        }
        if let Some(region) = required
            .into_iter()
            .find(|region| !found.contains_key(region))
        {
            return Err(CreatureCoatError::MissingSourceSemanticRegion { label, region });
        }
    }

    for channel in CreatureCoatAnatomicalChannel::ALL {
        if !found_anatomical_channels[channel as usize] {
            return Err(CreatureCoatError::MissingAnatomicalChannel(channel));
        }
    }

    let mut populated_regions = [false; 7];
    for region in CreatureCoatAtlasRegion::ALL {
        if semantic_pixels
            .iter()
            .flatten()
            .any(|(semantic, _)| *semantic == region.semantic())
        {
            populated_regions[region_index(region)] = true;
        } else {
            return Err(CreatureCoatError::MissingSemanticRegion(region));
        }
    }

    let palette = resolve_creature_coat_palette(key);
    let mut rgba = vec![0_u8; CREATURE_COAT_RGBA_BYTES];
    let mut channel_pixel_counts = [0; 7];
    for y in 0..CREATURE_COAT_ATLAS_SIZE {
        for x in 0..CREATURE_COAT_ATLAS_SIZE {
            let source_x = x * CREATURE_COAT_SOURCE_MASK_SIZE / CREATURE_COAT_ATLAS_SIZE;
            let source_y = y * CREATURE_COAT_SOURCE_MASK_SIZE / CREATURE_COAT_ATLAS_SIZE;
            let source_index = (source_y * CREATURE_COAT_SOURCE_MASK_SIZE + source_x) as usize;
            let rgb = if let Some((semantic, detail)) = semantic_pixels[source_index] {
                let authored_anatomy = anatomy_pixels[source_index];
                let (rgb, channel) =
                    coat_pixel(key, palette, semantic, detail, authored_anatomy, x, y);
                if let Some(channel) = channel {
                    channel_pixel_counts[channel as usize] += 1;
                }
                rgb
            } else {
                shade_color(palette.secondary, 92)
            };
            let offset = ((y * CREATURE_COAT_ATLAS_SIZE + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }

    Ok(CreatureCoatAtlas {
        width: CREATURE_COAT_ATLAS_SIZE,
        height: CREATURE_COAT_ATLAS_SIZE,
        rgba,
        palette,
        populated_regions,
        channel_pixel_counts,
    })
}

fn required_regions_for_source(label: &'static str) -> Vec<CreatureCoatSemanticRegion> {
    match label {
        "head" => vec![
            CreatureCoatSemanticRegion::Head,
            CreatureCoatSemanticRegion::Eyes,
            CreatureCoatSemanticRegion::Lids,
            CreatureCoatSemanticRegion::Hair,
            CreatureCoatSemanticRegion::Teeth,
            CreatureCoatSemanticRegion::Tongue,
        ],
        "torso" => vec![CreatureCoatSemanticRegion::Torso],
        "arms" => vec![
            CreatureCoatSemanticRegion::LeftArm,
            CreatureCoatSemanticRegion::RightArm,
        ],
        "legs" => vec![
            CreatureCoatSemanticRegion::LeftLeg,
            CreatureCoatSemanticRegion::RightLeg,
        ],
        "tail" => vec![CreatureCoatSemanticRegion::TailBack],
        _ => Vec::new(),
    }
}

fn allowed_anatomical_channels_for_source(
    label: &'static str,
) -> &'static [CreatureCoatAnatomicalChannel] {
    use CreatureCoatAnatomicalChannel as Channel;
    match label {
        "head" => &[
            Channel::Primary,
            Channel::Muzzle,
            Channel::InnerEar,
            Channel::KeratinSkin,
            Channel::SecondaryMarking,
        ],
        "torso" => &[
            Channel::Primary,
            Channel::Belly,
            Channel::KeratinSkin,
            Channel::SecondaryMarking,
        ],
        "arms" | "legs" => &[
            Channel::Primary,
            Channel::HandsFeet,
            Channel::KeratinSkin,
            Channel::SecondaryMarking,
        ],
        "tail" => &[
            Channel::Primary,
            Channel::KeratinSkin,
            Channel::SecondaryMarking,
        ],
        _ => &[],
    }
}

fn region_index(region: CreatureCoatAtlasRegion) -> usize {
    match region {
        CreatureCoatAtlasRegion::Head => 0,
        CreatureCoatAtlasRegion::Torso => 1,
        CreatureCoatAtlasRegion::LeftArm => 2,
        CreatureCoatAtlasRegion::RightArm => 3,
        CreatureCoatAtlasRegion::LeftLeg => 4,
        CreatureCoatAtlasRegion::RightLeg => 5,
        CreatureCoatAtlasRegion::TailBack => 6,
    }
}

fn coat_pixel(
    key: CreatureCoatKey,
    palette: CreatureCoatPalette,
    semantic: CreatureCoatSemanticRegion,
    detail: u8,
    authored_anatomy: Option<(CreatureCoatAnatomicalChannel, u8)>,
    x: u32,
    y: u32,
) -> ([u8; 3], Option<CreatureCoatAnatomicalChannel>) {
    match semantic {
        CreatureCoatSemanticRegion::Eyes => (eye_pixel(palette, x, y), None),
        CreatureCoatSemanticRegion::Teeth => (shade_color([238, 218, 172], detail), None),
        CreatureCoatSemanticRegion::Tongue => (shade_color([196, 48, 82], detail), None),
        CreatureCoatSemanticRegion::Hair => (shade_color(palette.accent, detail), None),
        CreatureCoatSemanticRegion::Lids => (shade_color(palette.secondary, detail), None),
        _ => {
            let (authored_channel, strength) =
                authored_anatomy.expect("validated body pixels have authored anatomy");
            let channel = if authored_channel == CreatureCoatAnatomicalChannel::SecondaryMarking
                && !uses_secondary_marking(key, semantic, x, y)
            {
                CreatureCoatAnatomicalChannel::Primary
            } else {
                authored_channel
            };
            let target = anatomical_channel_color(channel, palette);
            let color = if channel == CreatureCoatAnatomicalChannel::Primary {
                target
            } else {
                mix_color(palette.primary, target, u32::from(strength) * 100 / 255)
            };
            (shade_color(color, detail), Some(channel))
        }
    }
}

fn anatomical_channel_color(
    channel: CreatureCoatAnatomicalChannel,
    palette: CreatureCoatPalette,
) -> [u8; 3] {
    match channel {
        CreatureCoatAnatomicalChannel::Primary => palette.primary,
        CreatureCoatAnatomicalChannel::Belly => mix_color(palette.primary, [244, 220, 174], 42),
        CreatureCoatAnatomicalChannel::Muzzle => mix_color(palette.primary, [226, 158, 146], 52),
        CreatureCoatAnatomicalChannel::InnerEar => mix_color(palette.accent, [232, 116, 142], 52),
        CreatureCoatAnatomicalChannel::HandsFeet => palette.accent,
        CreatureCoatAnatomicalChannel::KeratinSkin => {
            mix_color(palette.secondary, [54, 44, 50], 42)
        }
        CreatureCoatAnatomicalChannel::SecondaryMarking => palette.secondary,
    }
}

fn mix_color(first: [u8; 3], second: [u8; 3], second_weight: u32) -> [u8; 3] {
    std::array::from_fn(|index| {
        ((u32::from(first[index]) * (100 - second_weight)
            + u32::from(second[index]) * second_weight)
            / 100) as u8
    })
}

fn eye_pixel(palette: CreatureCoatPalette, x: u32, y: u32) -> [u8; 3] {
    let local_x = i32::try_from(x % 64).unwrap_or_default() - 32;
    let local_y = i32::try_from(y % 85).unwrap_or_default() - 42;
    let radial = local_x * local_x * 2 + local_y * local_y;
    if (local_x + 7).pow(2) + (local_y + 8).pow(2) < 10 {
        [255, 248, 216]
    } else if radial < 120 {
        [26, 20, 30]
    } else if radial < 560 {
        palette.iris
    } else {
        [236, 216, 178]
    }
}

fn uses_secondary_marking(
    key: CreatureCoatKey,
    semantic: CreatureCoatSemanticRegion,
    x: u32,
    y: u32,
) -> bool {
    let region_seed = semantic as u32 * 0x9e37;
    let source_seed = u32::from(key.part_sources.head.0)
        .wrapping_mul(3)
        .wrapping_add(u32::from(key.part_sources.torso.0).wrapping_mul(5))
        .wrapping_add(u32::from(key.part_sources.arms.0).wrapping_mul(7))
        .wrapping_add(u32::from(key.part_sources.legs.0).wrapping_mul(11))
        .wrapping_add(u32::from(key.part_sources.tail.0).wrapping_mul(13));
    let phase = source_seed
        .wrapping_add(u32::from(key.fur_pattern) * 29)
        .wrapping_add(region_seed);
    let hash = pixel_hash(x, y, phase);
    let density = 24 + u32::from(key.marking_density) * 12;
    match key.fur_pattern % 10 {
        0 => hash & 0xff < density,
        1 => hash & 0xff < density && ((x / 9 + y / 7 + phase) & 1 == 0),
        2 => ((x + y / 3 + phase) % 38) < 8 + density / 32,
        3 => ((x * 2 + y + phase) % 28) < 3 + density / 42,
        4 => y % 64 > 28_u32.saturating_sub(density / 16),
        5 => ((x + phase) % 64).abs_diff(32) < 6 + density / 28,
        6 => ((x + y + phase) % 96) < 22 + density / 6,
        7 => ((x + y + phase) % 32) < 4 + density / 12,
        8 => (hash ^ (hash >> 9)) & 0xff < density,
        _ => {
            let coarse = pixel_hash(x / 6, y / 6, phase) & 0xff;
            let ring = pixel_hash(x / 3, y / 3, phase ^ 0xa5a5) & 0xff;
            coarse < density && ring > 68
        }
    }
}

fn pixel_hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut value = x
        .wrapping_mul(0x85eb_ca6b)
        .wrapping_add(y.wrapping_mul(0xc2b2_ae35))
        .wrapping_add(seed);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value.wrapping_mul(0x846c_a68b) ^ (value >> 16)
}

fn shade_color(color: [u8; 3], detail: u8) -> [u8; 3] {
    let factor = 196_u32 + u32::from(detail) * 68 / 255;
    color.map(|channel| ((u32::from(channel) * factor / 230).min(255)) as u8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureCoatCacheLimits {
    pub max_entries: usize,
    pub max_rgba_bytes: usize,
}

impl CreatureCoatCacheLimits {
    pub const fn minimum() -> Self {
        Self {
            max_entries: 48,
            max_rgba_bytes: 12 * 1024 * 1024,
        }
    }

    pub const fn comfort() -> Self {
        Self {
            max_entries: 96,
            max_rgba_bytes: 24 * 1024 * 1024,
        }
    }

    pub const fn future_scale_up() -> Self {
        Self {
            max_entries: 256,
            max_rgba_bytes: 64 * 1024 * 1024,
        }
    }

    pub const fn for_profile(profile: ProductionFrontendProfileId) -> Self {
        match profile {
            ProductionFrontendProfileId::MinimumSettings30x30 => Self::minimum(),
            ProductionFrontendProfileId::MinSpecComfort1080p => Self::comfort(),
            ProductionFrontendProfileId::Balanced1080p
            | ProductionFrontendProfileId::HighSpecScaleUp
            | ProductionFrontendProfileId::ResearchScale => Self::future_scale_up(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreatureCoatAssetPair {
    pub image_id: u64,
    pub material_id: u64,
}

impl CreatureCoatAssetPair {
    pub const fn new(image_id: u64, material_id: u64) -> Self {
        Self {
            image_id,
            material_id,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CreatureCoatCacheEntry {
    pair: CreatureCoatAssetPair,
    last_used: u64,
    pin_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureCoatCacheUpdate {
    pub selected_key: CreatureCoatKey,
    pub selected: CreatureCoatAssetPair,
    pub inserted: bool,
    pub used_pinned_fallback: bool,
    pub evicted: Vec<CreatureCoatAssetPair>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CreatureCoatCacheError {
    #[error("creature coat cache limits cannot hold one exact atlas: {limits:?}")]
    InvalidLimits { limits: CreatureCoatCacheLimits },
    #[error("creature coat key is not resident")]
    MissingKey,
    #[error("creature coat release is unbalanced")]
    UnbalancedRelease,
}

#[derive(Debug)]
pub struct CreatureCoatCache {
    limits: CreatureCoatCacheLimits,
    entries: BTreeMap<CreatureCoatKey, CreatureCoatCacheEntry>,
    rgba_bytes: usize,
    clock: u64,
}

impl CreatureCoatCache {
    pub fn new(limits: CreatureCoatCacheLimits) -> Result<Self, CreatureCoatCacheError> {
        if limits.max_entries == 0 || limits.max_rgba_bytes < CREATURE_COAT_RGBA_BYTES {
            return Err(CreatureCoatCacheError::InvalidLimits { limits });
        }
        Ok(Self {
            limits,
            entries: BTreeMap::new(),
            rgba_bytes: 0,
            clock: 0,
        })
    }

    fn insert(
        &mut self,
        key: CreatureCoatKey,
        pair: CreatureCoatAssetPair,
    ) -> CreatureCoatCacheUpdate {
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.clock;
            return CreatureCoatCacheUpdate {
                selected_key: key,
                selected: entry.pair,
                inserted: false,
                used_pinned_fallback: false,
                evicted: Vec::new(),
            };
        }

        let count_overflow = self.entries.len().saturating_add(1) > self.limits.max_entries;
        let byte_overflow =
            self.rgba_bytes.saturating_add(CREATURE_COAT_RGBA_BYTES) > self.limits.max_rgba_bytes;
        let mut eviction_keys = Vec::new();
        if count_overflow || byte_overflow {
            let mut candidates = self
                .entries
                .iter()
                .filter(|(_, entry)| entry.pin_count == 0)
                .map(|(candidate_key, entry)| (*candidate_key, entry.last_used))
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(candidate_key, last_used)| (*last_used, *candidate_key));
            let mut remaining_count = self.entries.len();
            let mut remaining_bytes = self.rgba_bytes;
            for (candidate_key, _) in candidates {
                if remaining_count.saturating_add(1) <= self.limits.max_entries
                    && remaining_bytes.saturating_add(CREATURE_COAT_RGBA_BYTES)
                        <= self.limits.max_rgba_bytes
                {
                    break;
                }
                eviction_keys.push(candidate_key);
                remaining_count = remaining_count.saturating_sub(1);
                remaining_bytes = remaining_bytes.saturating_sub(CREATURE_COAT_RGBA_BYTES);
            }
            if remaining_count.saturating_add(1) > self.limits.max_entries
                || remaining_bytes.saturating_add(CREATURE_COAT_RGBA_BYTES)
                    > self.limits.max_rgba_bytes
            {
                let (fallback_key, fallback_pair) = self
                    .nearest_pair(key)
                    .expect("a full valid cache always has a resident fallback");
                let entry = self
                    .entries
                    .get_mut(&fallback_key)
                    .expect("the selected fallback is resident");
                entry.last_used = self.clock;
                return CreatureCoatCacheUpdate {
                    selected_key: fallback_key,
                    selected: fallback_pair,
                    inserted: false,
                    used_pinned_fallback: true,
                    evicted: Vec::new(),
                };
            }
        }

        let mut evicted = Vec::with_capacity(eviction_keys.len());
        for eviction_key in eviction_keys {
            if let Some(entry) = self.entries.remove(&eviction_key) {
                self.rgba_bytes = self.rgba_bytes.saturating_sub(CREATURE_COAT_RGBA_BYTES);
                evicted.push(entry.pair);
            }
        }
        self.rgba_bytes = self.rgba_bytes.saturating_add(CREATURE_COAT_RGBA_BYTES);
        self.entries.insert(
            key,
            CreatureCoatCacheEntry {
                pair,
                last_used: self.clock,
                pin_count: 0,
            },
        );
        CreatureCoatCacheUpdate {
            selected_key: key,
            selected: pair,
            inserted: true,
            used_pinned_fallback: false,
            evicted,
        }
    }

    pub fn acquire(
        &mut self,
        key: CreatureCoatKey,
        pair: CreatureCoatAssetPair,
    ) -> CreatureCoatCacheUpdate {
        let update = self.insert(key, pair);
        let entry = self
            .entries
            .get_mut(&update.selected_key)
            .expect("cache acquisition must select a resident coat");
        entry.pin_count = entry.pin_count.saturating_add(1);
        update
    }

    pub fn release(&mut self, key: CreatureCoatKey) -> Result<(), CreatureCoatCacheError> {
        let entry = self
            .entries
            .get_mut(&key)
            .ok_or(CreatureCoatCacheError::MissingKey)?;
        if entry.pin_count == 0 {
            return Err(CreatureCoatCacheError::UnbalancedRelease);
        }
        entry.pin_count -= 1;
        Ok(())
    }

    pub fn pin_count(&self, key: CreatureCoatKey) -> Option<usize> {
        self.entries.get(&key).map(|entry| entry.pin_count)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub const fn limits(&self) -> CreatureCoatCacheLimits {
        self.limits
    }

    pub const fn rgba_bytes(&self) -> usize {
        self.rgba_bytes
    }

    pub fn contains_pair(&self, pair: CreatureCoatAssetPair) -> bool {
        self.entries.values().any(|entry| entry.pair == pair)
    }

    fn nearest_pair(
        &self,
        key: CreatureCoatKey,
    ) -> Option<(CreatureCoatKey, CreatureCoatAssetPair)> {
        self.entries
            .iter()
            .min_by_key(|(candidate_key, _)| {
                (
                    key.inherited_coat_distance(**candidate_key),
                    **candidate_key,
                )
            })
            .map(|(candidate_key, entry)| (*candidate_key, entry.pair))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use alife_world::{CreaturePartFamilyId, CreaturePartSources};
    use image::{Rgba, RgbaImage};

    use super::*;

    fn source_ids(offset: u16) -> CreaturePartSources {
        CreaturePartSources {
            head: CreaturePartFamilyId(offset),
            torso: CreaturePartFamilyId(offset + 1),
            arms: CreaturePartFamilyId(offset + 2),
            legs: CreaturePartFamilyId(offset + 3),
            tail: CreaturePartFamilyId(offset + 4),
        }
    }

    fn coat_key() -> CreatureCoatKey {
        CreatureCoatKey::new(source_ids(1), 4, 7, 11)
    }

    fn paint_region(image: &mut RgbaImage, region: CreatureCoatSemanticRegion, seed: u8) {
        let [column, row] = region.atlas_cell();
        let color = region.mask_color();
        let y_start = (row * CREATURE_COAT_SOURCE_MASK_SIZE + 1) / 3;
        let y_end = ((row + 1) * CREATURE_COAT_SOURCE_MASK_SIZE + 1) / 3;
        for y in y_start..y_end {
            for x in column * 16..(column + 1) * 16 {
                let detail =
                    48_u8.saturating_add(((x * 11 + y * 17 + u32::from(seed)) % 192) as u8);
                image.put_pixel(x, y, Rgba([color[0], color[1], color[2], detail]));
            }
        }
    }

    fn authored_anatomy(semantic_microdetail: &RgbaImage) -> RgbaImage {
        let mut anatomy = RgbaImage::new(64, 64);
        let primary = CreatureCoatAnatomicalChannel::Primary.mask_color();
        for (x, y, semantic) in semantic_microdetail.enumerate_pixels() {
            if semantic[3] > 0 {
                anatomy.put_pixel(x, y, Rgba([primary[0], primary[1], primary[2], 255]));
            }
        }
        anatomy
    }

    fn paint_anatomy_rect(
        anatomy: &mut RgbaImage,
        channel: CreatureCoatAnatomicalChannel,
        x_range: std::ops::Range<u32>,
        y_range: std::ops::Range<u32>,
    ) {
        let color = channel.mask_color();
        for y in y_range {
            for x in x_range.clone() {
                anatomy.put_pixel(x, y, Rgba([color[0], color[1], color[2], 224]));
            }
        }
    }

    fn source_masks() -> CreatureCoatSourceMasks {
        let mut head = RgbaImage::new(64, 64);
        for (index, region) in [
            CreatureCoatSemanticRegion::Head,
            CreatureCoatSemanticRegion::Eyes,
            CreatureCoatSemanticRegion::Lids,
            CreatureCoatSemanticRegion::Hair,
            CreatureCoatSemanticRegion::Teeth,
            CreatureCoatSemanticRegion::Tongue,
        ]
        .into_iter()
        .enumerate()
        {
            paint_region(&mut head, region, index as u8);
        }

        let mut head_anatomy = authored_anatomy(&head);
        paint_anatomy_rect(
            &mut head_anatomy,
            CreatureCoatAnatomicalChannel::Muzzle,
            4..8,
            9..13,
        );
        paint_anatomy_rect(
            &mut head_anatomy,
            CreatureCoatAnatomicalChannel::InnerEar,
            0..4,
            0..4,
        );
        paint_anatomy_rect(
            &mut head_anatomy,
            CreatureCoatAnatomicalChannel::KeratinSkin,
            12..16,
            0..4,
        );
        paint_anatomy_rect(
            &mut head_anatomy,
            CreatureCoatAnatomicalChannel::SecondaryMarking,
            8..12,
            0..4,
        );

        let mut torso = RgbaImage::new(64, 64);
        paint_region(&mut torso, CreatureCoatSemanticRegion::Torso, 19);
        let mut torso_anatomy = authored_anatomy(&torso);
        paint_anatomy_rect(
            &mut torso_anatomy,
            CreatureCoatAnatomicalChannel::Belly,
            35..41,
            27..33,
        );

        let mut arms = RgbaImage::new(64, 64);
        paint_region(&mut arms, CreatureCoatSemanticRegion::LeftArm, 23);
        paint_region(&mut arms, CreatureCoatSemanticRegion::RightArm, 29);
        let mut arms_anatomy = authored_anatomy(&arms);
        paint_anatomy_rect(
            &mut arms_anatomy,
            CreatureCoatAnatomicalChannel::HandsFeet,
            52..56,
            35..39,
        );

        let mut legs = RgbaImage::new(64, 64);
        paint_region(&mut legs, CreatureCoatSemanticRegion::LeftLeg, 31);
        paint_region(&mut legs, CreatureCoatSemanticRegion::RightLeg, 37);
        let mut legs_anatomy = authored_anatomy(&legs);
        paint_anatomy_rect(
            &mut legs_anatomy,
            CreatureCoatAnatomicalChannel::HandsFeet,
            20..24,
            55..59,
        );

        let mut tail = RgbaImage::new(64, 64);
        paint_region(&mut tail, CreatureCoatSemanticRegion::TailBack, 41);
        let tail_anatomy = authored_anatomy(&tail);

        CreatureCoatSourceMasks::new(
            CreatureCoatSourceInput::new(head, head_anatomy),
            CreatureCoatSourceInput::new(torso, torso_anatomy),
            CreatureCoatSourceInput::new(arms, arms_anatomy),
            CreatureCoatSourceInput::new(legs, legs_anatomy),
            CreatureCoatSourceInput::new(tail, tail_anatomy),
        )
        .unwrap()
    }

    #[test]
    fn every_source_input_requires_an_authored_anatomy_companion() {
        let masks = source_masks();

        for (_, input) in masks.iter() {
            assert_eq!(input.semantic_microdetail.dimensions(), (64, 64));
            assert_eq!(input.anatomy.dimensions(), (64, 64));
        }
    }

    fn atlas_rgb_for_source_pixel(atlas: &CreatureCoatAtlas, x: u32, y: u32) -> [u8; 3] {
        let atlas_x = x * 4 + 2;
        let atlas_y = y * 4 + 2;
        let offset = ((atlas_y * CREATURE_COAT_ATLAS_SIZE + atlas_x) * 4) as usize;
        [
            atlas.rgba[offset],
            atlas.rgba[offset + 1],
            atlas.rgba[offset + 2],
        ]
    }

    #[test]
    fn moving_authored_anatomy_pixels_moves_the_baked_channel() {
        let first_masks = source_masks();
        let mut moved_masks = first_masks.clone();
        let muzzle_position = (4, 9);
        let primary_position = (14, 14);
        let muzzle = *moved_masks
            .head
            .anatomy
            .get_pixel(muzzle_position.0, muzzle_position.1);
        let primary = *moved_masks
            .head
            .anatomy
            .get_pixel(primary_position.0, primary_position.1);
        moved_masks
            .head
            .anatomy
            .put_pixel(muzzle_position.0, muzzle_position.1, primary);
        moved_masks
            .head
            .anatomy
            .put_pixel(primary_position.0, primary_position.1, muzzle);

        let first = bake_creature_coat(coat_key(), &first_masks).unwrap();
        let moved = bake_creature_coat(coat_key(), &moved_masks).unwrap();

        assert_ne!(first.rgba, moved.rgba);
        assert_ne!(
            atlas_rgb_for_source_pixel(&first, muzzle_position.0, muzzle_position.1),
            atlas_rgb_for_source_pixel(&moved, muzzle_position.0, muzzle_position.1)
        );
        assert_ne!(
            atlas_rgb_for_source_pixel(&first, primary_position.0, primary_position.1),
            atlas_rgb_for_source_pixel(&moved, primary_position.0, primary_position.1)
        );
    }

    #[test]
    fn anatomy_occupancy_must_match_semantic_microdetail_occupancy() {
        let mut masks = source_masks();
        masks.head.anatomy.put_pixel(14, 14, Rgba([0, 0, 0, 0]));

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::AnatomyOccupancyMismatch {
                label: "head",
                x: 14,
                y: 14,
                semantic_occupied: true,
                anatomy_occupied: false,
            })
        );
    }

    #[test]
    fn occupied_anatomy_rejects_unknown_channel_colors() {
        let mut masks = source_masks();
        masks.head.anatomy.put_pixel(14, 14, Rgba([1, 2, 3, 255]));

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::UnknownAnatomyColor {
                label: "head",
                color: [1, 2, 3],
                x: 14,
                y: 14,
            })
        );
    }

    #[test]
    fn anatomy_channels_reject_foreign_source_ownership() {
        let mut masks = source_masks();
        let belly = CreatureCoatAnatomicalChannel::Belly.mask_color();
        masks
            .arms
            .anatomy
            .put_pixel(52, 35, Rgba([belly[0], belly[1], belly[2], 255]));

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::AnatomicalChannelOwnedByWrongSource {
                label: "arms",
                channel: CreatureCoatAnatomicalChannel::Belly,
            })
        );
    }

    #[test]
    fn assembled_inputs_require_every_anatomical_channel() {
        let mut masks = source_masks();
        let belly = CreatureCoatAnatomicalChannel::Belly.mask_color();
        let primary = CreatureCoatAnatomicalChannel::Primary.mask_color();
        for pixel in masks.torso.anatomy.pixels_mut() {
            if [pixel[0], pixel[1], pixel[2]] == belly {
                *pixel = Rgba([primary[0], primary[1], primary[2], pixel[3]]);
            }
        }

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::MissingAnatomicalChannel(
                CreatureCoatAnatomicalChannel::Belly
            ))
        );
    }

    #[test]
    fn bake_revalidates_authored_anatomy_dimensions() {
        let mut masks = source_masks();
        masks.tail.anatomy = RgbaImage::new(63, 64);

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::InvalidMaskDimensions {
                label: "tail",
                map_kind: CreatureCoatSourceMapKind::Anatomy,
                width: 63,
                height: 64,
            })
        );
    }

    #[test]
    fn coat_key_contains_all_five_sources_and_every_atlas_gene() {
        let baseline = coat_key();
        let variants = [
            CreatureCoatKey::new(
                CreaturePartSources {
                    head: CreaturePartFamilyId(99),
                    ..baseline.part_sources
                },
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                CreaturePartSources {
                    torso: CreaturePartFamilyId(99),
                    ..baseline.part_sources
                },
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                CreaturePartSources {
                    arms: CreaturePartFamilyId(99),
                    ..baseline.part_sources
                },
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                CreaturePartSources {
                    legs: CreaturePartFamilyId(99),
                    ..baseline.part_sources
                },
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                CreaturePartSources {
                    tail: CreaturePartFamilyId(99),
                    ..baseline.part_sources
                },
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                baseline.part_sources,
                baseline.palette_family + 1,
                baseline.fur_pattern,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                baseline.part_sources,
                baseline.palette_family,
                baseline.fur_pattern + 1,
                baseline.marking_density,
            ),
            CreatureCoatKey::new(
                baseline.part_sources,
                baseline.palette_family,
                baseline.fur_pattern,
                baseline.marking_density + 1,
            ),
        ];

        assert!(variants.into_iter().all(|variant| variant != baseline));
    }

    #[test]
    fn donor_geometry_identity_does_not_select_coat_colors() {
        let first = CreatureCoatKey::new(source_ids(0), 9, 3, 12);
        let second = CreatureCoatKey::new(source_ids(40), 9, 3, 12);

        assert_eq!(
            resolve_creature_coat_palette(first),
            resolve_creature_coat_palette(second)
        );
    }

    #[test]
    fn controlled_gene_changes_alter_deterministic_atlas_bytes() {
        let masks = source_masks();
        let baseline = bake_creature_coat(coat_key(), &masks).unwrap();
        for changed in [
            CreatureCoatKey::new(source_ids(1), 5, 7, 11),
            CreatureCoatKey::new(source_ids(1), 4, 8, 11),
            CreatureCoatKey::new(source_ids(1), 4, 7, 12),
        ] {
            let atlas = bake_creature_coat(changed, &masks).unwrap();
            assert_ne!(atlas.rgba, baseline.rgba, "changed key {changed:?}");
        }
    }

    #[test]
    fn identical_coat_inputs_are_byte_identical_rgba8_atlases() {
        let masks = source_masks();
        let first = bake_creature_coat(coat_key(), &masks).unwrap();
        let second = bake_creature_coat(coat_key(), &masks).unwrap();

        assert_eq!((first.width, first.height), (256, 256));
        assert_eq!(first.rgba.len(), CREATURE_COAT_RGBA_BYTES);
        assert_eq!(first, second);
        assert!(first.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn every_production_palette_is_bold_and_high_contrast() {
        for palette_family in 0..16 {
            let palette = resolve_creature_coat_palette(CreatureCoatKey::new(
                source_ids(0),
                palette_family,
                0,
                0,
            ));
            assert!(
                palette.primary_secondary_value_contrast() >= 0.28,
                "palette {palette_family} is too low contrast: {palette:?}"
            );
            assert!(palette.minimum_saturation() >= 0.52);
        }
    }

    #[test]
    fn all_seven_slot_regions_are_populated_and_join_covers_match() {
        let atlas = bake_creature_coat(coat_key(), &source_masks()).unwrap();
        for region in CreatureCoatAtlasRegion::ALL {
            assert!(atlas.region_has_nonzero_color(region), "missing {region:?}");
            assert_eq!(join_cover_atlas_region(region.slot()), region);
        }
    }

    #[test]
    fn all_anatomical_coat_channels_visibly_populate() {
        let atlas = bake_creature_coat(coat_key(), &source_masks()).unwrap();
        let channels = [
            CreatureCoatAnatomicalChannel::Primary,
            CreatureCoatAnatomicalChannel::Belly,
            CreatureCoatAnatomicalChannel::Muzzle,
            CreatureCoatAnatomicalChannel::InnerEar,
            CreatureCoatAnatomicalChannel::HandsFeet,
            CreatureCoatAnatomicalChannel::KeratinSkin,
            CreatureCoatAnatomicalChannel::SecondaryMarking,
        ];
        for channel in channels {
            assert!(
                atlas.channel_pixel_count(channel) >= 64,
                "missing visible {channel:?} pixels"
            );
        }
        assert_eq!(
            channels
                .into_iter()
                .map(|channel| anatomical_channel_color(channel, atlas.palette))
                .collect::<BTreeSet<_>>()
                .len(),
            channels.len(),
            "anatomical channels must resolve to visibly distinct colors"
        );
    }

    #[test]
    fn semantic_masks_reject_regions_owned_by_another_source() {
        let mut masks = source_masks();
        paint_region(
            &mut masks.torso.semantic_microdetail,
            CreatureCoatSemanticRegion::LeftArm,
            73,
        );
        let primary = CreatureCoatAnatomicalChannel::Primary.mask_color();
        for (x, y, semantic) in masks.torso.semantic_microdetail.enumerate_pixels() {
            if semantic[3] > 0 && masks.torso.anatomy.get_pixel(x, y)[3] == 0 {
                masks.torso.anatomy.put_pixel(
                    x,
                    y,
                    Rgba([primary[0], primary[1], primary[2], 255]),
                );
            }
        }

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::SemanticRegionOwnedByWrongSource {
                label: "torso",
                region: CreatureCoatSemanticRegion::LeftArm,
            })
        );
    }

    #[test]
    fn each_source_must_supply_its_required_regions_before_merge() {
        let mut masks = source_masks();
        masks.arms.semantic_microdetail = RgbaImage::new(64, 64);
        masks.arms.anatomy = RgbaImage::new(64, 64);

        assert_eq!(
            bake_creature_coat(coat_key(), &masks),
            Err(CreatureCoatError::MissingSourceSemanticRegion {
                label: "arms",
                region: CreatureCoatSemanticRegion::LeftArm,
            })
        );
    }

    #[test]
    fn cache_reuse_returns_one_material_identity_for_an_entire_assembly() {
        let mut cache = CreatureCoatCache::new(CreatureCoatCacheLimits::minimum()).unwrap();
        let key = coat_key();
        let first = cache.acquire(key, CreatureCoatAssetPair::new(10, 20));
        let reused = cache.acquire(key, CreatureCoatAssetPair::new(11, 21));

        assert!(first.inserted);
        assert!(!reused.inserted);
        assert_eq!(first.selected.material_id, 20);
        assert_eq!(reused.selected.material_id, 20);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.pin_count(key), Some(2));
        assert_eq!(
            CreatureCoatAtlasRegion::ALL
                .into_iter()
                .map(|_| reused.selected.material_id)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([20])
        );
        cache.release(first.selected_key).unwrap();
        cache.release(reused.selected_key).unwrap();
    }

    #[test]
    fn production_cache_api_has_no_raw_unpinned_insert() {
        let source = include_str!("creature_coat.rs");
        let public_insert_signature = ["    pub fn ", "insert("].concat();

        assert!(
            !source.contains(&public_insert_signature),
            "runtime callers must use atomic acquire/release"
        );
    }

    #[test]
    fn profile_cache_limits_match_the_approved_count_and_byte_caps() {
        assert_eq!(
            CreatureCoatCacheLimits::for_profile(
                crate::ProductionFrontendProfileId::MinimumSettings30x30,
            ),
            CreatureCoatCacheLimits {
                max_entries: 48,
                max_rgba_bytes: 12 * 1024 * 1024,
            }
        );
        assert_eq!(
            CreatureCoatCacheLimits::for_profile(
                crate::ProductionFrontendProfileId::MinSpecComfort1080p,
            ),
            CreatureCoatCacheLimits {
                max_entries: 96,
                max_rgba_bytes: 24 * 1024 * 1024,
            }
        );
        for profile in [
            crate::ProductionFrontendProfileId::Balanced1080p,
            crate::ProductionFrontendProfileId::HighSpecScaleUp,
            crate::ProductionFrontendProfileId::ResearchScale,
        ] {
            assert_eq!(
                CreatureCoatCacheLimits::for_profile(profile),
                CreatureCoatCacheLimits {
                    max_entries: 256,
                    max_rgba_bytes: 64 * 1024 * 1024,
                }
            );
        }
    }

    #[test]
    fn cache_rejects_limits_that_cannot_hold_one_exact_atlas() {
        for limits in [
            CreatureCoatCacheLimits {
                max_entries: 0,
                max_rgba_bytes: CREATURE_COAT_RGBA_BYTES,
            },
            CreatureCoatCacheLimits {
                max_entries: 1,
                max_rgba_bytes: CREATURE_COAT_RGBA_BYTES - 1,
            },
        ] {
            assert!(matches!(
                CreatureCoatCache::new(limits),
                Err(CreatureCoatCacheError::InvalidLimits { limits: rejected })
                    if rejected == limits
            ));
        }
    }

    #[test]
    fn thousand_generation_churn_stays_bounded_and_never_evicts_pins() {
        let limits = CreatureCoatCacheLimits::minimum();
        let mut cache = CreatureCoatCache::new(limits).unwrap();
        let mut pinned_pairs = BTreeSet::new();
        let mut pinned_keys = Vec::new();

        for generation in 0_u16..1_000 {
            let key = CreatureCoatKey::new(
                source_ids(generation),
                (generation % 16) as u8,
                ((generation / 3) % 16) as u8,
                ((generation / 7) % 16) as u8,
            );
            let pair = CreatureCoatAssetPair::new(
                u64::from(generation) * 2,
                u64::from(generation) * 2 + 1,
            );
            let update = cache.acquire(key, pair);
            if generation < 8 {
                pinned_pairs.insert(update.selected);
                pinned_keys.push(update.selected_key);
            } else {
                cache.release(update.selected_key).unwrap();
            }
            assert!(update
                .evicted
                .iter()
                .all(|evicted| !pinned_pairs.contains(evicted)));
            assert!(cache.len() <= limits.max_entries);
            assert!(cache.rgba_bytes() <= limits.max_rgba_bytes);
        }

        assert!(pinned_pairs.iter().all(|pair| cache.contains_pair(*pair)));
        assert_eq!(cache.len(), limits.max_entries);
        assert_eq!(cache.rgba_bytes(), limits.max_rgba_bytes);
        for key in pinned_keys {
            cache.release(key).unwrap();
        }
    }

    #[test]
    fn fully_pinned_cache_reuses_nearest_coat_without_growing() {
        let limits = CreatureCoatCacheLimits {
            max_entries: 2,
            max_rgba_bytes: CREATURE_COAT_RGBA_BYTES * 2,
        };
        let mut cache = CreatureCoatCache::new(limits).unwrap();
        let first_key = CreatureCoatKey::new(source_ids(0), 2, 3, 4);
        let second_key = CreatureCoatKey::new(source_ids(5), 8, 9, 10);
        cache.acquire(first_key, CreatureCoatAssetPair::new(1, 2));
        cache.acquire(second_key, CreatureCoatAssetPair::new(3, 4));

        let requested_key = CreatureCoatKey::new(source_ids(10), 3, 3, 4);
        let result = cache.acquire(requested_key, CreatureCoatAssetPair::new(5, 6));

        assert!(!result.inserted);
        assert!(result.used_pinned_fallback);
        assert_eq!(result.selected_key, first_key);
        assert_ne!(result.selected_key, requested_key);
        assert_eq!(result.selected.material_id, 2);
        assert!(cache.contains_pair(result.selected));
        assert_eq!(cache.pin_count(result.selected_key), Some(2));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.rgba_bytes(), CREATURE_COAT_RGBA_BYTES * 2);
        cache.release(result.selected_key).unwrap();
        cache.release(first_key).unwrap();
        cache.release(second_key).unwrap();
    }

    #[test]
    fn acquire_is_atomic_and_release_rejects_unbalanced_underflow() {
        let mut cache = CreatureCoatCache::new(CreatureCoatCacheLimits::minimum()).unwrap();
        let key = coat_key();
        let acquired = cache.acquire(key, CreatureCoatAssetPair::new(1, 2));

        assert_eq!(acquired.selected_key, key);
        assert_eq!(cache.pin_count(key), Some(1));
        cache.release(acquired.selected_key).unwrap();
        assert_eq!(cache.pin_count(key), Some(0));
        assert_eq!(
            cache.release(key),
            Err(CreatureCoatCacheError::UnbalancedRelease)
        );
    }

    #[test]
    fn nearest_fallback_distance_includes_all_five_source_ids() {
        let limits = CreatureCoatCacheLimits {
            max_entries: 2,
            max_rgba_bytes: CREATURE_COAT_RGBA_BYTES * 2,
        };
        let requested = CreatureCoatKey::new(source_ids(100), 4, 5, 6);
        let gene_near_source_far = CreatureCoatKey::new(source_ids(0), 4, 5, 6);
        let gene_far_source_near = CreatureCoatKey::new(source_ids(99), 5, 5, 6);
        let mut cache = CreatureCoatCache::new(limits).unwrap();
        cache.acquire(gene_near_source_far, CreatureCoatAssetPair::new(1, 2));
        cache.acquire(gene_far_source_near, CreatureCoatAssetPair::new(3, 4));

        let fallback = cache.acquire(requested, CreatureCoatAssetPair::new(5, 6));

        assert!(fallback.used_pinned_fallback);
        assert_eq!(fallback.selected_key, gene_far_source_near);
        assert!(cache.contains_pair(fallback.selected));
        assert_eq!(cache.pin_count(fallback.selected_key), Some(2));
        cache.release(gene_near_source_far).unwrap();
        cache.release(fallback.selected_key).unwrap();
        cache.release(gene_far_source_near).unwrap();
    }
}
