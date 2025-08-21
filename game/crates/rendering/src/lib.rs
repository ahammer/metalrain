 // Phase 3 (in progress): Rendering crate basics.
 // Adds: RenderingPlugin that spawns a 2D camera, sets a clear color, and defines a simple placeholder palette.
 // Future: background mesh/shader, materials, palette integration, camera controls.

 use bevy::prelude::*;

 pub struct RenderingPlugin;

 #[derive(Component)]
 pub struct GameCamera;

 /// Placeholder palette (will be expanded with material and UI colors later).
 pub struct Palette;
 impl Palette {
     pub const BG: Color = Color::srgb(0.02, 0.02, 0.05);
     pub const BALL: Color = Color::srgb(0.95, 0.95, 1.0);
     pub const HIGHLIGHT: Color = Color::srgb(0.3, 0.6, 1.0);
 }

 fn setup_camera(mut commands: Commands) {
     // Minimal placeholder camera (will be upgraded to proper 2D camera bundle later).
     commands.spawn((
         Camera::default(),
         GameCamera,
     ));
 }


 impl Plugin for RenderingPlugin {
     fn build(&self, app: &mut App) {
         app.insert_resource(ClearColor(Palette::BG));
         app.add_systems(Startup, setup_camera);
     }
 }

 #[cfg(test)]
 mod tests {
     use super::*;

     #[test]
     fn plugin_adds_and_spawns_camera() {
         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.add_plugins(RenderingPlugin);
         // Run startup
         app.update();

         // Count GameCamera components
         let world = app.world_mut();
         let mut query = world.query::<&GameCamera>();
         let count = query.iter(&world).count();
         assert_eq!(count, 1, "expected exactly one GameCamera, found {count}");
     }

     #[test]
     fn clear_color_applied() {
         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.add_plugins(RenderingPlugin);
         app.update();
         let cc = app.world().resource::<ClearColor>();
         assert_eq!(cc.0, Palette::BG, "clear color mismatch");
     }
 }
