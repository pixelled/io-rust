use bevy_rapier2d::rapier::na::Vector2;

pub enum Role {
	Player(String),
	Boundary(String),
	CelestialBody(String),
	Shape(i8),
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

pub struct Shape {
	pub id: i8,
}
