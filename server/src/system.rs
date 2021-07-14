use bevy::ecs::system::Command;
use bevy::prelude::*;
use bevy_rapier2d::na::Vector2;
use bevy_rapier2d::physics::{
	ColliderBundle, JointBuilderComponent, JointHandleComponent, RapierConfiguration,
	RigidBodyBundle, RigidBodyPositionSync,
};
use bevy_rapier2d::prelude::{RigidBodyCcd, IntoEntity};
use bevy_rapier2d::rapier::dynamics::{
	JointParams, JointSet, PrismaticJoint, RigidBodyForces, RigidBodyMassProps, RigidBodyType,
	RigidBodyVelocity,
};
use bevy_rapier2d::rapier::geometry::{ColliderMassProps, ColliderMaterial, ColliderShape};
use bevy_rapier2d::rapier::na::Vector;
use rand::prelude::ThreadRng;
use rand::Rng;

use game_shared::{
	CelestialView, Ori, PlayerState, PlayerView, Position, ShieldView, StaticView, Status,
	ViewSnapshot, CELESTIAL_RADIUS, INIT_RADIUS, MAP_HEIGHT, MAP_WIDTH, SHIELD_RADIUS, VIEW_X,
	VIEW_Y,
};

use crate::component::*;
use crate::event::{EventListener, GameEvent};
use crate::server::GameServer;
use crate::{View, WsSession};
use actix::Addr;
use futures::channel::oneshot::Sender;
use bevy_rapier2d::rapier::pipeline::ActiveEvents;
use bevy_rapier2d::rapier::prelude::ContactEvent;

const INIT_MASS: f32 = 1.0;
const INIT_RESTITUTION: f32 = 1.0;
const INIT_DENSITY: f32 = 0.0008;

const PLAYER_DENSITY: f32 = 0.0008;
const SHIELD_DENSITY: f32 = 0.000008;

const CELESTIAL_MASS: f32 = 10000000.0;
const CELESTIAL_DENSITY: f32 = 318.3;

const GRAVITY_CONST: f32 = 20.0;

/// TODO: generalize `create_[...]` as a trait?
fn create_body(
	commands: &mut Commands,
	name: String,
	x: f32,
	y: f32,
	rigid_body: RigidBodyBundle,
	collider: ColliderBundle,
) -> Entity {
	commands
		.spawn_bundle((
			Player { name },
			Thrust { x: 0.0, y: 0.0 },
			Ori { deg: 0.0, push: false },
			Transform::from_translation(Vec3::new(x, y, 0.0)),
			Dmg { val: 1 },
			HP { val: 100 },
		))
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete)
		.id()
}

/// Create a shield of `shield_type`.
fn create_shield(
	commands: &mut Commands,
	shield_type: ShieldType,
	x: f32,
	y: f32,
	rigid_body: RigidBodyBundle,
	collider: ColliderBundle,
) -> Entity {
	commands
		.spawn_bundle((
			shield_type,
			Transform::from_translation(Vec3::new(x, y, 0.0)),
			Dmg { val: 1 },
			HP { val: 100 },
		))
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete)
		.id()
}

/// Create a geometric objects with `shape`.
fn create_object(
	commands: &mut Commands,
	shape: Shape,
	x: f32,
	y: f32,
	rigid_body: RigidBodyBundle,
	collider: ColliderBundle,
) -> Entity {
	commands
		.spawn_bundle((
			shape,
			Transform::from_translation(Vec3::new(x, y, 0.0)),
			Dmg { val: 1 },
			HP { val: 100 },
		))
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
		.insert(RigidBodyPositionSync::Discrete)
		.id()
}

/// Create a segmented-shape boundary centered at (`x`, `y`).
fn create_seg_boundary(commands: &mut Commands, x: Vec2, y: Vec2) {
	let entity = commands
		.spawn_bundle((
			Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			Dmg { val: 1 },
			HP { val: 100 },
		))
		.id();
	let rigid_body = RigidBodyBundle { body_type: RigidBodyType::Static, ..Default::default() };
	let collider = ColliderBundle {
		shape: ColliderShape::segment(x.into(), y.into()),
		material: ColliderMaterial { restitution: INIT_RESTITUTION, ..Default::default() },
		..Default::default()
	};
	commands
		.entity(entity)
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
		.insert(Boundary { info: "Seg".to_string() });
}

/// Create a planet centered at (`x`, `y`) with `linvel`.
fn create_planet(commands: &mut Commands, x: f32, y: f32, linvel: Vec2) {
	let entity = commands
		.spawn_bundle((
			Thrust { x: 0.0, y: 0.0 },
			Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			Dmg { val: 1 },
			HP { val: 100 },
		))
		.id();
	let rigid_body = RigidBodyBundle {
		position: Vec2::new(x, y).into(),
		velocity: RigidBodyVelocity { linvel: linvel.into(), ..Default::default() },
		..Default::default()
	};
	let collider = ColliderBundle {
		shape: ColliderShape::ball(CELESTIAL_RADIUS),
		mass_properties: ColliderMassProps::Density(CELESTIAL_DENSITY),
		material: ColliderMaterial { restitution: INIT_RESTITUTION, ..Default::default() },
		..Default::default()
	};
	commands
		.entity(entity)
		.insert_bundle(rigid_body)
		.insert_bundle(collider)
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
	create_seg_boundary(
		&mut commands,
		Vec2::new(0.0, MAP_HEIGHT),
		Vec2::new(MAP_WIDTH, MAP_HEIGHT),
	);

	// Add Random stuffs.
	for _ in 0..100 {
		let x = rng.gen_range(0.4 * MAP_WIDTH..0.6 * MAP_WIDTH);
		let y = rng.gen_range(0.4 * MAP_HEIGHT..0.6 * MAP_HEIGHT);
		let rigid_body = RigidBodyBundle {
			position: Vec2::new(x, y).into(),
			ccd: RigidBodyCcd { ccd_enabled: true, ..Default::default() },
			..Default::default()
		};
		let collider = ColliderBundle {
			shape: ColliderShape::ball(INIT_RADIUS),
			mass_properties: ColliderMassProps::Density(INIT_DENSITY),
			material: ColliderMaterial { restitution: INIT_RESTITUTION, ..Default::default() },
			..Default::default()
		};
		create_object(&mut commands, Shape::Circle, x, y, rigid_body, collider);
	}

	// Add Celestial objects.
	create_planet(&mut commands, 4029.99564, 5243.08753, Vec2::new(46.62036850, 43.23657300));
	create_planet(&mut commands, 5000.0, 5000.0, Vec2::new(-93.240737, -86.473146));
	create_planet(&mut commands, 5970.00436, 4756.91247, Vec2::new(46.62036850, 43.23657300));
}

pub fn handle_events(
	mut commands: Commands,
	mut game_state: ResMut<GameServer>,
	mut events: ResMut<EventListener>,
) {
	let mut rng = rand::thread_rng();
	for event in events.drain() {
		match event {
			GameEvent::CreatePlayer(name, sender, session) => {
				create_player(&mut commands, name, sender, session, &mut rng, &mut *game_state)
			}
			GameEvent::RemovePlayer(player) => {
				commands.entity(player).despawn();
			}
			GameEvent::UpdatePlayer(player, state) => {
				commands.add(ChangeMovement { player, state });
			}
		}
	}
}

/// Create players for a stream of [CreatePlayer] `events`.
fn create_player(
	commands: &mut Commands,
	name: String,
	sender: Sender<Entity>,
	session: Addr<WsSession>,
	rng: &mut ThreadRng,
	game_state: &mut GameServer,
) {
	let x = rng.gen_range(0.4 * MAP_WIDTH..0.6 * MAP_WIDTH);
	let y = rng.gen_range(0.4 * MAP_HEIGHT..0.6 * MAP_HEIGHT);

	// The entity of player's body.
	let rigid_body = RigidBodyBundle {
		position: (Vec2::new(x, y), 0.0).into(),
		ccd: RigidBodyCcd { ccd_enabled: true, ..Default::default() },
		..Default::default()
	};
	let collider = ColliderBundle {
		shape: ColliderShape::ball(INIT_RADIUS),
		mass_properties: ColliderMassProps::Density(PLAYER_DENSITY),
		material: ColliderMaterial { restitution: INIT_RESTITUTION, ..Default::default() },
		flags: (ActiveEvents::CONTACT_EVENTS).into(),
		..Default::default()
	};
	let entity_body = create_body(commands, name.clone(), x, y, rigid_body, collider);

	// The entity of shield.
	let x_shield = x + 40.0;
	let y_shield = y;
	let rigid_body = RigidBodyBundle {
		position: Vec2::new(x_shield, y_shield).into(),
		ccd: RigidBodyCcd { ccd_enabled: true, ..Default::default() },
		..Default::default()
	};
	let collider = ColliderBundle {
		shape: ColliderShape::ball(SHIELD_RADIUS),
		mass_properties: ColliderMassProps::Density(SHIELD_DENSITY),
		material: ColliderMaterial { restitution: INIT_RESTITUTION, ..Default::default() },
		flags: (ActiveEvents::CONTACT_EVENTS).into(),
		..Default::default()
	};
	let entity_shield =
		create_shield(commands, ShieldType::Circle, x_shield, y_shield, rigid_body, collider);

	commands.entity(entity_body).insert(ShieldID { entity: entity_shield });

	// Create a prismatic joint connecting the body and the shield.
	let x = Vector::x_axis();
	let mut joint = PrismaticJoint::new(Vec2::ZERO.into(), x, Vec2::new(0.0, 0.0).into(), x);
	// The shield is limited to 20~80 px away from the body.
	joint.limits = [-80.0, -20.0];
	commands.spawn().insert(JointBuilderComponent::new(joint, entity_body, entity_shield));

	game_state.sessions.insert(entity_body, session.clone());
	sender.send(entity_body).unwrap();
	println!("Player {} (#{} #{}) joined the game.", name, entity_body.id(), entity_shield.id());
}

/// Rotate shields towards the cursor's position `Ori.deg`.
pub fn rotate_shield(
	mut players: Query<(&Transform, &Ori, &mut RigidBodyVelocity)>
) {
	for (body_transform, ori, mut player_vel) in players.iter_mut() {
		let (axis, angle) = body_transform.rotation.to_axis_angle();
		let mut ori_body = axis[2] * angle;
		let pi = std::f32::consts::PI;

		// Transform `ori_body` to the space of `ori.deg`.
		if ori_body >= 0.0 {
			ori_body -= pi;
		} else {
			ori_body += pi;
		}

		let diff_ori = ori.deg - ori_body;

		if diff_ori.abs() < 0.1 {
			player_vel.angvel = 0.0;
		} else {
			// TODO: simplify calculation.
			if (diff_ori > 0.0 && diff_ori < pi) || diff_ori < -pi {
				// Clockwise.
				if diff_ori.abs() < 0.5 {
					player_vel.angvel = 5.0;
				} else {
					player_vel.angvel = 20.0;
				}
			} else {
				if diff_ori.abs() < 0.5 {
					player_vel.angvel = -5.0;
				} else {
					player_vel.angvel = -20.0;
				}
			}
		}
	}
}

pub fn push_shield(
	mut joint_set: ResMut<JointSet>,
	players: Query<(&Ori), With<Player>>,
	joints: Query<(&JointHandleComponent)>,
) {
	for (joint_hc) in joints.iter() {
		let ori = players.get(joint_hc.entity1()).unwrap();
		let joint = joint_set.get_mut(joint_hc.handle()).unwrap();
		match &mut joint.params {
			JointParams::PrismaticJoint(prismatic_joint) => {
				if ori.push {
					prismatic_joint.configure_motor_velocity(-300.0, 0.1);
				} else {
					prismatic_joint.configure_motor_velocity(300.0, 0.1);
				}
			}
			_ => panic!(),
		}
	}
}

/// Simulate gravitational forces exerted by `celestial_bodies` on `object_bodies`.
/// TODO: include both `Player` and `Shape` (the performance behaves strangely?) and remove Thrust.
pub fn simulate(
	celestial_bodies: Query<(&Transform, &RigidBodyMassProps), With<CelestialBody>>,
	mut object_bodies: Query<
		(&Thrust, &Transform, &mut RigidBodyForces, &RigidBodyMassProps),
		With<Player>,
	>, /*mut object_bodies: Query<(&Thrust, &Transform, &mut RigidBodyForces, &RigidBodyMassProps), Or<(With<Player>, With<Shape>, With<CelestialBody>)>>*/
) {
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

pub fn compute_dmg(
	mut contact_events: EventReader<ContactEvent>,
	dmg_query: Query<(&Dmg)>,
	mut hp_query: Query<(&mut HP)>
) {
	let mut c = 0;
	for contact_event in contact_events.iter() {
		if let ContactEvent::Started(h1, h2) = contact_event {
			let mut hp1 = hp_query.get_mut(h1.entity()).unwrap();
			let dmg2 = dmg_query.get(h2.entity()).unwrap();
			hp1.val -= dmg2.val;
			let mut hp2 = hp_query.get_mut(h2.entity()).unwrap();
			let dmg1 = dmg_query.get(h1.entity()).unwrap();
			hp2.val -= dmg1.val;
		}
	}
}

pub fn restore_hp(
	mut players: Query<(&Dmg, &mut HP, &ShieldID)>,
	mut shields: Query<(&Dmg, &mut HP), (With<ShieldType>, Without<ShieldID>)>
) {
	// for (player_dmg, mut player_hp, shield_id) in players.iter_mut() {
	// 	let (shield_dmg, mut shield_hp) = shields.get_mut(shield_id.entity).unwrap();
	// 	shield_hp.val += player_dmg.val;
	// 	player_hp.val += shield_dmg.val;
	// }
}

pub fn extract_render_state(
	game_state: Res<GameServer>,
	query: Query<(Entity, &HP, &Player, &Transform, &ShieldID)>,
	shields: Query<(Entity, &HP, &ShieldType, &Transform)>,
	obj_query: Query<(Entity, &HP, &Shape, &Transform)>,
	celestial_query: Query<(Entity, &HP, &CelestialBody, &Transform)>,
) {
	for (entity, _hp, _player, self_pos, _shield_id) in query.iter() {
		// Collect players' positions.
		let positions = query
			.iter()
			.filter_map(|(entity, hp, player, pos, shield_id)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						PlayerView {
							name: player.name.clone(),
							pos: Position { x: pos.translation.x, y: pos.translation.y },
							// TODO: this isn't used in rendering.
							ori: {
								let (axis, angle) = pos.rotation.to_axis_angle();
								axis[2] * angle
							},
							shield_id: shield_id.entity.to_bits(),
							hp: hp.val,
						},
					))
				} else {
					None
				}
			})
			.collect();

		let shield_info = shields
			.iter()
			.filter_map(|(entity, hp, _, pos)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						ShieldView { pos: Position { x: pos.translation.x, y: pos.translation.y }, hp: hp.val },
					))
				} else {
					None
				}
			})
			.collect();

		// Collect positions of static objects.
		let static_pos = obj_query
			.iter()
			.filter_map(|(entity, hp, _, pos)| {
				if (self_pos.translation.x - pos.translation.x).abs() < VIEW_X
					&& (self_pos.translation.y - pos.translation.y).abs() < VIEW_Y
				{
					Some((
						entity.to_bits(),
						StaticView { pos: Position { x: pos.translation.x, y: pos.translation.y }, hp: hp.val },
					))
				} else {
					None
				}
			})
			.collect();

		// Collect celestial positions.
		let celestial_pos = celestial_query
			.iter()
			.map(|(entity, hp, _, pos)| {
				(
					entity.to_bits(),
					CelestialView { pos: Position { x: pos.translation.x, y: pos.translation.y }, hp: hp.val },
				)
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

#[derive(Clone)]
pub struct ChangeMovement {
	pub(crate) player: Entity,
	pub(crate) state: PlayerState,
}

impl Command for ChangeMovement {
	fn write(self: Box<Self>, world: &mut World) {
		let (fy, fx) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
		let mut thrust = world.get_mut::<Thrust>(self.player).expect("No component found.");
		thrust.x = fx * 20000.0;
		thrust.y = fy * 20000.0;
		let mut ori = world.get_mut::<Ori>(self.player).expect("No component found.");
		ori.deg = self.state.ori;
		ori.push = self.state.push_shield;
	}
}
