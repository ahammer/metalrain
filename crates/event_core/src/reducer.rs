use bevy::prelude::*;
use crate::{EventQueue, HandlerRegistry, MiddlewareChain, EventPayload, GameEvent, EventResult, EventSourceTag, FrameCounter, EventEnvelope};

/// Exclusive system: drains current frame's queue, applies middleware sequentially, dispatches to handlers.
pub fn reducer_system(world: &mut World) {
    let frame_idx = world.resource::<FrameCounter>().0;
    // Drain events first (drop borrow afterwards)
    let events: Vec<EventEnvelope> = {
        let mut q = world.resource_mut::<EventQueue>();
        q.drain_for_frame(frame_idx)
    };
    for env in events.into_iter() {
        // Middleware phase
        let maybe_final = {
            let mut mw = world.resource_mut::<MiddlewareChain>();
            mw.run(env.clone())
        };
        let Some(final_env) = maybe_final else { continue; };
        let result = match &final_env.payload {
            EventPayload::Game(g) => {
                // Use resource_scope to borrow registry & world simultaneously.
                let mut result = EventResult::Ignored;
                world.resource_scope(|world, mut handlers: Mut<HandlerRegistry>| {
                    result = handlers.dispatch(g, world);
                });
                result
            }
            EventPayload::Input(_raw) => EventResult::Ignored, // should have been mapped earlier
            #[cfg(debug_assertions)]
            EventPayload::Debug(_d) => EventResult::Ignored,
        };
        // Journal push
        {
            let mut q = world.resource_mut::<EventQueue>();
            q.push_journal(crate::queue::JournalEntry { event: final_env, result, frame_processed: frame_idx });
        }
    }
    // Promote deferred events
    world.resource_mut::<EventQueue>().promote_next_frame();
}

/// Helper for handlers to emit new events (which will be deferred to next frame).
pub fn emit_game_event(world: &mut World, game_event: GameEvent) {
    let frame = world.resource::<FrameCounter>().0;
    world.resource_mut::<EventQueue>().enqueue_game(game_event, EventSourceTag::Handler, frame + 1); // defers
}
