use bevy::prelude::*;
pub const BASE_COLORS: [Color; 4] = [
    Color::srgb(1.0, 0.20, 0.25),
    Color::srgb(0.20, 0.55, 0.90),
    Color::srgb(1.0, 1.0, 0.15),
    Color::srgb(0.20, 0.80, 0.45),
];
#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}
