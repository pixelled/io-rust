use std::collections::HashMap;
use std::time::Instant;

use actix::Addr;
use bevy::ecs::entity::Entity;
use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot::Sender;

use game_shared::PlayerState;

use crate::event::GameEvent;
use crate::WsSession;

pub struct GameServer {
	pub(crate) start_time: Instant,
	pub(crate) sessions: HashMap<Entity, Addr<WsSession>>,
}

impl GameServer {
	pub fn new() -> Self {
		GameServer { start_time: Instant::now(), sessions: HashMap::new() }
	}
}

#[derive(Clone)]
pub struct GameProxy {
	sender: UnboundedSender<GameEvent>,
}

impl GameProxy {
	pub fn new(sender: UnboundedSender<GameEvent>) -> Self {
		GameProxy { sender }
	}

	pub fn create_player(
		&mut self,
		name: String,
		sender: Sender<Entity>,
		session: Addr<WsSession>,
	) {
		self.sender.unbounded_send(GameEvent::CreatePlayer(name, sender, session)).unwrap();
	}

	pub fn change_movement(&mut self, player: Option<Entity>, state: PlayerState) {
		if let Some(player) = player {
			self.sender.unbounded_send(GameEvent::UpdatePlayer(player, state)).unwrap();
		}
	}

	pub fn remove_player(&mut self, player: Option<Entity>) {
		if let Some(player) = player {
			self.sender.unbounded_send(GameEvent::RemovePlayer(player)).unwrap();
		}
	}
}
