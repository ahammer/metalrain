// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::background::BackgroundPlugin;
use crate::cluster::ClusterPlugin;
use crate::input_interaction::InputInteractionPlugin;
use crate::materials::MaterialsPlugin;
use crate::metaballs::MetaballsPlugin;
use crate::radial_gravity::RadialGravityPlugin;
use crate::rapier_physics::PhysicsSetupPlugin;
use crate::separation::SeparationPlugin;
use crate::spawn::BallSpawnPlugin;
use crate::system_order::{PostPhysicsAdjustSet, PrePhysicsSet};
use crate::debug::DebugPlugin;
use crate::config_hot_reload::ConfigHotReloadPlugin;
use crate::fluid_sim::FluidSimPlugin;
use crate::fluid_impulses::FluidImpulsesPlugin;
use crate::auto_close::AutoClosePlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
    app
            // Register custom system sets (order constraints added later as needed)
            .configure_sets(
                Update,
                (PrePhysicsSet, PostPhysicsAdjustSet.after(PrePhysicsSet)),
            )
            .add_plugins((
    BackgroundPlugin, // draws implicit background (no clear)
    FluidImpulsesPlugin, // collects & extracts per-frame fluid impulses (Phase 2)
    FluidSimPlugin, // GPU fluid simulation background
                CameraPlugin,
                MaterialsPlugin,
                PhysicsSetupPlugin,
                RadialGravityPlugin,
                BallSpawnPlugin, // initial burst only
                SeparationPlugin,
                ClusterPlugin,
                MetaballsPlugin,
                InputInteractionPlugin,
                DebugPlugin,
                ConfigHotReloadPlugin,
                AutoClosePlugin,
            ));
    }
}
// Removed verbose debug_entity_counts logging system (was spamming every second).
