mod event;
pub mod handlers;
mod middleware;
mod queue;
mod reducer;

pub use event::*;
pub use middleware::*;
pub use queue::*;
pub use reducer::*;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EventFlowSet {
    InputCollect,
    InputProcess,
    UIUpdate,
}

pub struct EventCorePlugin {
    pub journal_capacity: usize,
}

impl Default for EventCorePlugin {
    fn default() -> Self {
        Self {
            journal_capacity: 512,
        }
    }
}

impl Plugin for EventCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCounter>()
            .insert_resource(EventQueue::with_capacity(self.journal_capacity))
            .init_resource::<HandlerRegistry>()
            .init_resource::<MiddlewareChain>()
            .configure_sets(
                Update,
                (
                    EventFlowSet::InputCollect,
                    EventFlowSet::InputProcess,
                    EventFlowSet::UIUpdate,
                )
                    .chain(),
            )
            .add_systems(PreUpdate, increment_frame_counter)
            .add_systems(PostUpdate, reducer_system);
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct FrameCounter(pub u64);
fn increment_frame_counter(mut fc: ResMut<FrameCounter>) {
    fc.0 += 1;
}

pub trait EventCoreAppExt {
    fn register_handler<H: EventHandler + Send + Sync + 'static>(
        &mut self,
        handler: H,
    ) -> &mut Self;
    fn register_middleware<M: Middleware + Send + Sync + 'static>(&mut self, mw: M) -> &mut Self;
}

impl EventCoreAppExt for App {
    fn register_handler<H: EventHandler + Send + Sync + 'static>(
        &mut self,
        handler: H,
    ) -> &mut Self {
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
