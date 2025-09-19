use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::default())
        .run();
}
