use crate::{TICK_TIME, PlayerView};
use crate::component::{Position, Velocity, Acceleration, Player};
use crate::event::{CreatePlayer, ChangeMovement, RemovePlayer};
use bevy::app::{Events, EventReader};
use bevy::ecs::{Query, Commands, ResMut, Local, Res, Command, World, Resources};
use game_shared::{RenderState, Ori};
use crate::server::GameServer;
use rand::Rng;

pub fn create_player(
    commands: &mut Commands,
    mut events: ResMut<Events<CreatePlayer>>,
    mut game_state: ResMut<GameServer>,
) {
    let mut rng = rand::thread_rng();
    for event in events.drain() {
        commands.spawn((
            Player { name: event.name.clone() },
            Position { x: rng.gen_range(0.0..1000.0), y: rng.gen_range(0.0..1000.0) },
            Ori { deg: 0.0 },
            Velocity { x: 0.0, y: 0.0 },
            Acceleration { x: 0.0, y: 0.0 },
        ));
        let entity = commands.current_entity().unwrap();
        game_state.sessions.insert(entity, event.session.clone());
        event.sender.send(entity);
        println!("Player {} (#{}) joined the game.", event.name, entity.id());
    }
}

impl Command for ChangeMovement {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        let mut acc = world.get_mut::<Acceleration>(self.player).expect("No component found.");
        let (ay, ax) = self.state.dir.map_or((0.0, 0.0), |dir| dir.sin_cos());
        acc.x = ax * 1000.0;
        acc.y = ay * 1000.0;
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
) {
    for event in event_reader.iter(&events) {
        commands.despawn(event.player);
    }
}

pub fn next_frame(mut game_state: ResMut<GameServer>, mut query: Query<(&mut Position, &mut Velocity, &Acceleration)>) {
    game_state.up_time += TICK_TIME;
    let dt = TICK_TIME.as_secs_f32();
    for (mut pos, mut vel, acc) in query.iter_mut() {
        pos.x += dt * vel.x;
        pos.x = pos.x.rem_euclid(1000.0);
        pos.y += dt * vel.y;
        pos.y = pos.y.rem_euclid(1000.0);
        vel.x += dt * (acc.x - 0.3 * vel.x);
        vel.y += dt * (acc.y - 0.3 * vel.y);
    }
}

pub fn extract_render_state(game_state: Res<GameServer>, query: Query<(&Player, &Position, &Ori)>) {
    let positions = query.iter().map(|(player, pos, ori)| {
        (player.name.clone(), pos.clone(), ori.deg)
    }).collect();
    let state = RenderState { time: game_state.up_time, positions };
    game_state.sessions.iter().for_each(|(_, rec)| {
        rec.do_send(PlayerView(state.clone()));
    });
}