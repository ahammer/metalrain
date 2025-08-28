use bevy::prelude::*;
pub const BASE_COLORS: [Color; 4] = [
    // Brighter, higher contrast enabled colors
    Color::srgb(1.0, 0.15, 0.20),   // Vivid warm red
    Color::srgb(0.10, 0.60, 1.0),   // Electric blue
    Color::srgb(1.0, 1.0, 0.25),    // Bright lemon yellow
    Color::srgb(0.15, 0.95, 0.55),  // Bright spring green
];

// Secondary palette (disabled state variants)
pub const SECONDARY_COLORS: [Color; 4] = [
    // Darker, desaturated disabled variants (clearly distinct, harmonious)
    Color::srgb(0.35, 0.05, 0.07),  // Deep muted red
    Color::srgb(0.05, 0.22, 0.40),  // Deep steel blue
    Color::srgb(0.40, 0.40, 0.06),  // Dark olive yellow
    Color::srgb(0.05, 0.30, 0.17),  // Deep forest green
];

#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}
#[inline]
pub fn secondary_color_for_index(i: usize) -> Color {
    SECONDARY_COLORS[i % SECONDARY_COLORS.len()]
}
