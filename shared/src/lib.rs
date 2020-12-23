use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Operation {
    Join(String),
    Update(Option<f32>),
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

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderState {
    pub time: Duration,
    pub positions: Vec<(String, Position)>,
}

impl RenderState {
    pub fn new() -> Self {
        RenderState {
            time: Duration::from_nanos(0),
            positions: Vec::new(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize State.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to State.")
    }
}
