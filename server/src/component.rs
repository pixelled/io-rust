pub struct Player {
    pub name: String,
}

pub struct CelestialBody {
    pub form: String,
}

pub struct Shape {
    pub id: i8,
}

pub use game_shared::Position;

pub use game_shared::Ori;

pub struct Force {
    pub(crate) x: f32,
    pub(crate) y: f32,
}
