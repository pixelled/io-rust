use actix::Addr;
use bevy::ecs::entity::Entity;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot::Sender;

use game_shared::PlayerState;

use crate::WsSession;

pub struct EventListener(pub UnboundedReceiver<GameEvent>);

impl EventListener {
	pub fn drain(&mut self) -> Drain {
		Drain { source: self }
	}
}

pub struct Drain<'a> {
	source: &'a mut EventListener,
}

impl Iterator for Drain<'_> {
	type Item = GameEvent;

	fn next(&mut self) -> Option<Self::Item> {
		match self.source.0.try_next() {
			Ok(e) => e,
			Err(_) => None,
		}
	}
}

pub enum GameEvent {
	CreatePlayer(String, Sender<Entity>, Addr<WsSession>),
	RemovePlayer(Entity),
	UpdatePlayer(Entity, PlayerState),
}
