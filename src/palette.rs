//! Centralized ball color palette & helpers.
//! Ensures a single source of truth for visual + debug + shader paths.

use bevy::prelude::*;

/// Base SRGB palette (kept small & high-contrast). Update here only.
pub const BASE_COLORS: [Color; 6] = [
    Color::srgb(0.90, 0.20, 0.25), // red
    Color::srgb(0.20, 0.55, 0.90), // blue
    Color::srgb(0.95, 0.75, 0.15), // yellow
    Color::srgb(0.20, 0.80, 0.45), // green
    Color::srgb(0.65, 0.45, 0.95), // purple
    Color::srgb(0.95, 0.50, 0.15), // orange
];

/// Returns a color for arbitrary index, wrapping around the base palette.
#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_behavior() {
        assert_eq!(color_for_index(0), BASE_COLORS[0]);
        assert_eq!(color_for_index(6), BASE_COLORS[0]); // wrap
        assert_eq!(color_for_index(7), BASE_COLORS[1]);
    }

    #[test]
    fn all_colors_distinct_enough() {
        // Ensure no two colors are exactly identical (protect against accidental duplicates)
        for (i, c1) in BASE_COLORS.iter().enumerate() {
            for (j, c2) in BASE_COLORS.iter().enumerate() {
                if i == j {
                    continue;
                }
                assert!(c1 != c2, "Palette contains duplicate colors at {i} and {j}");
            }
        }
    }
}
