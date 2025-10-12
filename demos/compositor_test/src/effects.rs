//! Visual effect animation systems.

use bevy::prelude::*;

use crate::components::EffectsPulse;

/// Animates the alpha channel of the effect overlay sprite.
pub fn animate_effect_overlay(time: Res<Time>, mut query: Query<&mut Sprite, With<EffectsPulse>>) {
    let elapsed = time.elapsed_secs();
    for mut sprite in &mut query {
        let wave = (elapsed * 1.2).sin() * 0.5 + 0.5;
        sprite.color = sprite.color.with_alpha(0.12 + wave * 0.18);
    }
}
