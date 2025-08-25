// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;

use crate::core::system::system_order::{PostPhysicsAdjustSet, PrePhysicsSet};
use crate::debug::DebugPlugin;
use crate::gameplay::spawn::spawn::BallSpawnPlugin;
use crate::interaction::cluster_pop::ClusterPopPlugin;
#[cfg(feature = "debug")]
use crate::interaction::inputmap::hot_reload::InputMapHotReloadPlugin;
use crate::interaction::inputmap::plugin::InputActionsPlugin;
use crate::interaction::session::auto_close::AutoClosePlugin;
use crate::interaction::session::config_hot_reload::ConfigHotReloadPlugin;
use crate::physics::clustering::cluster::ClusterPlugin;
use crate::physics::gravity::radial_gravity::RadialGravityPlugin;
use crate::physics::rapier::rapier_physics::PhysicsSetupPlugin;
use crate::physics::separation::separation::SeparationPlugin;
use crate::rendering::camera::camera::CameraPlugin;
use crate::rendering::materials::materials::MaterialsPlugin;
use crate::rendering::metaballs::metaballs::MetaballsPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (PrePhysicsSet, PostPhysicsAdjustSet.after(PrePhysicsSet)),
        )
        .add_plugins((
            CameraPlugin,
            MaterialsPlugin,
            PhysicsSetupPlugin,
            RadialGravityPlugin,
            BallSpawnPlugin,
            SeparationPlugin,
            ClusterPlugin,
            MetaballsPlugin,
            InputActionsPlugin,
            ClusterPopPlugin,

            DebugPlugin,
            ConfigHotReloadPlugin,
            AutoClosePlugin,
            #[cfg(feature = "debug")]
            InputMapHotReloadPlugin,
        ));
    }
}
// Removed verbose debug_entity_counts logging system.
