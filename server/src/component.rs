use bevy::ecs::prelude::Entity;

pub enum Shape {
	Circle,
}

pub struct ShieldID {
	pub entity: Entity,
}

pub enum ShieldType {
	Circle,
}

pub struct Player {
	pub name: String,
}

pub struct Thrust {
	pub x: f32,
	pub y: f32,
}

pub struct Boundary {
	pub info: String,
}

pub struct CelestialBody {
	pub form: String,
}
