use bevy::prelude::*;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction2D { Up, Down, Left, Right }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    PrimaryAction,
    SecondaryAction,
    Move(Direction2D),
    Confirm,
    Cancel,
    SelectNext,
    SelectPrevious,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    SpawnBall,
    BallLostToHazard,
    TargetHit,
    TargetDestroyed,
    GameWon { balls_remaining: u32, time_elapsed: f32 },
    GameLost { targets_remaining: u32, time_elapsed: f32 },
    StartLevel { level_id: Option<String> },
    ResetLevel,
    PauseGame,
    ResumeGame,
    PlayerAction(PlayerAction),

    SpawnBallAtCursor { position: Vec2 },
    PlaceWidget { widget_type: WidgetType, position: Vec2 },
    SelectEntity { entity: Option<Entity> },
    DeleteEntity { entity: Entity },
    MoveEntity { entity: Entity, position: Vec2 },
    ClearArena,
    TogglePhysics,
    ChangeTool { mode: PlaygroundMode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum PlaygroundMode {
    SpawnBall,
    PlaceWall,
    PlaceTarget,
    PlaceHazard,
    PlacePaddle,
    PlaceSpawnPoint,
    Select,
    Delete,
}

impl Default for PlaygroundMode {
    fn default() -> Self { Self::SpawnBall }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetType {
    Wall { start: Vec2, end: Vec2, thickness: f32 },
    Target { health: u8, radius: f32 },
    Hazard { bounds: Rect },
    Paddle,
    SpawnPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    KeyDown(KeyCode),
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, PartialEq)]
pub enum DebugEvent {
    DespawnAllDynamic,
    ResetLevelFast,
}

#[derive(Clone)]
pub enum EventPayload {
    Game(GameEvent),
    Input(InputEvent),
    #[cfg(debug_assertions)]
    Debug(DebugEvent),
}
impl fmt::Debug for EventPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(g) => write!(f, "Game::{g:?}"),
            Self::Input(i) => write!(f, "Input::{i:?}"),
            #[cfg(debug_assertions)]
            Self::Debug(d) => write!(f, "Debug::{d:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSourceTag {
    System,
    Input,
    Handler,
    Test,
}

#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub payload: EventPayload,
    pub source: EventSourceTag,
    pub frame_enqueued: u64,
    pub timestamp_ns: u128,
}

impl EventEnvelope {
    pub fn new(payload: EventPayload, source: EventSourceTag, frame: u64) -> Self {
        Self { payload, source, frame_enqueued: frame, timestamp_ns: std::time::Instant::now().elapsed().as_nanos() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventResult { Handled, Ignored, Error(String) }

pub trait EventHandler: Send + Sync {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult;
    fn name(&self) -> &'static str;
}

#[derive(Resource, Default)]
pub struct HandlerRegistry { handlers: Vec<Box<dyn EventHandler>> }
impl HandlerRegistry {
    pub fn register<H: EventHandler + 'static>(&mut self, h: H) { self.handlers.push(Box::new(h)); }
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Box<dyn EventHandler>> { self.handlers.iter_mut() }
    pub fn dispatch(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        let mut any = false;
        for h in self.handlers.iter_mut() {
            match h.handle(ev, world) {
                EventResult::Handled => any = true,
                EventResult::Ignored => {},
                e @ EventResult::Error(_) => return e,
            }
        }
        if any { EventResult::Handled } else { EventResult::Ignored }
    }
}
