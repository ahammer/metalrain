// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;

use crate::rendering::camera::camera::CameraPlugin;
use crate::rendering::background::background::BackgroundPlugin;
use crate::physics::clustering::cluster::ClusterPlugin;
use crate::interaction::input::input_interaction::InputInteractionPlugin;
use crate::rendering::materials::materials::MaterialsPlugin;
use crate::rendering::metaballs::metaballs::MetaballsPlugin;
use crate::physics::gravity::radial_gravity::RadialGravityPlugin;
use crate::physics::rapier::rapier_physics::PhysicsSetupPlugin;
use crate::physics::separation::separation::SeparationPlugin;
use crate::gameplay::spawn::spawn::BallSpawnPlugin;
use crate::core::system::system_order::{PostPhysicsAdjustSet, PrePhysicsSet};
use crate::debug::DebugPlugin;
use crate::interaction::session::config_hot_reload::ConfigHotReloadPlugin;
use crate::interaction::session::auto_close::AutoClosePlugin;
use crate::interaction::inputmap::plugin::InputActionsPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets(
                Update,
                (PrePhysicsSet, PostPhysicsAdjustSet.after(PrePhysicsSet)),
            )
            .add_plugins((
                BackgroundPlugin,
                CameraPlugin,
                MaterialsPlugin,
                PhysicsSetupPlugin,
                RadialGravityPlugin,
                BallSpawnPlugin,
                SeparationPlugin,
                ClusterPlugin,
                MetaballsPlugin,
                InputActionsPlugin,
                InputInteractionPlugin,
                DebugPlugin,
                ConfigHotReloadPlugin,
                AutoClosePlugin,
            ));
    }
}
// Removed verbose debug_entity_counts logging system.
