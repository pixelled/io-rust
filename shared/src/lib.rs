use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

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

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to Movement.")
    }
}

#[derive(Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    pub fn advance(&mut self, dir: Movement) {
        match dir {
            Movement::Up => self.y -= 10.0,
            Movement::Right => self.x += 10.0,
            Movement::Down => self.y += 10.0,
            Movement::Left => self.x -= 10.0,
        }
    }
}

pub type Id = String;

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub time: Duration,
    pub positions: Vec<(Id, Pos)>,
}

impl GameState {
    pub fn new() -> Self {
        GameState { time: Instant::now().elapsed(), positions: Vec::new() }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Cannot serialize State.")
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Cannot deserialize to State.")
    }

    pub fn add_player(&mut self) {
        self.positions.push(("Peach".to_owned(), Pos { x: 100.0, y: 100.0 }));
    }

    pub fn update(&mut self, commands: &mut VecDeque<Movement>) {
        if !commands.is_empty() {
            commands.drain(..).for_each(|m| self.positions[0].1.advance(m));
        }
    }
}
