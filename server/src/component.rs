use bevy_rapier2d::rapier::na::Vector2;

pub struct Player {
	pub name: String,
}

pub struct Thrust {
	pub x: f32,
	pub y: f32,
}

pub struct Boundary;

pub struct CelestialBody {
	pub form: String,
}

pub struct Shape {
	pub id: i8,
}
