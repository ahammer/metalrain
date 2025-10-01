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

/// Output of a key mapping: either a direct GameEvent or a PlayerAction wrapped into a GameEvent.
#[derive(Clone)]
pub enum KeyMappingOutput { Game(GameEvent), Action(PlayerAction) }

/// Configurable key mapping middleware: transforms raw Input(KeyDown) into higher level events/actions.
pub struct KeyMappingMiddleware {
    mappings: HashMap<KeyCode, KeyMappingOutput>,
    name: &'static str,
}

impl KeyMappingMiddleware {
    /// Create an empty mapping set. No keys transformed until `map` is called.
    pub fn empty() -> Self { Self { mappings: HashMap::new(), name: "KeyMapping" } }
    /// Convenience constructor replicating previous hardcoded default gameplay mapping.
    pub fn with_default_gameplay() -> Self {
        let mut km = Self::empty();
        km.map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel))
          .map(KeyCode::KeyP, KeyMappingOutput::Game(GameEvent::PauseGame))
          .map_many(&[KeyCode::ArrowUp, KeyCode::KeyW], KeyMappingOutput::Action(PlayerAction::Move(Direction2D::Up)))
          .map_many(&[KeyCode::ArrowDown, KeyCode::KeyS], KeyMappingOutput::Action(PlayerAction::Move(Direction2D::Down)))
          .map_many(&[KeyCode::ArrowLeft, KeyCode::KeyA], KeyMappingOutput::Action(PlayerAction::Move(Direction2D::Left)))
          .map_many(&[KeyCode::ArrowRight, KeyCode::KeyD], KeyMappingOutput::Action(PlayerAction::Move(Direction2D::Right)))
          .map(KeyCode::Space, KeyMappingOutput::Action(PlayerAction::PrimaryAction));
        km
    }
    /// Map a single key to an output (overwrites existing mapping).
    pub fn map(&mut self, key: KeyCode, out: KeyMappingOutput) -> &mut Self { self.mappings.insert(key, out); self }
    /// Map multiple keys to the same output.
    pub fn map_many(&mut self, keys: &[KeyCode], out: KeyMappingOutput) -> &mut Self { for k in keys { self.mappings.insert(*k, out.clone()); } self }
    /// Replace entire mapping (builder-style chaining support).
    pub fn set_mappings(&mut self, map: HashMap<KeyCode, KeyMappingOutput>) -> &mut Self { self.mappings = map; self }
}

impl Middleware for KeyMappingMiddleware {
    fn name(&self) -> &'static str { self.name }
    fn process(&mut self, ev: EventEnvelope) -> Option<EventEnvelope> {
        if let EventPayload::Input(InputEvent::KeyDown(code)) = &ev.payload {
            if let Some(out) = self.mappings.get(code).cloned() {
                let game_ev = match out { KeyMappingOutput::Game(g) => g, KeyMappingOutput::Action(a) => GameEvent::PlayerAction(a) };
                return Some(EventEnvelope { payload: EventPayload::Game(game_ev), ..ev });
            }
        }
        Some(ev)
    }
}

fn event_kind_key(env: &EventEnvelope) -> &'static str {
    match &env.payload {
        EventPayload::Game(g) => match g {
            GameEvent::SpawnBall => "SpawnBall",
            GameEvent::BallLostToHazard => "BallLostToHazard",
            GameEvent::TargetHit => "TargetHit",
            GameEvent::TargetDestroyed => "TargetDestroyed",
            GameEvent::GameWon { .. } => "GameWon",
            GameEvent::GameLost { .. } => "GameLost",
            GameEvent::StartLevel { .. } => "StartLevel",
            GameEvent::ResetLevel => "ResetLevel",
            GameEvent::PauseGame => "PauseGame",
            GameEvent::ResumeGame => "ResumeGame",
            GameEvent::PlayerAction(_) => "PlayerAction",
            GameEvent::SpawnBallAtCursor { .. } => "SpawnBallAtCursor",
            GameEvent::PlaceWidget { .. } => "PlaceWidget",
            GameEvent::SelectEntity { .. } => "SelectEntity",
            GameEvent::DeleteEntity { .. } => "DeleteEntity",
            GameEvent::MoveEntity { .. } => "MoveEntity",
            GameEvent::ClearArena => "ClearArena",
            GameEvent::TogglePhysics => "TogglePhysics",
            GameEvent::ChangeTool { .. } => "ChangeTool",
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
