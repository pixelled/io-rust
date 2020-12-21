use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use game_shared::{GameState, Movement};
use std::sync::Mutex;
use std::collections::VecDeque;
use std::ops::DerefMut;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
const TICK_RATE: Duration = Duration::from_millis(16);

struct WsSession {
    hb: Instant,
    state: web::Data<Mutex<AppState>>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        self.state.lock().unwrap().game.add_player();
        ctx.run_interval(TICK_RATE, |act, ctx| {
            let mut state =  act.state.lock().unwrap();
            let AppState { game, commands } = state.deref_mut();
            game.update(commands);
            ctx.binary(game.serialize());
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => {
                self.state.lock().unwrap().commands.push_back(Movement::deserialize(bin.as_ref()));
            }
            Ok(ws::Message::Close(reason)) => {
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
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

struct AppState {
    commands: VecDeque<Movement>,
    game: GameState,
}

async fn index(req: HttpRequest, stream: web::Payload, data: web::Data<Mutex<AppState>>) -> Result<HttpResponse, Error> {
    let res = ws::start(WsSession {
        hb: Instant::now(),
        state: data,
    }, &req, stream);
    println!("{:?}", res);
    res
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(Mutex::new(AppState {
        commands: VecDeque::new(),
        game: GameState::new()
    }));
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(web::resource("/ws").route(web::get().to(index)))
            .service(fs::Files::new("/", "dist/").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
