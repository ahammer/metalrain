// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;

use crate::core::system::system_order::{PostPhysicsAdjustSet, PrePhysicsSet};
use crate::debug::DebugPlugin;
use crate::interaction::cluster_pop::ClusterPopPlugin;
#[cfg(feature = "debug")]
use crate::interaction::inputmap::hot_reload::InputMapHotReloadPlugin;
use crate::interaction::inputmap::plugin::InputActionsPlugin;
use crate::interaction::session::auto_close::AutoClosePlugin;
use crate::interaction::session::config_hot_reload::ConfigHotReloadPlugin;
use crate::physics::clustering::cluster::ClusterPlugin;
// use crate::physics::gravity::radial_gravity::RadialGravityPlugin; // legacy
use crate::gameplay::spawn_widgets::SpawnWidgetsPlugin;
use crate::physics::gravity::widgets::GravityWidgetsPlugin;
use crate::physics::rapier::rapier_physics::PhysicsSetupPlugin;
use crate::rendering::camera::camera::CameraPlugin;
use crate::rendering::materials::materials::MaterialsPlugin;
use crate::rendering::metaballs::MetaballsPlugin;
use crate::rendering::sdf_atlas::SdfAtlasPlugin;

/// Aggregates core plugins: rendering, physics setup, spawning, clustering,
/// metaball rendering, input actions, interaction mechanics, debug & tooling.
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
            GravityWidgetsPlugin, // gravity
            SpawnWidgetsPlugin,   // new spawn widgets
            ClusterPlugin,
            MetaballsPlugin,
            SdfAtlasPlugin,
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
