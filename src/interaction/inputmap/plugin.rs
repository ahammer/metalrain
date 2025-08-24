use bevy::prelude::*;

use super::types::InputMap;
use super::parse::parse_input_toml;
use super::systems::{system_collect_inputs, system_evaluate_bindings};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct InputActionUpdateSet;

pub struct InputActionsPlugin;
impl Plugin for InputActionsPlugin { fn build(&self, app: &mut App) { app
        .init_resource::<InputMap>()
        .configure_sets(PreUpdate, InputActionUpdateSet)
        .add_systems(PreStartup, load_initial_input_map)
        .add_systems(PreUpdate, (system_collect_inputs, system_evaluate_bindings).chain().in_set(InputActionUpdateSet)); } }

fn load_initial_input_map(mut commands: Commands) {
    let path = std::env::var("INPUT_CONFIG_PATH").unwrap_or_else(|_| "assets/config/input.toml".into());
    #[cfg(target_arch = "wasm32")] let raw: String = include_str!("../../../assets/config/input.toml").to_string();
    #[cfg(not(target_arch = "wasm32"))] let raw: String = std::fs::read_to_string(&path).unwrap_or_default();
    let parsed = parse_input_toml(&raw, cfg!(feature="debug"));
    if !parsed.errors.is_empty() { for e in parsed.errors { error!("INPUT MAP ERROR: {e}"); } } else { info!("Input map loaded: {} actions", parsed.input_map.actions.len()); }
    commands.insert_resource(parsed.input_map); }
