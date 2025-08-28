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
    Color::srgb(0.22, 0.03, 0.04),  // Deeper muted red
    Color::srgb(0.03, 0.14, 0.26),  // Darker steel blue
    Color::srgb(0.24, 0.24, 0.04),  // Darker olive yellow
    Color::srgb(0.03, 0.18, 0.10),  // Darker forest green
];

#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}
#[inline]
pub fn secondary_color_for_index(i: usize) -> Color {
    SECONDARY_COLORS[i % SECONDARY_COLORS.len()]
}
