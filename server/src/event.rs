use bevy::ecs::{Entity, ResMut};
use crate::component::Acceleration;
use futures::channel::oneshot::Sender;
use actix::Addr;
use futures::channel::mpsc::UnboundedReceiver;
use bevy::app::Events;
use crate::WsSession;

pub struct EventListener<T>(pub UnboundedReceiver<T>);

impl<T> EventListener<T> {
    pub fn new(receiver: UnboundedReceiver<T>) -> Self {
        EventListener(receiver)
    }

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
    pub(crate) direction: Option<f32>,
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
