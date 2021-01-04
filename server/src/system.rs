use crate::component::*;
use crate::event::{ChangeMovement, CreatePlayer, RemovePlayer};
use crate::server::GameServer;
use crate::{PlayerView, TICK_TIME};
use bevy::app::{EventReader, Events};
use bevy::ecs::{Command, Commands, Entity, Local, Or, Query, Res, ResMut, Resources, With, World};
use game_shared::{Position, RenderState, MAP_HEIGHT, MAP_WIDTH};
use rand::Rng;

use bevy_rapier2d::na::Point2;
use bevy_rapier2d::physics::{RapierConfiguration, RigidBodyHandleComponent};
use bevy_rapier2d::rapier::dynamics::{RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use bevy_rapier2d::rapier::ncollide::na::Vector2;

use bevy::prelude::Transform;

const VIEW_X: f32 = 2080.0;
const VIEW_Y: f32 = 1170.0;
const INIT_MASS: f32 = 1.0;
const INIT_RADIUS: f32 = 20.0;
const INIT_RESTITUTION: f32 = 1.0;
const CELESTIAL_MASS: f32 = 20000000.0;
const CELESTIAL_RADIUS: f32 = 500.0;
const GRAVITY_CONST: f32 = 1.0;

pub fn setup(commands: &mut Commands, mut configuration: ResMut<RapierConfiguration>) {
    let mut rng = rand::thread_rng();

    // Disable gravity.
    configuration.gravity = Vector2::new(0.0, 0.0);

    // Boundaries.
    commands.spawn((
        Boundary,
        RigidBodyBuilder::new_static(),
        ColliderBuilder::segment(Point2::new(0.0, 0.0), Point2::new(MAP_WIDTH, 0.0))
            .restitution(INIT_RESTITUTION),
    ));
    commands.spawn((
        Boundary,
        RigidBodyBuilder::new_static(),
        ColliderBuilder::segment(Point2::new(0.0, 0.0), Point2::new(0.0, MAP_HEIGHT))
            .restitution(INIT_RESTITUTION),
    ));
    commands.spawn((
        Boundary,
        RigidBodyBuilder::new_static(),
        ColliderBuilder::segment(
            Point2::new(MAP_WIDTH, 0.0),
            Point2::new(MAP_WIDTH, MAP_HEIGHT),
        )
        .restitution(INIT_RESTITUTION),
    ));
    commands.spawn((
        Boundary,
        RigidBodyBuilder::new_static(),
        ColliderBuilder::segment(
            Point2::new(0.0, MAP_HEIGHT),
            Point2::new(MAP_WIDTH, MAP_HEIGHT),
        )
        .restitution(INIT_RESTITUTION),
    ));

    // Random stuffs.
    for _ in 0..100 {
        let x = rng.gen_range(500.0..1500.0);
        let y = rng.gen_range(500.0..1500.0);
        let body = RigidBodyBuilder::new_dynamic()
            .translation(x, y)
            .mass(INIT_MASS, false);
        let body_collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
        commands.spawn((Shape { id: 0 }, Transform::identity(), body, body_collider));
    }

    commands.spawn((
        CelestialBody {
            form: "".to_string(),
        },
        Transform::identity(),
        RigidBodyBuilder::new_static()
            .translation(2000.0, 2000.0)
            .mass(CELESTIAL_MASS, false),
        ColliderBuilder::ball(CELESTIAL_RADIUS).restitution(INIT_RESTITUTION),
    ));
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
        let body = RigidBodyBuilder::new_dynamic()
            .translation(x, y)
            .mass(INIT_MASS, false);
        let body_collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
        commands.spawn((
            Player {
                name: event.name.clone(),
            },
            Transform::identity(),
            body,
            body_collider,
        ));
        let entity = commands.current_entity().unwrap();
        game_state.sessions.insert(entity, event.session.clone());
        event.sender.send(entity).unwrap();
        println!("Player {} (#{}) joined the game.", event.name, entity.id());
    }
}

impl Command for ChangeMovement {
    fn write(self: Box<Self>, world: &mut World, resources: &mut Resources) {
        let rigid_body_handle = world
            .get_mut::<RigidBodyHandleComponent>(self.player)
            .expect("No component found.")
            .handle();
        let mut rigid_body_set = resources
            .get_mut::<RigidBodySet>()
            .expect("No resource found.");
        let rigid_body = rigid_body_set.get_mut(rigid_body_handle).unwrap();
        let (fy, fx) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
        rigid_body.apply_force(Vector2::new(fx * 1000.0, fy * 1000.0), true);
        // let mut ori = world.get_mut::<Ori>(self.player).expect("No component found.");
        // ori.deg = self.state.ori;
    }
}

pub fn change_movement(commands: &mut Commands, mut events: ResMut<Events<ChangeMovement>>) {
    for event in events.drain() {
        commands.add_command(event);
    }
}

pub fn remove_player(
    commands: &mut Commands,
    mut event_reader: Local<EventReader<RemovePlayer>>,
    events: Res<Events<RemovePlayer>>,
    _game_state: ResMut<GameServer>,
) {
    for event in event_reader.iter(&events) {
        commands.despawn(event.player);
        // game_state.sessions.remove(&event.player);
        println!("Player (#{}) left the game.", event.player.id());
    }
}

pub fn next_frame(
    mut game_state: ResMut<GameServer>,
    mut rigid_body_set: ResMut<RigidBodySet>,
    celestial_query: Query<(&CelestialBody, &RigidBodyHandleComponent, &Transform)>,
    player_query: Query<(&RigidBodyHandleComponent, &Transform), With<Player>>,
    // object_query: Query<(&RigidBodyHandleComponent, &Transform), Or<(With<Player>, With<Shape>)>>,
) {
    game_state.up_time += TICK_TIME;
    for (player_handle, player_transform) in player_query.iter() {
        let mut force = Vector2::new(0.0, 0.0);
        let player_body = rigid_body_set.get(player_handle.handle()).unwrap();
        let player_mass = player_body.mass();
        for (_, celestial_handle, celestial_transform) in celestial_query.iter() {
            let celestial_mass = rigid_body_set
                .get(celestial_handle.handle())
                .unwrap()
                .mass();
            let displacement_3d = celestial_transform.translation - player_transform.translation;
            let displacement: Vector2<f32> = Vector2::new(displacement_3d.x, displacement_3d.y);
            force += GRAVITY_CONST * player_mass * celestial_mass * displacement
                / displacement.norm().powi(3);
        }
        rigid_body_set
            .get_mut(player_handle.handle())
            .unwrap()
            .apply_impulse(force * TICK_TIME.as_secs_f32(), true);
    }
}

pub fn extract_render_state(
    game_state: Res<GameServer>,
    query: Query<(Entity, &Player, &Transform)>,
    obj_query: Query<(&Shape, &Transform)>,
    celestial_query: Query<(&CelestialBody, &Transform)>,
) {
    for (entity, _player, self_pos) in query.iter() {
        // Collect players' positions.
        let positions = query
            .iter()
            .filter(|(_, _, pos)| {
                (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
                    && (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
            })
            .map(|(_, player, pos)| {
                (
                    player.name.clone(),
                    Position {
                        x: pos.translation.x,
                        y: pos.translation.y,
                    },
                    0.0,
                )
            })
            .collect();

        // Collect positions of static objects.
        let static_pos = obj_query
            .iter()
            .filter(|(_, pos)| {
                (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
                    && (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
            })
            .map(|(_, pos)| Position {
                x: pos.translation.x,
                y: pos.translation.y,
            })
            .collect();

        // Collect celestial positions.
        let celestial_pos = celestial_query
            .iter()
            .filter_map(|(_, pos)| {
                if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X + CELESTIAL_RADIUS
                    && (self_pos.translation.y - pos.translation.y).abs()
                        < VIEW_Y + CELESTIAL_RADIUS
                {
                    Some(Position {
                        x: pos.translation.x,
                        y: pos.translation.y,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Collect self position.
        let self_pos = Position {
            x: self_pos.translation.x,
            y: self_pos.translation.y,
        };

        let state = RenderState {
            time: game_state.up_time,
            self_pos,
            positions,
            static_pos,
            celestial_pos,
        };
        game_state
            .sessions
            .get(&entity)
            .expect("Left player still alive")
            .do_send(PlayerView(state.clone()));
    }
}
