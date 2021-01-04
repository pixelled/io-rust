use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAP_WIDTH: f32 = 10000.0;
pub const MAP_HEIGHT: f32 = 10000.0;

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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
	pub x: f32,
	pub y: f32,
}

impl Position {
	pub fn interpolate(prev: &Position, next: &Position, t: f32) -> Position {
		Position { x: (1.0 - t) * prev.x + t * next.x, y: (1.0 - t) * prev.y + t * next.y }
	}
}

pub struct Ori {
	pub deg: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
pub struct ViewSnapshot {
	pub time: Duration,
	pub self_pos: Position,
	pub players: Vec<(u64, PlayerView)>,
	pub static_pos: Vec<(u64, StaticView)>,
	pub celestial_pos: Vec<(u64, CelestialView)>,
}

impl ViewSnapshot {
	pub fn new() -> Self {
		ViewSnapshot {
			time: Duration::from_nanos(0),
			self_pos: Position { x: 0.0, y: 0.0 },
			players: Vec::new(),
			static_pos: Vec::new(),
			celestial_pos: Vec::new(),
		}
	}

	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize RenderState.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerView {
	pub name: String,
	pub pos: Position,
	pub ori: f32,
}

impl PlayerView {
	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize RenderState.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
	}

	pub fn interpolate(prev: &PlayerView, next: &PlayerView, t: f32) -> PlayerView {
		PlayerView {
			name: prev.name.clone(),
			pos: Position::interpolate(&prev.pos, &next.pos, t),
			ori: prev.ori,
		}
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StaticView {
	pub pos: Position,
}

impl StaticView {
	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize RenderState.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
	}

	pub fn interpolate(prev: &StaticView, next: &StaticView, t: f32) -> StaticView {
		StaticView { pos: Position::interpolate(&prev.pos, &next.pos, t) }
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CelestialView {
	pub pos: Position,
}

impl CelestialView {
	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize RenderState.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
	}

	pub fn interpolate(prev: &CelestialView, next: &CelestialView, t: f32) -> CelestialView {
		CelestialView { pos: Position::interpolate(&prev.pos, &next.pos, t) }
	}
}
