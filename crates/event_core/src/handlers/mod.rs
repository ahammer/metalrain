use crate::{EventHandler, EventResult, GameEvent};
use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct BallCounter {
    pub balls: u32,
}
#[derive(Resource, Default, Debug)]
pub struct TargetCounter {
    pub targets: u32,
}

pub struct BallLifecycleHandler;
impl EventHandler for BallLifecycleHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        match ev {
            GameEvent::SpawnBall => {
                let mut c = world.get_resource_or_insert_with::<BallCounter>(Default::default);
                c.balls += 1;
                EventResult::Handled
            }
            GameEvent::BallLostToHazard => {
                let mut c = world.get_resource_or_insert_with::<BallCounter>(Default::default);
                if c.balls > 0 {
                    c.balls -= 1;
                }
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
    fn name(&self) -> &'static str {
        "BallLifecycleHandler"
    }
}

pub struct TargetInteractionHandler;
impl EventHandler for TargetInteractionHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        match ev {
            GameEvent::TargetHit => EventResult::Handled,
            GameEvent::TargetDestroyed => {
                let mut t = world.get_resource_or_insert_with::<TargetCounter>(Default::default);
                if t.targets > 0 {
                    t.targets -= 1;
                }
                EventResult::Handled
            }
            GameEvent::StartLevel { .. } | GameEvent::ResetLevel => {
                world.insert_resource(TargetCounter { targets: 5 });
                world.insert_resource(BallCounter { balls: 0 });
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
    fn name(&self) -> &'static str {
        "TargetInteractionHandler"
    }
}
