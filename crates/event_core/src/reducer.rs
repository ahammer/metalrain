use crate::{
    EventEnvelope, EventPayload, EventQueue, EventResult, EventSourceTag, FrameCounter, GameEvent,
    HandlerRegistry, MiddlewareChain,
};
use bevy::prelude::*;

pub fn reducer_system(world: &mut World) {
    let frame_idx = world.resource::<FrameCounter>().0;
    let events: Vec<EventEnvelope> = {
        let mut q = world.resource_mut::<EventQueue>();
        q.drain_for_frame(frame_idx)
    };
    for env in events.into_iter() {
        let maybe_final = {
            let mut mw = world.resource_mut::<MiddlewareChain>();
            mw.run(env.clone())
        };
        let Some(final_env) = maybe_final else {
            continue;
        };
        let result = match &final_env.payload {
            EventPayload::Game(g) => {
                let mut result = EventResult::Ignored;
                world.resource_scope(|world, mut handlers: Mut<HandlerRegistry>| {
                    result = handlers.dispatch(g, world);
                });
                result
            }
            EventPayload::Input(_raw) => EventResult::Ignored,
            #[cfg(debug_assertions)]
            EventPayload::Debug(_d) => EventResult::Ignored,
        };
        {
            let mut q = world.resource_mut::<EventQueue>();
            q.push_journal(crate::queue::JournalEntry {
                event: final_env,
                result,
                frame_processed: frame_idx,
            });
        }
    }
    world.resource_mut::<EventQueue>().promote_next_frame();
}

pub fn emit_game_event(world: &mut World, game_event: GameEvent) {
    let frame = world.resource::<FrameCounter>().0;
    world
        .resource_mut::<EventQueue>()
        .enqueue_game(game_event, EventSourceTag::Handler, frame + 1);
}
