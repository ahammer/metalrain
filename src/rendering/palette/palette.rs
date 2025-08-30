use bevy::prelude::*;
pub const BASE_COLORS: [Color; 4] = [
    // Brighter, higher contrast enabled colors
    Color::srgb(1.0, 0.15, 0.20),   // Vivid warm red
    Color::srgb(0.10, 0.60, 1.0),   // Electric blue
    Color::srgb(1.0, 1.0, 0.25),    // Bright lemon yellow
    Color::srgb(0.15, 0.95, 0.55),  // Bright spring green
];

#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}
