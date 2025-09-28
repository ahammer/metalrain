use crate::{Ball, Target};
use bevy::prelude::*;

#[derive(Event)]
pub struct BallSpawned(pub Entity, pub Ball);

#[derive(Event)]
pub struct TargetDestroyed(pub Entity, pub Target);

#[derive(Event, Default)]
pub struct GameWon;

#[derive(Event, Default)]
pub struct GameLost;
