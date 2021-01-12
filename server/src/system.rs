use crate::component::*;
use crate::event::{ChangeMovement, CreatePlayer, RemovePlayer};
use crate::server::GameServer;
use crate::{View, TICK_TIME};
use bevy::app::{EventReader, Events};
use bevy::ecs::{Command, Commands, Entity, Local, Or, Query, Res, ResMut, Resources, With, World};
use game_shared::{CelestialView, PlayerView, Position, StaticView, ViewSnapshot, Status, EffectType,  CELESTIAL_RADIUS, INIT_RADIUS, MAP_HEIGHT, MAP_WIDTH, VIEW_X, VIEW_Y};
use rand::Rng;

use bevy_rapier2d::na::{Point2, Isometry, Translation, Rotation2};
use bevy_rapier2d::physics::{RapierConfiguration, RigidBodyHandleComponent, EventQueue};
use bevy_rapier2d::rapier::dynamics::{RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::{ColliderBuilder, ContactEvent, ColliderSet};
use bevy_rapier2d::rapier::ncollide::na::{Vector2, Unit};

use bevy::prelude::*;

const INIT_MASS: f32 = 1.0;
const INIT_RESTITUTION: f32 = 1.0;
const CELESTIAL_MASS: f32 = 10000000.0;
const GRAVITY_CONST: f32 = 1.0;

fn create_entity(commands: &mut Commands, role: Role, x: f32, y: f32, rigid_body_builder: RigidBodyBuilder, collider_builder: ColliderBuilder) -> Entity {
	let entity = commands.spawn((
		Thrust {x: 0.0, y: 0.0},
		Transform::from_translation(Vec3::new(x, y, 0.0)),
		Status {effects: Vec::new()},
	)).current_entity().unwrap();
	match role { Role::Player(name) => {
		commands.insert_one(entity, Player { name });
		commands.spawn(()).with(Parent(entity))
		},
		Role::Boundary(info) => commands.insert_one(entity, Boundary { info }),
		Role::CelestialBody(form) => commands.insert_one(entity, CelestialBody { form }),
		Role::Shape(id) => commands.insert_one(entity, Shape {id}),
	};
	commands.insert_one(entity, rigid_body_builder.translation(x, y).user_data(entity.to_bits() as u128));
	commands.spawn((collider_builder.user_data(true as u128),)).with(Parent(entity));
	entity
}

pub fn setup(commands: &mut Commands, mut configuration: ResMut<RapierConfiguration>) {
	let mut rng = rand::thread_rng();

	// Disable gravity.
	configuration.gravity = Vector2::new(0.0, 0.0);

	// Boundaries.
	create_entity(commands, Role::Boundary("Top".to_string()), 0.0, 0.0,
				  RigidBodyBuilder::new_static(),
				  ColliderBuilder::segment(Point2::new(0.0, 0.0), Point2::new(MAP_WIDTH, 0.0)).restitution(INIT_RESTITUTION));

	create_entity(commands, Role::Boundary("Left".to_string()), 0.0, 0.0,
				  RigidBodyBuilder::new_static(),
				  ColliderBuilder::segment(Point2::new(0.0, 0.0), Point2::new(0.0, MAP_HEIGHT)).restitution(INIT_RESTITUTION));

	create_entity(commands, Role::Boundary("Right".to_string()), 0.0, 0.0,
				  RigidBodyBuilder::new_static(),
				  ColliderBuilder::segment(Point2::new(MAP_WIDTH, 0.0), Point2::new(MAP_WIDTH, MAP_HEIGHT)).restitution(INIT_RESTITUTION));

	create_entity(commands, Role::Boundary("Bottom".to_string()), 0.0, 0.0,
				  RigidBodyBuilder::new_static(),
				  ColliderBuilder::segment(Point2::new(0.0, MAP_HEIGHT), Point2::new(MAP_WIDTH, MAP_HEIGHT)).restitution(INIT_RESTITUTION));

	// Random stuffs.
	for _ in 0..100 {
		let x = rng.gen_range(0.4 * MAP_WIDTH .. 0.6 * MAP_WIDTH);
		let y = rng.gen_range(0.4 * MAP_HEIGHT .. 0.6 * MAP_HEIGHT);
		let body = RigidBodyBuilder::new_dynamic().translation(x, y).mass(INIT_MASS, false);
		let collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
		create_entity(commands, Role::Shape(0), x, y, body, collider);
	}

	create_entity(commands, Role::CelestialBody("Planet".to_string()),
				  4029.99564, 5243.08753,
				  RigidBodyBuilder::new_dynamic().mass(CELESTIAL_MASS, false).linvel(46.62036850, 43.23657300),
				  ColliderBuilder::ball(CELESTIAL_RADIUS).restitution(INIT_RESTITUTION));
	create_entity(commands, Role::CelestialBody("Planet".to_string()),
				  5000.0, 5000.0,
				  RigidBodyBuilder::new_dynamic().mass(CELESTIAL_MASS, false).linvel(-93.240737, -86.473146),
				  ColliderBuilder::ball(CELESTIAL_RADIUS).restitution(INIT_RESTITUTION));
	create_entity(commands, Role::CelestialBody("Planet".to_string()),
				  5970.00436, 4756.91247,
				  RigidBodyBuilder::new_dynamic().mass(CELESTIAL_MASS, false).linvel(46.62036850, 43.23657300),
				  ColliderBuilder::ball(CELESTIAL_RADIUS).restitution(INIT_RESTITUTION));
}

pub fn create_player(
	commands: &mut Commands,
	mut events: ResMut<Events<CreatePlayer>>,
	mut game_state: ResMut<GameServer>,
) {
	let mut rng = rand::thread_rng();
	for event in events.drain() {
		let x = rng.gen_range(0.4 * MAP_WIDTH .. 0.6 * MAP_WIDTH);
		let y = rng.gen_range(0.4 * MAP_HEIGHT .. 0.6 * MAP_HEIGHT);
		let body = RigidBodyBuilder::new_dynamic().translation(x, y).mass(INIT_MASS * 100.0, false);
		let collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
		let entity = create_entity(commands, Role::Player(event.name.clone()), x, y,body, collider);
		game_state.sessions.insert(entity, event.session.clone());
		event.sender.send(entity).unwrap();
		println!("Player {} (#{}) joined the game.", event.name, entity.id());
	}
}

impl Command for ChangeMovement {
	fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
		let (fy, fx) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
		let mut thrust = world.get_mut::<Thrust>(self.player).expect("No component found.");
		thrust.x = fx * 40000.0;
		thrust.y = fy * 40000.0;
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
	mut rigid_body_set: ResMut<RigidBodySet>,
	celestial_query: Query<(&CelestialBody, &RigidBodyHandleComponent, &Transform)>,
	// player_query: Query<(&Thrust, &RigidBodyHandleComponent, &Transform), With<Player>>,
	object_query: Query<
		(&Thrust, &RigidBodyHandleComponent, &Transform),
		Or<(With<Player>, With<Shape>, With<CelestialBody>)>,
	>,
) {
	for (thrust, object_handle, object_transform) in object_query.iter() {
		let mut force = Vector2::new(thrust.x, thrust.y);
		let object_body = rigid_body_set.get(object_handle.handle()).unwrap();
		let object_mass = object_body.mass();
		for (_, celestial_handle, celestial_transform) in celestial_query.iter() {
			let celestial_mass = rigid_body_set.get(celestial_handle.handle()).unwrap().mass();
			let displacement_3d = celestial_transform.translation - object_transform.translation;
			let displacement: Vector2<f32> = Vector2::new(displacement_3d.x, displacement_3d.y);
			if displacement.norm() == 0.0 {
				continue;
			}
			force += GRAVITY_CONST * object_mass * celestial_mass * displacement
				/ displacement.norm().powi(3);
		}
		rigid_body_set.get_mut(object_handle.handle()).unwrap().apply_force(force, true);
	}
}

pub fn collisions(events: ResMut<EventQueue>, collider_set: Res<ColliderSet>, mut rigid_body_set: ResMut<RigidBodySet>) {
	while let Ok(contact_event) = events.contact_events.pop() {
		if let ContactEvent::Started(first_handle, second_handle) = contact_event {
			let first_body = rigid_body_set.get(collider_set.get(first_handle).unwrap().parent()).unwrap();
			let second_body = rigid_body_set.get( collider_set.get(second_handle).unwrap().parent()).unwrap();
			let mut first_entity = Entity::from_bits(first_body.user_data as u64);
			let mut second_entity = Entity::from_bits(second_body.user_data as u64);
		}
	}
}

pub fn extract_render_state(
	game_state: Res<GameServer>,
	query: Query<(Entity, &Player, &Transform)>,
	obj_query: Query<(Entity, &Shape, &Transform)>,
	celestial_query: Query<(Entity, &CelestialBody, &Transform)>,
) {
	for (entity, _player, self_pos) in query.iter() {
		// Collect players' positions.
		let positions = query
			.iter()
			.filter(|(_, _, pos)| {
				(self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
			})
			.map(|(entity, player, pos)| {
				(
					entity.to_bits(),
					PlayerView {
						name: player.name.clone(),
						pos: Position { x: pos.translation.x, y: pos.translation.y },
						ori: 0.0,
					},
				)
			})
			.collect();

		// Collect positions of static objects.
		let static_pos = obj_query
			.iter()
			.filter_map(|(entity, _, pos)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						StaticView { pos: Position { x: pos.translation.x, y: pos.translation.y } },
					))
				} else {
					None
				}
			})
			.collect();

		// Collect celestial positions.
		let celestial_pos = celestial_query
			.iter()
			.map(|(entity, _, pos)| {
				(entity.to_bits(), CelestialView { pos: Position { x: pos.translation.x, y: pos.translation.y } })
			})
			.collect();

		// Collect self position.
		let self_pos = Position { x: self_pos.translation.x, y: self_pos.translation.y };

		let state = ViewSnapshot {
			time: game_state.start_time.elapsed(),
			self_pos,
			players: positions,
			static_pos,
			celestial_pos,
		};
		game_state
			.sessions
			.get(&entity)
			.expect("Left player still alive")
			.do_send(View(state.clone()));
	}
}
