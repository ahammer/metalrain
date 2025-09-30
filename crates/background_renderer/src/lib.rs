use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;

pub mod config;
pub mod material;
pub mod systems;

pub use config::{BackgroundConfig, BackgroundMode};
pub use material::BackgroundMaterial;

use systems::{setup_background, update_background};

pub struct BackgroundRendererPlugin;

impl Plugin for BackgroundRendererPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BackgroundConfig>()
            .register_type::<BackgroundMode>()
            .add_plugins(Material2dPlugin::<BackgroundMaterial>::default());

        if !app.world().contains_resource::<BackgroundConfig>() {
            app.insert_resource(BackgroundConfig::default());
        }
        app.add_systems(Startup, setup_background)
           .add_systems(Update, update_background);
    }
}
