use actix::Addr;
use std::collections::HashMap;
use crate::WsSession;
use std::time::{Duration, Instant};
use bevy::ecs::Entity;
use crate::event::{CreatePlayer, ChangeMovement, RemovePlayer};
use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot::Sender;

pub struct GameServer {
    start_time: Instant,
    pub(crate) up_time: Duration,
    pub(crate) sessions: HashMap<Entity, Addr<WsSession>>,
}

impl GameServer {
    pub fn new() -> Self {
        GameServer {
            start_time: Instant::now(),
            up_time: Duration::from_nanos(0),
            sessions: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct GameProxy {
    s1: UnboundedSender<CreatePlayer>,
    s2: UnboundedSender<ChangeMovement>,
    s3: UnboundedSender<RemovePlayer>,
}

impl GameProxy {
    pub fn new(s1: UnboundedSender<CreatePlayer>, s2: UnboundedSender<ChangeMovement>, s3: UnboundedSender<RemovePlayer>) -> Self {
        GameProxy { s1, s2, s3 }
    }

    pub fn create_player(&mut self, name: String, sender: Sender<Entity>, session: Addr<WsSession>) {
        self.s1.unbounded_send(CreatePlayer { name, sender, session }).unwrap();
    }

    pub fn change_movement(&mut self, player: Option<Entity>, direction: Option<f32>) {
        if let Some(player) = player {
            self.s2.unbounded_send(ChangeMovement { player, direction }).unwrap();
        }
    }

    pub fn remove_player(&mut self, player: Option<Entity>) {
        if let Some(player) = player {
            self.s3.unbounded_send(RemovePlayer { player }).unwrap();
        }
    }
}
