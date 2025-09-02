use bevy::prelude::*;
use bevy::ui::{Node, FlexDirection, JustifyContent, AlignItems};
// use bevy_text::{TextFont, TextColor, TextLayout, JustifyText};

use crate::core::level::registry::LevelRegistry;
use crate::core::level::loader::PendingLevel;
use super::state::AppState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            // Log + UI spawn on enter
            .add_systems(OnEnter(AppState::MainMenu), (show_menu_instructions, spawn_menu_ui))
            // Update systems while in menu
            .add_systems(
                Update,
                (
                    handle_menu_input,
                    populate_menu_text,
                )
                    .run_if(in_state(AppState::MainMenu)),
            )
            // Cleanup UI on exit
            .add_systems(OnExit(AppState::MainMenu), despawn_menu_ui);
    }
}

fn show_menu_instructions(registry: Option<Res<LevelRegistry>>) {
    info!(target: "menu", "=== MAIN MENU ===");
    if let Some(reg) = registry {
        info!(target: "menu", "Select a level by pressing its number:");
        for (i, entry) in reg.list.iter().enumerate() {
            info!(target: "menu", "  {}: {} (layout='{}' widgets='{}')", i + 1, entry.id, entry.layout, entry.widgets);
        }
    } else {
        warn!(target: "menu", "LevelRegistry missing; no levels to display.");
    }
}

fn handle_menu_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    registry: Option<Res<LevelRegistry>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(reg) = registry else { return; };
    // Scan digits 1..=9,0 (0 not used unless >9 levels)
    for (i, entry) in reg.list.iter().enumerate() {
        // Support up to 10 via Digit1..Digit0 mapping (0 => index 9)
        let keycode = match i { 
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            3 => KeyCode::Digit4,
            4 => KeyCode::Digit5,
            5 => KeyCode::Digit6,
            6 => KeyCode::Digit7,
            7 => KeyCode::Digit8,
            8 => KeyCode::Digit9,
            9 => KeyCode::Digit0,
            _ => break, // Keep minimal (spec only needs 1..N numeric)
        };
        if keys.just_pressed(keycode) {
            info!(target: "menu", "Selected level '{}' (index {})", entry.id, i + 1);
            // Insert pending level BEFORE state transition (race-free)
            commands.insert_resource(PendingLevel { id: entry.id.clone() });
            next_state.set(AppState::Loading);
            break;
        }
    }
}

// === UI IMPLEMENTATION ===

#[derive(Component)]
struct MenuUiRoot;
#[derive(Component)]
struct MenuUiText;

fn spawn_menu_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Attempt to load font (optional; default font may be used otherwise)
    let _ = asset_server.load::<Font>("fonts/FiraSans-Bold.ttf");

    let root = commands
        .spawn((
            MenuUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.85)),
        ))
        .id();

    // Placeholder text; will be populated once registry is present.
    commands.entity(root).with_children(|p| {
        p.spawn((MenuUiText, Text::new("Loading level list...")));
    });
}

fn populate_menu_text(
    registry: Option<Res<LevelRegistry>>,
    mut q_text: Query<&mut Text, With<MenuUiText>>,
) {
    let Ok(mut text) = q_text.single_mut() else { return; };
    if let Some(reg) = registry {
        let mut s = String::new();
        s.push_str("=== MAIN MENU ===\n");
        s.push_str("Select a level by pressing its number:\n");
        for (i, entry) in reg.list.iter().enumerate() {
            s.push_str(&format!("  {}: {} (layout='{}' widgets='{}')\n", i + 1, entry.id, entry.layout, entry.widgets));
        }
        if text.as_str() != s { *text = Text::new(s); }
    } else {
        let fallback = "No level registry found. (Press Esc to quit)";
        if text.as_str() != fallback { *text = Text::new(fallback); }
    }
}

fn despawn_menu_ui(mut commands: Commands, q_root: Query<Entity, With<MenuUiRoot>>) {
    for e in &q_root {
        commands.entity(e).despawn();
    }
}
