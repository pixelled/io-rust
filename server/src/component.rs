pub struct Player {
    pub name: String,
}

pub struct Shape {
    pub id: i8,
}

pub use game_shared::Position;

pub use game_shared::Ori;

pub struct Velocity {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

pub struct Acceleration {
    pub(crate) x: f32,
    pub(crate) y: f32,
}
