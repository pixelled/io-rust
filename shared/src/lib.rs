use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAP_WIDTH: f32 = 10000.0;
pub const MAP_HEIGHT: f32 = 10000.0;
pub const VIEW_X: f32 = 2080.0;
pub const VIEW_Y: f32 = 1170.0;

pub const INIT_RADIUS: f32 = 20.0;
pub const SHIELD_RADIUS: f32 = 25.0;
pub const CELESTIAL_RADIUS: f32 = 100.0;

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

pub struct Ori {
	/// Radians.
	pub deg: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
	/// The direction player moves towards.
	pub dir: Option<f32>,
	/// The orientation of shield.
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
	pub shield_info: Vec<(u64, ShieldView)>,
	pub static_pos: Vec<(u64, StaticView)>,
	pub celestial_pos: Vec<(u64, CelestialView)>,
}

impl ViewSnapshot {
	pub fn new() -> Self {
		ViewSnapshot {
			time: Duration::from_nanos(0),
			self_pos: Position { x: 0.0, y: 0.0 },
			players: Vec::new(),
			shield_info: Vec::new(),
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

/// Parameters associated with the player's body.
#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerView {
	pub name: String,
	pub pos: Position,
	pub ori: f32,
	pub shield_id: u64,
}

impl PlayerView {
	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize RenderState.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to RenderState.")
	}
}

/// Parameters associated with shields.
#[derive(Clone, Serialize, Deserialize)]
pub struct ShieldView {
	pub pos: Position,
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
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EffectType {
	BodyDamage(f32, f32),
	ShieldDeflection(f32, f32),
	ShieldAbsorption(f32, f32),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Status {
	pub effects: Vec<EffectType>,
}

impl Status {
	pub fn serialize(&self) -> Vec<u8> {
		bincode::serialize(&self).expect("Cannot serialize Events.")
	}

	pub fn deserialize(data: &[u8]) -> Self {
		bincode::deserialize(data).expect("Cannot deserialize to Events.")
	}
}
