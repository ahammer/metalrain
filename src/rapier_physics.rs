use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::config::GameConfig;

pub struct PhysicsSetupPlugin; // our wrapper to configure Rapier & arena

impl Plugin for PhysicsSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RapierPhysicsPlugin::<NoUserData>::default(),))
            .add_systems(Startup, configure_gravity);
    }
}

fn configure_gravity(
    mut q_cfg: Query<&mut RapierConfiguration>,
    _game_cfg: Res<GameConfig>,
) {
    // Bevy/Rapier migration note (0.16 / recent rapier): RapierConfiguration is now queried
    // as a component instead of taken as a ResMut<...>. We use Query::single_mut() (new
    // unified error handling API) instead of deprecated get_single_mut().
    if let Ok(mut cfg) = q_cfg.single_mut() {
        // Disable global gravity; custom radial gravity system applies forces per body.
        cfg.gravity = Vect::new(0.0, 0.0);
    }
}
