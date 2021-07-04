use crate::component::*;
use crate::event::{ChangeMovement, CreatePlayer, RemovePlayer};
use crate::server::GameServer;
use crate::{View};
use bevy::app::{EventReader, Events};
use game_shared::{CelestialView, PlayerView, Position, StaticView, ViewSnapshot, Status, EffectType, CELESTIAL_RADIUS, INIT_RADIUS, MAP_HEIGHT, MAP_WIDTH, VIEW_X, VIEW_Y, SHIELD_RADIUS, ShieldView, Ori};
use rand::Rng;
use bevy_rapier2d::na::Vector2;
use bevy_rapier2d::physics::{RapierConfiguration, RigidBodyHandleComponent, RigidBodyBundle, ColliderBundle, RigidBodyPositionSync, JointBuilderComponent, JointHandleComponent};
use bevy_rapier2d::rapier::geometry::{ContactEvent, ColliderShape, ColliderMaterial, ColliderMassProps};
use bevy::prelude::*;
use bevy::ecs::system::Command;
use bevy_rapier2d::rapier::geometry::ColliderMassProps::MassProperties;
use bevy_rapier2d::rapier::dynamics::{RigidBodyType, RigidBodyVelocity, RigidBodyForces, RigidBodyMassProps, BallJoint, JointSet, JointHandle, JointParams, RigidBodyPosition};
use bevy_rapier2d::rapier::na::{UnitVector2, Rotation, Rotation2, Unit, UnitQuaternion, UnitComplex, Complex};

const INIT_MASS: f32 = 1.0;
const INIT_RESTITUTION: f32 = 1.0;
const INIT_DENSITY: f32 = 0.0008;

const PLAYER_DENSITY: f32 = 0.0008;
const SHIELD_DENSITY: f32 = 0.000008;

const CELESTIAL_MASS: f32 = 10000000.0;
const CELESTIAL_DENSITY: f32 = 318.3;

const GRAVITY_CONST: f32 = 20.0;

/// TODO: remove this one and generalize as trait?
fn create_entity(commands: &mut Commands, role: Role, x: f32, y: f32, rigid_body: RigidBodyBundle, collider: ColliderBundle) -> Entity {
	let entity = commands.spawn_bundle((
		Thrust {x: 0.0, y: 0.0},
		Ori { deg: 0.0 },
		Transform::from_translation(Vec3::new(x, y, 0.0)),
		Status {effects: Vec::new()},
	)).id();
	match role { Role::Player(name) => {
		commands.entity(entity).insert(Player { name })
		// commands.spawn(()).with(Parent(entity))
		},
		Role::Boundary(info) => commands.entity(entity).insert(Boundary { info }),
		Role::CelestialBody(form) => commands.entity(entity).insert(CelestialBody { form }),
		Role::Shape(id) => commands.entity(entity).insert(Shape {id}),
	};
	commands.entity(entity).insert_bundle(rigid_body).insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete);
	// commands.entity(entity).insert(rigid_body_builder.translation(x, y).user_data(entity.to_bits() as u128));
	// commands.spawn_bundle((collider_builder.user_data(true as u128),)).with(Parent(entity));

	entity
}

fn create_shield(commands: &mut Commands, shield_type: ShieldType, x: f32, y: f32, rigid_body: RigidBodyBundle, collider: ColliderBundle) -> Entity {
	let entity = commands
		.spawn_bundle((
			shield_type,
			Transform::from_translation(Vec3::new(x, y, 0.0)),
		))
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete)
		.id();
	entity
}

/// Create a segmented-shape boundary centered at (`x`, `y`).
fn create_seg_boundary(commands: &mut Commands, x: Vec2, y: Vec2) {
	let entity = commands.spawn_bundle((
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
		Status {effects: Vec::new()},
	)).id();
	let rigid_body = RigidBodyBundle {
		body_type: RigidBodyType::Static,
		..Default::default()
	};
	let collider = ColliderBundle {
		shape: ColliderShape::segment(x.into(), y.into()),
		material: ColliderMaterial {
			restitution: INIT_RESTITUTION,
			..Default::default()
		},
		..Default::default()
	};
	commands.entity(entity).insert_bundle(rigid_body).insert_bundle(collider)
		.insert(Boundary { info: "Seg".to_string() });
}

/// Create a planet centered at (`x`, `y`) with `linvel`.
fn create_planet(commands: &mut Commands, x: f32, y: f32, linvel: Vec2) {
	let entity = commands.spawn_bundle((
		Thrust {x: 0.0, y: 0.0},
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
	)).id();
	let rigid_body = RigidBodyBundle {
		position: Vec2::new(x, y).into(),
		velocity: RigidBodyVelocity {
			linvel: linvel.into(),
			..Default::default()
		},
		..Default::default()
	};
	let collider = ColliderBundle {
		shape: ColliderShape::ball(CELESTIAL_RADIUS),
		mass_properties: ColliderMassProps::Density(CELESTIAL_DENSITY),
		material: ColliderMaterial {
			restitution: INIT_RESTITUTION,
			..Default::default()
		},
		..Default::default()
	};
	commands.entity(entity).insert_bundle(rigid_body).insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete)
		.insert(CelestialBody { form: "planet".to_string() });
}

/// Basic setup at the beginning.
pub fn setup(mut commands: Commands, mut configuration: ResMut<RapierConfiguration>) {
	let mut rng = rand::thread_rng();

	// Disable gravity.
	configuration.gravity = Vector2::new(0.0, 0.0);

	// Create Boundaries.
	create_seg_boundary(&mut commands, Vec2::new(0.0, 0.0), Vec2::new(MAP_WIDTH, 0.0));
	create_seg_boundary(&mut commands, Vec2::new(0.0, 0.0), Vec2::new(0.0, MAP_HEIGHT));
	create_seg_boundary(&mut commands, Vec2::new(MAP_WIDTH, 0.0), Vec2::new(MAP_WIDTH, MAP_HEIGHT));
	create_seg_boundary(&mut commands, Vec2::new(0.0, MAP_HEIGHT), Vec2::new(MAP_WIDTH, MAP_HEIGHT));

	// Add Random stuffs.
	for _ in 0..100 {
		let x = rng.gen_range(0.4 * MAP_WIDTH .. 0.6 * MAP_WIDTH);
		let y = rng.gen_range(0.4 * MAP_HEIGHT .. 0.6 * MAP_HEIGHT);
		let rigid_body = RigidBodyBundle {
			position: Vec2::new(x, y).into(),
			..Default::default()
		};
		let collider = ColliderBundle {
			shape: ColliderShape::ball(INIT_RADIUS),
			mass_properties: ColliderMassProps::Density(INIT_DENSITY),
			material: ColliderMaterial {
				restitution: INIT_RESTITUTION,
				..Default::default()
			},
			..Default::default()
		};
		create_entity(&mut commands, Role::Shape(0), x, y, rigid_body, collider);
	}

	// Add Celestial objects.
	create_planet(&mut commands, 4029.99564, 5243.08753, Vec2::new(46.62036850, 43.23657300));
	create_planet(&mut commands, 5000.0, 5000.0, Vec2::new(-93.240737, -86.473146));
	create_planet(&mut commands, 5970.00436, 4756.91247, Vec2::new(46.62036850, 43.23657300));
}

/// Create players for a stream of [CreatePlayer] `events`.
pub fn create_player(
	mut commands: Commands,
	mut events: ResMut<Events<CreatePlayer>>,
	mut game_state: ResMut<GameServer>,
) {
	let mut rng = rand::thread_rng();
	for event in events.drain() {
		let x = rng.gen_range(0.4 * MAP_WIDTH .. 0.6 * MAP_WIDTH);
		let y = rng.gen_range(0.4 * MAP_HEIGHT .. 0.6 * MAP_HEIGHT);

		// The entity of player's body.
		let rigid_body = RigidBodyBundle {
			position: (Vec2::new(x, y), 0.0).into(),
			..Default::default()
		};
		let collider = ColliderBundle {
			shape: ColliderShape::ball(INIT_RADIUS),
			mass_properties: ColliderMassProps::Density(PLAYER_DENSITY),
			material: ColliderMaterial {
				restitution: INIT_RESTITUTION,
				..Default::default()
			},
			..Default::default()
		};
		let entity_body = create_entity(&mut commands, Role::Player(event.name.clone()), x, y, rigid_body, collider);

		// The entity of shield.
		let x_shield = x + 40.0;
		let y_shield = y;
		let rigid_body = RigidBodyBundle {
			position: Vec2::new(x_shield, y_shield).into(),
			..Default::default()
		};
		let collider = ColliderBundle {
			shape: ColliderShape::ball(SHIELD_RADIUS),
			mass_properties: ColliderMassProps::Density(SHIELD_DENSITY),
			material: ColliderMaterial {
				restitution: INIT_RESTITUTION,
				..Default::default()
			},
			..Default::default()
		};
		let entity_shield = create_shield(&mut commands, ShieldType::Circle, x_shield, y_shield, rigid_body, collider);

		commands.entity(entity_body).insert(ShieldID { entity: entity_shield } )
			.insert(Parent(entity_body));

		let mut joint = BallJoint::new(Vec2::ZERO.into(), Vec2::new(60.0, 0.0).into());
		joint.configure_motor_velocity(20.0, 0.5);
		commands.spawn().insert(JointBuilderComponent::new(joint, entity_body, entity_shield));

		game_state.sessions.insert(entity_body, event.session.clone());
		event.sender.send(entity_body).unwrap();
		println!("Player {} (#{} #{}) joined the game.", event.name, entity_body.id(), entity_shield.id());
	}
}

impl Command for ChangeMovement {
	fn write(self: Box<Self>, world: &mut World) {
		let (fy, fx) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
		let mut thrust = world.get_mut::<Thrust>(self.player).expect("No component found.");
		thrust.x = fx * 40000.0;
		thrust.y = fy * 40000.0;
		let mut ori = world.get_mut::<Ori>(self.player).expect("No component found.");
		ori.deg = self.state.ori;
	}
}

pub fn change_movement(mut commands: Commands, mut events: ResMut<Events<ChangeMovement>>) {
	for event in events.drain() {
		commands.add(event);
	}
}

pub fn remove_player(
	mut commands: Commands,
	mut reader: EventReader<RemovePlayer>,
) {
	for event in reader.iter() {
		commands.entity(event.player).despawn();
		// game_state.sessions.remove(&event.player);
		println!("Player (#{}) left the game.", event.player.id());
	}
}

pub fn simulate_shield(players: Query<(&Transform, &Ori, &ShieldID), With<Player>>,
					   mut shields: Query<(&Transform, &mut RigidBodyVelocity), Or<(With<ShieldType>, With<Player>)>>) {
	for (body_transform, ori, shield_id) in players.iter() {
		let (shield_transform, mut shield_rb_vel) = shields.get_mut(shield_id.entity).expect("Shield entity not found.");
		let diff = shield_transform.translation - body_transform.translation;
		let diff_ori = ori.deg - diff.y.atan2(diff.x);
		let angle = std::f32::consts::PI;

		if diff_ori.abs() < 0.1 {
			shield_rb_vel.linvel = Vec2::new(0.0, 0.0).into();
		} else {
			if (diff_ori > 0.0 && diff_ori < angle) || diff_ori < -angle {
				// Clockwise.
				shield_rb_vel.linvel = Vec2::new(-diff.y * 10.0, diff.x * 10.0).into();
			} else {
				shield_rb_vel.linvel = Vec2::new(diff.y * 10.0, -diff.x * 10.0).into();
			}
		}
	}
}

/// Simulate gravitational forces exerted by `celestial_bodies` on `object_bodies`.
/// TODO: include both `Player` and `Shape` (the performance behaves strangely?) and remove Thrust.
pub fn simulate(celestial_bodies: Query<(&Transform, &RigidBodyMassProps), With<CelestialBody>>,
				mut object_bodies: Query<(&Thrust, &Transform, &mut RigidBodyForces, &RigidBodyMassProps), With<Player>>
				/*mut object_bodies: Query<(&Thrust, &Transform, &mut RigidBodyForces, &RigidBodyMassProps), Or<(With<Player>, With<Shape>, With<CelestialBody>)>>*/) {
	for (thrust, obj_transform, mut obj_forces, obj_mprops) in object_bodies.iter_mut() {
		let mut forces = Vector2::new(thrust.x, thrust.y);
		let obj_mass = 1.0 / obj_mprops.local_mprops.inv_mass;
		// Compute gravitational forces.
		for (cb_transform, cb_mprops) in celestial_bodies.iter() {
			let cb_mass = 1.0 / cb_mprops.local_mprops.inv_mass;
			let disp3 = cb_transform.translation - obj_transform.translation;
			let disp2: Vector2<f32> = Vector2::new(disp3.x, disp3.y);
			if disp2.norm() == 0.0 {
				continue;
			}
			forces += GRAVITY_CONST * cb_mass * obj_mass * disp2 / disp2.norm().powi(3);
		}
		// Apply forces.
		obj_forces.force = forces;
	}
}

/*pub fn collisions(events: ResMut<EventQueue>, collider_set: Res<ColliderSet>, mut rigid_body_set: ResMut<RigidBodySet>) {
	while let Ok(contact_event) = events.contact_events.pop() {
		if let ContactEvent::Started(first_handle, second_handle) = contact_event {
			let first_body = rigid_body_set.get(collider_set.get(first_handle).unwrap().parent()).unwrap();
			let second_body = rigid_body_set.get( collider_set.get(second_handle).unwrap().parent()).unwrap();
			let mut first_entity = Entity::from_bits(first_body.user_data as u64);
			let mut second_entity = Entity::from_bits(second_body.user_data as u64);
		}
	}
}*/

pub fn extract_render_state(
	game_state: Res<GameServer>,
	query: Query<(Entity, &Player, &Transform, &ShieldID)>,
	shields: Query<(Entity, &ShieldType, &Transform)>,
	obj_query: Query<(Entity, &Shape, &Transform)>,
	celestial_query: Query<(Entity, &CelestialBody, &Transform)>,
) {
	for (entity, _player, self_pos, _shield_id) in query.iter() {
		// Collect players' positions.
		let positions = query
			.iter()
			.filter_map(|(entity, player, pos, shield_id)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						PlayerView {
							name: player.name.clone(),
							pos: Position { x: pos.translation.x, y: pos.translation.y },
							ori: { let (axis, angle) = pos.rotation.to_axis_angle(); axis[2] * angle },
							shield_id: shield_id.entity.to_bits(),
						}
					))
				} else {
					None
				}
			})
			.collect();

		let shield_info = shields
			.iter()
			.filter_map(|(entity, _, pos)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						ShieldView { pos: Position { x: pos.translation.x, y: pos.translation.y } },
					))
				} else {
					None
				}
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
			shield_info,
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
