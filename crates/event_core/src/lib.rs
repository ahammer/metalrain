//! Event & Input Reduction Core Crate
//!
//! This crate provides a deterministic, testable event pipeline that decouples raw input
//! from high‑level game state mutations. It is intentionally self‑contained so demos can
//! migrate incrementally. See README.md for architecture overview.

mod event;
mod queue;
mod middleware;
mod reducer;
pub mod handlers;

pub use event::*;
pub use queue::*;
pub use middleware::*;
pub use reducer::*;

use bevy::prelude::*;

/// High-level standardized system flow ordering for input + event pipeline aware apps.
///
/// Stages:
/// - InputCollect: gather raw device input (mouse position, button/key states)
/// - InputProcess: transform raw input into high-level domain events (enqueue Game / Input events)
/// - UIUpdate: update overlays / previews that depend on processed input state
///
/// Additional sets can be layered before reducer_system (which runs PostUpdate) if needed.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EventFlowSet {
    InputCollect,
    InputProcess,
    UIUpdate,
}

/// Plugin configuring the event pipeline (queue, journal, reducer system).
pub struct EventCorePlugin {
    pub journal_capacity: usize,
}

impl Default for EventCorePlugin {
    fn default() -> Self { Self { journal_capacity: 512 } }
}

impl Plugin for EventCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCounter>()
            .insert_resource(EventQueue::with_capacity(self.journal_capacity))
            .init_resource::<HandlerRegistry>()
            .init_resource::<MiddlewareChain>()
            // Configure ordered chain for higher-level gameplay crates to hook into.
            .configure_sets(Update, (
                EventFlowSet::InputCollect,
                EventFlowSet::InputProcess,
                EventFlowSet::UIUpdate,
            ).chain())
            .add_systems(PreUpdate, increment_frame_counter)
            .add_systems(PostUpdate, reducer_system);
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct FrameCounter(pub u64);
fn increment_frame_counter(mut fc: ResMut<FrameCounter>) { fc.0 += 1; }

/// Builder style extension methods for registering handlers and middleware.
pub trait EventCoreAppExt {
    fn register_handler<H: EventHandler + Send + Sync + 'static>(&mut self, handler: H) -> &mut Self;
    fn register_middleware<M: Middleware + Send + Sync + 'static>(&mut self, mw: M) -> &mut Self;
}

impl EventCoreAppExt for App {
    fn register_handler<H: EventHandler + Send + Sync + 'static>(&mut self, handler: H) -> &mut Self {
    let mut reg = self.world_mut().resource_mut::<HandlerRegistry>();
        reg.register(handler);
        self
    }
    fn register_middleware<M: Middleware + Send + Sync + 'static>(&mut self, mw: M) -> &mut Self {
    let mut chain = self.world_mut().resource_mut::<MiddlewareChain>();
        chain.add(mw);
        self
    }
}
