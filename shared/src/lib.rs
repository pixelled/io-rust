use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Operation {
    Join(String),
    Update(PlayerState),
    Leave,
}

impl Operation {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize Operation.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to Operation.")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

pub struct Ori {
    pub deg: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub dir: Option<f32>,
    pub ori: f32,
}

impl PlayerState {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize PlayerState.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to PlayerState.")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderState {
    pub time: Duration,
    pub self_pos: Position,
    pub positions: Vec<(String, Position, f32)>,
}

impl RenderState {
    pub fn new() -> Self {
        RenderState {
            time: Duration::from_nanos(0),
            self_pos: Position { x: 0.0, y: 0.0 },
            positions: Vec::new(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize RenderState.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
    }
}
