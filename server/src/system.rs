use crate::component::*;
use crate::event::{ChangeMovement, CreatePlayer, RemovePlayer};
use crate::server::GameServer;
use crate::View;
use bevy::app::{EventReader, Events};
use bevy::ecs::{Command, Commands, Entity, Local, Or, Query, Res, ResMut, Resources, With, World};
use game_shared::{
	CelestialView, PlayerView, Position, StaticView, ViewSnapshot, CELESTIAL_RADIUS, INIT_RADIUS,
	MAP_HEIGHT, MAP_WIDTH,
};
use rand::Rng;

use bevy_rapier2d::na::Point2;
use bevy_rapier2d::physics::{RapierConfiguration, RigidBodyHandleComponent, EventQueue};
use bevy_rapier2d::rapier::dynamics::{RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::{ColliderBuilder, ContactEvent, ColliderSet};
use bevy_rapier2d::rapier::ncollide::na::Vector2;

use bevy::prelude::{Transform, Vec3};

const VIEW_X: f32 = 2080.0;
const VIEW_Y: f32 = 1170.0;
const INIT_MASS: f32 = 1.0;
const INIT_RESTITUTION: f32 = 1.0;
const CELESTIAL_MASS: f32 = 20000000.0;
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
		ColliderBuilder::segment(Point2::new(MAP_WIDTH, 0.0), Point2::new(MAP_WIDTH, MAP_HEIGHT))
			.restitution(INIT_RESTITUTION),
	));
	commands.spawn((
		Boundary,
		RigidBodyBuilder::new_static(),
		ColliderBuilder::segment(Point2::new(0.0, MAP_HEIGHT), Point2::new(MAP_WIDTH, MAP_HEIGHT))
			.restitution(INIT_RESTITUTION),
	));

	// Random stuffs.
	for _ in 0..1000 {
		let x = rng.gen_range(20.0..8000.0);
		let y = rng.gen_range(20.0..8000.0);
		let body = RigidBodyBuilder::new_dynamic().translation(x, y).mass(INIT_MASS, false);
		let body_collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
		commands.spawn((
			Shape { id: 0 },
			Thrust { x: 0.0, y: 0.0 },
			Transform::from_translation(Vec3::new(x, y, 0.0)),
			body,
			body_collider,
		));
	}

	commands.spawn((
		CelestialBody { form: "".to_string() },
		Transform::identity(),
		RigidBodyBuilder::new_static().translation(2000.0, 2000.0).mass(CELESTIAL_MASS, false),
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
		let x = rng.gen_range(500.0..3000.0);
		let y = rng.gen_range(500.0..3000.0);
		let body = RigidBodyBuilder::new_dynamic().translation(x, y).mass(INIT_MASS * 100.0, false);
		let body_collider = ColliderBuilder::ball(INIT_RADIUS).restitution(INIT_RESTITUTION);
		commands.spawn((
			Player { name: event.name.clone() },
			Thrust { x: 0.0, y: 0.0 },
			Transform::from_translation(Vec3::new(x, y, 0.0)),
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
		Or<(With<Player>, With<Shape>)>,
	>,
) {
	for (thrust, player_handle, player_transform) in object_query.iter() {
		let mut force = Vector2::new(thrust.x, thrust.y);
		let player_body = rigid_body_set.get(player_handle.handle()).unwrap();
		let player_mass = player_body.mass();
		for (_, celestial_handle, celestial_transform) in celestial_query.iter() {
			let celestial_mass = rigid_body_set.get(celestial_handle.handle()).unwrap().mass();
			let displacement_3d = celestial_transform.translation - player_transform.translation;
			let displacement: Vector2<f32> = Vector2::new(displacement_3d.x, displacement_3d.y);
			force += GRAVITY_CONST * player_mass * celestial_mass * displacement
				/ displacement.norm().powi(3);
		}
		rigid_body_set.get_mut(player_handle.handle()).unwrap().apply_force(force, true);
	}
}

pub fn collisions(events: ResMut<EventQueue>, collider_set: Res<ColliderSet>, mut rigid_body_set: ResMut<RigidBodySet>) {
	while let Ok(contact_event) = events.contact_events.pop() {
		if let ContactEvent::Started(first_handle, second_handle) = contact_event {
			let mut first_body = rigid_body_set.get_mut(collider_set.get(first_handle).unwrap().parent()).unwrap();
			let mut second_body = rigid_body_set.get_mut( collider_set.get(second_handle).unwrap().parent()).unwrap();
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
			.filter_map(|(entity, _, pos)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X + CELESTIAL_RADIUS
					&& (self_pos.translation.y - pos.translation.y).abs()
						< VIEW_Y + CELESTIAL_RADIUS
				{
					Some((
						entity.to_bits(),
						CelestialView {
							pos: Position { x: pos.translation.x, y: pos.translation.y },
						},
					))
				} else {
					None
				}
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
