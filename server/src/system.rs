use crate::{TICK_TIME, PlayerView};
use crate::component::*;
use crate::event::{CreatePlayer, ChangeMovement, RemovePlayer};
use bevy::app::{Events, EventReader};
use bevy::ecs::{Query, Commands, ResMut, Local, Res, Command, World, Resources};
use game_shared::{RenderState, Ori};
use crate::server::GameServer;
use rand::Rng;

use bevy_rapier2d::na::Point2;
use bevy_rapier2d::physics::{JointBuilderComponent, RapierPhysicsPlugin, RigidBodyHandleComponent, ColliderHandleComponent};
use bevy_rapier2d::rapier::dynamics::{BallJoint, RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;

const MAP_WIDTH: f32 = 16000.0;
const MAP_HEIGHT: f32 = 9000.0;
const VIEW_X: f32 = 2080.0;
const VIEW_Y: f32 = 1170.0;
const INIT_MASS: f32 = 1.0;
const INIT_RADIUS: f32 = 20.0;

pub fn setup(commands: &mut Commands) {

}

pub fn create_player(
    commands: &mut Commands,
    mut events: ResMut<Events<CreatePlayer>>,
    mut game_state: ResMut<GameServer>,
) {
    let mut rng = rand::thread_rng();
    for event in events.drain() {
        let x = rng.gen_range(500.0..1500.0);
        let y = rng.gen_range(500.0..1500.0);
        let body = RigidBodyBuilder::new_dynamic().translation(x, y);
        body.mass(INIT_MASS, true);
        let body_collider = ColliderBuilder::ball(INIT_RADIUS);
        commands.spawn((
            Player { name: event.name.clone() },
            body,
            body_collider,
            Position {x, y},
            Ori { deg: 0.0 },
            Force { x: 0.0, y: 0.0 },
        ));
        let entity = commands.current_entity().unwrap();
        game_state.sessions.insert(entity, event.session.clone());
        event.sender.send(entity);
        println!("Player {} (#{}) joined the game.", event.name, entity.id());
    }
}

impl Command for ChangeMovement {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        let mut force = world.get_mut::<Force>(self.player).expect("No component found.");
        let (fy, fx) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
        force.x = fx * 1000.0;
        force.y = fy * 1000.0;
        let mut ori = world.get_mut::<Ori>(self.player).expect("No component found.");
        ori.deg = self.state.ori;
    }
}

pub fn change_movement(
    commands: &mut Commands,
    mut events: ResMut<Events<ChangeMovement>>,
) {
    for event in events.drain() {
        commands.add_command(event);
    }
}

pub fn remove_player(
    commands: &mut Commands,
    mut event_reader: Local<EventReader<RemovePlayer>>,
    events: Res<Events<RemovePlayer>>,
    mut game_state: ResMut<GameServer>,
) {
    for event in event_reader.iter(&events) {
        commands.despawn(event.player);
        // game_state.sessions.remove(&event.player);
        println!("Player (#{}) left the game.", event.player.id());
    }
}

pub fn next_frame(mut game_state: ResMut<GameServer>,
                  mut rigid_body_set: ResMut<RigidBodySet>,
                  mut player_query: Query<(&Player,
                                    &RigidBodyHandleComponent,
                                    &mut Position,
                                    &mut Force)>) {
    game_state.up_time += TICK_TIME;
    let dt = TICK_TIME.as_secs_f32();
    for (_, rbh, mut pos, mut force) in player_query.iter_mut() {
        let mut body = rigid_body_set.get_mut(rbh.handle()).unwrap();
        body.apply_force(force, true);
        pos.x = body.position().translation.vector.x();
        pos.y = body.position().translation.vector.y();
    }
}

pub fn extract_render_state(
    game_state: Res<GameServer>,
    query: Query<(bevy::ecs::Entity, &Player, &Position, &Ori)>,
    obj_query: Query<(&Shape, &Position)>,
) {
    for (entity, player, self_pos, ori) in query.iter() {
        let positions: Vec<(String, Position, f32)> = query.iter().filter(|(_, _, pos, _)| {
            (self_pos.x - pos.x).abs() < VIEW_X && (self_pos.y - pos.y).abs() < VIEW_Y
        }).map(|(_, player, pos, ori)| {
            (player.name.clone(), pos.clone(), ori.deg)
        }).collect();
        let static_pos: Vec<Position> = obj_query.iter().filter(|(_, pos)| {
            (self_pos.x - pos.x).abs() < VIEW_X && (self_pos.y - pos.y).abs() < VIEW_Y
        }).map(|(_, pos)| {
            pos.clone()
        }).collect();
        let state = RenderState { time: game_state.up_time, self_pos: self_pos.clone(), positions, static_pos};
        game_state.sessions.get(&entity).expect("Left player still alive").do_send(PlayerView(state.clone()));
    }
}