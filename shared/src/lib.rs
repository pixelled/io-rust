use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Movement {
    Up,
    Right,
    Down,
    Left,
}

impl Movement {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize Movement.")
    }
}

#[derive(Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

pub type Id = u32;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub time: Duration,
    pub positions: Vec<(Id, Pos)>,
}

impl State {
    pub fn new() -> Self {
        State { time: Instant::now().elapsed(), positions: Vec::new() }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize State.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to State.")
    }
}
