use crate::{EventEnvelope, EventPayload, InputEvent, GameEvent, PlayerAction, Direction2D};
use bevy::prelude::*;
use std::collections::HashMap;

pub trait Middleware: Send + Sync {
    fn name(&self) -> &'static str;
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope>;
}

#[derive(Resource, Default)]
pub struct MiddlewareChain { chain: Vec<Box<dyn Middleware>> }
impl MiddlewareChain {
    pub fn add<M: Middleware + 'static>(&mut self, mw: M) { self.chain.push(Box::new(mw)); }
    pub fn run(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> {
        let mut cur = ev;
        for mw in self.chain.iter_mut() {
            if let Some(next) = mw.process(cur) { cur = next; } else { return None; }
        }
        Some(cur)
    }
}

/// Simple filter middleware using predicate closure (boxed) mainly for tests / examples.
pub struct FilterMiddleware { predicate: Box<dyn Fn(&EventEnvelope) -> bool + Send + Sync>, name: &'static str }
impl FilterMiddleware { pub fn new(name: &'static str, pred: impl Fn(&EventEnvelope)->bool + Send + Sync + 'static) -> Self { Self { predicate: Box::new(pred), name } } }
impl Middleware for FilterMiddleware {
    fn name(&self) -> &'static str { self.name }
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> { if (self.predicate)(&ev) { Some(ev) } else { None } }
}

/// Key mapping middleware: transforms raw Input(KeyDown) to higher level Game events or PlayerActions.
pub struct KeyMappingMiddleware;
impl Middleware for KeyMappingMiddleware {
    fn name(&self) -> &'static str { "KeyMapping" }
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> {
        if let EventPayload::Input(InputEvent::KeyDown(code)) = &ev.payload {
            let mapped = match code {
                KeyCode::KeyR => Some(GameEvent::ResetLevel),
                KeyCode::KeyP => Some(GameEvent::PauseGame),
                KeyCode::ArrowUp | KeyCode::KeyW => Some(GameEvent::PlayerAction(PlayerAction::Move(Direction2D::Up))),
                KeyCode::ArrowDown | KeyCode::KeyS => Some(GameEvent::PlayerAction(PlayerAction::Move(Direction2D::Down))),
                KeyCode::ArrowLeft | KeyCode::KeyA => Some(GameEvent::PlayerAction(PlayerAction::Move(Direction2D::Left))),
                KeyCode::ArrowRight | KeyCode::KeyD => Some(GameEvent::PlayerAction(PlayerAction::Move(Direction2D::Right))),
                KeyCode::Space => Some(GameEvent::PlayerAction(PlayerAction::PrimaryAction)),
                _ => None,
            };
            if let Some(game_ev) = mapped { return Some(EventEnvelope { payload: EventPayload::Game(game_ev), ..ev }); }
        }
        Some(ev)
    }
}

fn event_kind_key(env: &EventEnvelope) -> &'static str {
    match &env.payload {
        EventPayload::Game(g) => match g {
            GameEvent::SpawnBall { .. } => "SpawnBall",
            GameEvent::BallLostToHazard { .. } => "BallLostToHazard",
            GameEvent::TargetHit { .. } => "TargetHit",
            GameEvent::TargetDestroyed { .. } => "TargetDestroyed",
            GameEvent::GameWon { .. } => "GameWon",
            GameEvent::GameLost { .. } => "GameLost",
            GameEvent::StartLevel { .. } => "StartLevel",
            GameEvent::ResetLevel => "ResetLevel",
            GameEvent::PauseGame => "PauseGame",
            GameEvent::ResumeGame => "ResumeGame",
            GameEvent::PlayerAction(_) => "PlayerAction",
        },
        EventPayload::Input(_) => "Input",
        #[cfg(debug_assertions)]
        EventPayload::Debug(_) => "Debug",
    }
}

/// Debounce duplicate events of same kind within a frame window.
pub struct DebounceMiddleware { window_frames: u64, last_seen: HashMap<&'static str, u64> }
impl DebounceMiddleware { pub fn new(window_frames: u64) -> Self { Self { window_frames, last_seen: HashMap::new() } } }
impl Middleware for DebounceMiddleware {
    fn name(&self) -> &'static str { "Debounce" }
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> {
        let key = event_kind_key(&ev);
        let frame = ev.frame_enqueued;
        if let Some(&last) = self.last_seen.get(key) { if frame.saturating_sub(last) <= self.window_frames { return None; } }
        self.last_seen.insert(key, frame);
        Some(ev)
    }
}

/// Cooldown middleware enforcing minimum frame separation per event kind.
pub struct CooldownMiddleware { cooldown: u64, last_processed: HashMap<&'static str, u64> }
impl CooldownMiddleware { pub fn new(cooldown_frames: u64) -> Self { Self { cooldown: cooldown_frames, last_processed: HashMap::new() } } }
impl Middleware for CooldownMiddleware {
    fn name(&self) -> &'static str { "Cooldown" }
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> {
        let key = event_kind_key(&ev);
        let frame = ev.frame_enqueued;
        if let Some(&last) = self.last_processed.get(key) { if frame < last + self.cooldown { return None; } }
        self.last_processed.insert(key, frame);
        Some(ev)
    }
}
