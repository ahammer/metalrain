use crate::simulation::HALF_EXTENT;
use bevy::prelude::*;
use bevy::sprite::MeshMaterial2d;
use game_core::AppState;
use metaball_renderer::{MetaBall, MetaBallColor};

#[derive(Resource, Default)]
pub struct DebugVisToggle(pub bool);

pub struct DebugVisPlugin;
impl Plugin for DebugVisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugVisToggle>()
            .add_systems(OnEnter(AppState::Playing), setup_lines)
            .add_systems(
                Update,
                (toggle_debug, apply_visibility, draw_ball_circles)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

#[derive(Component)]
struct DebugLine;

fn setup_lines(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let he = HALF_EXTENT;
    let z = 5.0;
    let red = materials.add(Color::linear_rgba(0.9, 0.15, 0.15, 1.0));
    let green = materials.add(Color::linear_rgba(0.15, 0.9, 0.25, 1.0));
    let gray = materials.add(Color::linear_rgba(0.55, 0.55, 0.65, 1.0));
    let mut spawn_rect = |w: f32, h: f32, x: f32, y: f32, mat: Handle<ColorMaterial>| {
        let m = Mesh::from(Rectangle::new(w, h));
        let mh = meshes.add(m);
        commands.spawn((
            Mesh2d(mh),
            MeshMaterial2d(mat),
            Transform::from_xyz(x, y, z),
            DebugLine,
        ));
    };
    let thickness = 2.0;
    spawn_rect(he * 2.0, thickness, 0.0, 0.0, red.clone());
    spawn_rect(thickness, he * 2.0, 0.0, 0.0, green.clone());
    let bw = 1.5;
    let size = he * 2.0;
    spawn_rect(size, bw, 0.0, he, gray.clone());
    spawn_rect(size, bw, 0.0, -he, gray.clone());
    spawn_rect(bw, size, -he, 0.0, gray.clone());
    spawn_rect(bw, size, he, 0.0, gray.clone());
}

fn toggle_debug(keys: Res<ButtonInput<KeyCode>>, mut toggle: ResMut<DebugVisToggle>) {
    if keys.just_pressed(KeyCode::KeyH) {
        toggle.0 = !toggle.0;
        info!("Debug vis {}", if toggle.0 { "ON" } else { "OFF" });
    }
}

fn apply_visibility(toggle: Res<DebugVisToggle>, mut q: Query<&mut Visibility, With<DebugLine>>) {
    if !toggle.is_changed() {
        return;
    }
    let vis = if toggle.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in q.iter_mut() {
        *v = vis.clone();
    }
}

fn draw_ball_circles(
    toggle: Res<DebugVisToggle>,
    q: Query<(&Transform, &MetaBall, Option<&MetaBallColor>)>,
    mut gizmos: Gizmos,
) {
    if !toggle.0 {
        return;
    }
    for (tr, mb, color) in q.iter() {
        let p = tr.translation.truncate();
        let base = color.map(|c| Color::from(c.0)).unwrap_or(Color::WHITE);
        let linear = base.to_linear();
        let mut r = linear.red + (1.0 - linear.red) * 0.5;
        let mut g = linear.green + (1.0 - linear.green) * 0.5;
        let mut b = linear.blue + (1.0 - linear.blue) * 0.5;
        g = (g + 0.15).min(1.0);
        let avg = (r + g + b) / 3.0;
        let desat_factor = 0.25;
        r = r + (avg - r) * desat_factor;
        g = g + (avg - g) * desat_factor;
        b = b + (avg - b) * desat_factor;
        let col = Color::linear_rgba(r, g, b, 0.55);
        gizmos.circle_2d(Isometry2d::from_translation(p), mb.radius_world, col);
    }
}
