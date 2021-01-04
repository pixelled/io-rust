use crate::WsSession;
use actix::Addr;
use bevy::app::Events;
use bevy::ecs::{Entity, ResMut};
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot::Sender;
use game_shared::PlayerState;

pub struct EventListener<T>(pub UnboundedReceiver<T>);

impl<T> EventListener<T> {
    pub fn next(&mut self) -> Option<T> {
        match self.0.try_next() {
            Ok(e) => e,
            Err(_) => None,
        }
    }
}

pub struct CreatePlayer {
    pub(crate) name: String,
    pub(crate) sender: Sender<Entity>,
    pub(crate) session: Addr<WsSession>,
}

pub struct RemovePlayer {
    pub player: Entity,
}

#[derive(Clone)]
pub struct ChangeMovement {
    pub(crate) player: Entity,
    pub(crate) state: PlayerState,
}

pub fn trigger_events(
    mut create_player_listener: ResMut<EventListener<CreatePlayer>>,
    mut change_movement_listener: ResMut<EventListener<ChangeMovement>>,
    mut remove_player_listener: ResMut<EventListener<RemovePlayer>>,
    mut create_player_events: ResMut<Events<CreatePlayer>>,
    mut change_movement_events: ResMut<Events<ChangeMovement>>,
    mut remove_player_events: ResMut<Events<RemovePlayer>>,
) {
    while let Some(event) = create_player_listener.next() {
        create_player_events.send(event);
    }
    while let Some(event) = change_movement_listener.next() {
        change_movement_events.send(event);
    }
    while let Some(event) = remove_player_listener.next() {
        remove_player_events.send(event);
    }
}
