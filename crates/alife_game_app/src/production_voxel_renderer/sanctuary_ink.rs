//! Shared sea-glass enamel, recessed wells and warm ceramic lettering.
use bevy::prelude::{BorderColor, Color};

pub(crate) const GLASS: Color = Color::srgba(0.105, 0.215, 0.235, 0.96);
pub(crate) const WELL: Color = Color::srgb(0.095, 0.155, 0.185);
pub(crate) const PORCELAIN: Color = Color::srgb(1.0, 0.94, 0.80);
pub(crate) const JADE: Color = Color::srgb(0.53, 0.86, 0.76);

pub(crate) fn bevel(raised: bool) -> BorderColor {
    let rim = Color::srgb(0.43, 0.58, 0.58);
    let shade = Color::srgb(0.07, 0.115, 0.15);
    let (top, bottom) = if raised { (rim, shade) } else { (shade, rim) };
    BorderColor {
        top,
        left: top,
        bottom,
        right: bottom,
    }
}
