use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

#[derive(Serialize, Deserialize)]
pub enum Movement {
    Up,
    Right,
    Down,
    Left,
    NoAction,
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
    acc: (f32, f32),
    vel: (f32, f32),
}

struct Velocity {
    x: f32,
    y: f32,
}

struct Acceleration {
    x: f32,
    y: f32,
}

impl Pos {
    pub fn new() -> Self {
        Pos {
            x: 100.0,
            y: 100.0,
            acc: (0.5, 0.5),
            vel: (0.0, 0.0),
        }
    }

    pub fn advance(&mut self, dir: Movement) {
        match dir {
            Movement::Up => self.vel.1 -= self.acc.1,
            Movement::Right => self.vel.0 += self.acc.0,
            Movement::Down => self.vel.1 += self.acc.1,
            Movement::Left => self.vel.0 -= self.acc.0,
            Movement::NoAction => {
                self.vel = (self.vel.0 * 0.96, self.vel.1 * 0.96);
            }
        }
        self.x += self.vel.0;
        self.y += self.vel.1;
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
        self.positions.push(("Peach".to_owned(), Pos::new()));
    }

    pub fn update(&mut self, commands: &mut VecDeque<Movement>) {
        if !commands.is_empty() {
            commands.drain(..).for_each(|m| self.positions[0].1.advance(m));
        } else {
            self.positions[0].1.advance(Movement::NoAction);
        }
    }
}
