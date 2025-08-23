//! Centralized color palette & helpers (ported scaffold from legacy).
//! Keeps a single source of truth for ball / highlight / UI colors.
//! Future: expand with material parameterization & theme variants.

use bevy::prelude::*;

/// Base SRGB palette (kept small & high-contrast). Mirrors legacy ordering.
pub const BASE_COLORS: [Color; 4] = [
    Color::srgb(1.0, 0.20, 0.25), // red
    Color::srgb(0.20, 0.55, 0.90), // blue
    Color::srgb(1.0, 1.0, 0.15),   // yellow
    Color::srgb(0.20, 0.80, 0.45), // green
];

/// Returns a color for arbitrary index, wrapping around the base palette.
#[inline]
pub fn color_for_index(i: usize) -> Color {
    BASE_COLORS[i % BASE_COLORS.len()]
}

/// Rendering crate public palette surface (will later gain more semantic roles).
pub struct Palette;
impl Palette {
    pub const BG: Color = Color::srgb(0.02, 0.02, 0.05);
    pub const BALL_FALLBACK: Color = Color::srgb(0.95, 0.95, 1.0);
    pub const HIGHLIGHT: Color = Color::srgb(0.3, 0.6, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_behavior() {
        assert_eq!(color_for_index(0), BASE_COLORS[0]);
        assert_eq!(color_for_index(4), BASE_COLORS[0]);
        assert_eq!(color_for_index(5), BASE_COLORS[1]);
    }

    #[test]
    fn distinct_colors() {
        for (i, c1) in BASE_COLORS.iter().enumerate() {
            for (j, c2) in BASE_COLORS.iter().enumerate() {
                if i != j {
                    assert!(c1 != c2, "duplicate colors at {i} and {j}");
                }
            }
        }
    }
}
