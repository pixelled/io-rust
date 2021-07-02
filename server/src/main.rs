use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use crate::event::{ChangeMovement, CreatePlayer, EventListener, RemovePlayer};
use crate::server::{GameProxy, GameServer};
use bevy::MinimalPlugins;
use bevy_rapier2d::physics::RapierPhysicsPlugin;
use game_shared::{Operation, ViewSnapshot};
use bevy::ecs::system::IntoSystem;
use bevy::ecs::entity::Entity;
use bevy::ecs::schedule::SystemSet;
use bevy::core::FixedTimestep;
use bevy_rapier2d::prelude::NoUserData;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
pub const TICK_TIME: Duration = Duration::from_millis(16);

mod component;
mod event;
mod server;
mod system;

pub struct WsSession {
	hb: Instant,
	player_entity: Option<Entity>,
	proxy: GameProxy,
}

impl Actor for WsSession {
	type Context = ws::WebsocketContext<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.hb(ctx);
	}
}

struct View(ViewSnapshot);

impl Message for View {
	type Result = ();
}

impl Handler<View> for WsSession {
	type Result = ();

	fn handle(&mut self, msg: View, ctx: &mut Self::Context) -> Self::Result {
		ctx.binary(msg.0.serialize());
	}
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Pong(_)) => {
				self.hb = Instant::now();
			}
			Ok(ws::Message::Binary(bin)) => {
				match Operation::deserialize(bin.as_ref()) {
					Operation::Join(name) => {
						let (sender, receiver) = futures::channel::oneshot::channel();
						self.proxy.create_player(name, sender, ctx.address());
						receiver
							.into_actor(self)
							.then(|e, act, _ctx| {
								act.player_entity = Some(e.unwrap());
								fut::ready(())
							})
							.wait(ctx);
					}
					Operation::Update(player_state) => {
						self.proxy.change_movement(self.player_entity, player_state)
					}
					// Unused
					Operation::Leave => self.proxy.remove_player(self.player_entity),
				}
			}
			Ok(ws::Message::Close(reason)) => {
				self.proxy.remove_player(self.player_entity);
				ctx.close(reason);
				ctx.stop();
			}
			_ => (),
		}
	}
}

impl WsSession {
	fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
		ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
			if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
				println!("Heartbeat failed!");
				act.proxy.remove_player(act.player_entity);
				ctx.stop();
				return;
			}
			ctx.ping(b"");
		});
	}
}

async fn index(
	req: HttpRequest,
	stream: web::Payload,
	data: web::Data<GameProxy>,
) -> Result<HttpResponse, Error> {
	let res = ws::start(
		WsSession { hb: Instant::now(), player_entity: None, proxy: data.as_ref().clone() },
		&req,
		stream,
	);
	println!("{:?}", res);
	res
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
	Playing,
	GameOver,
}

#[actix_web::main]
async fn main() {
	let (s1, r1) = futures::channel::mpsc::unbounded();
	let (s2, r2) = futures::channel::mpsc::unbounded();
	let (s3, r3) = futures::channel::mpsc::unbounded();

	futures::future::join(
		async {
			bevy::prelude::App::build()
				.add_plugins(MinimalPlugins)
				.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
				.add_event::<CreatePlayer>()
				.add_event::<RemovePlayer>()
				.add_event::<ChangeMovement>()
				.add_state(GameState::Playing)
				.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(system::setup.system()))
				.insert_resource(GameServer::new())
				.insert_resource(EventListener(r1))
				.insert_resource(EventListener(r2))
				.insert_resource(EventListener(r3))
				.add_system_set(SystemSet::on_update(GameState::Playing)
					.with_run_criteria(FixedTimestep::step(TICK_TIME.as_secs_f64()))
					.with_system(event::trigger_events.system())
					.with_system(system::create_player.system())
					.with_system(system::remove_player.system())
					.with_system(system::change_movement.system())
					.with_system(system::simulate.system())
					.with_system(system::extract_render_state.system())
				)
				//.add_system(system::collisions.system())
				.run();
		},
		HttpServer::new(move || {
			App::new()
				.data(GameProxy::new(s1.clone(), s2.clone(), s3.clone()))
				.service(web::resource("/ws").route(web::get().to(index)))
				.service(fs::Files::new("/", "dist/").index_file("index.html"))
		})
		.bind("127.0.0.1:8080")
		.unwrap()
		.run(),
	)
	.await;
}
