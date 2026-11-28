//! The headless server: listens for clients, gives each one a replicated player entity,
//! and (for now) mirrors each client's reported state onto it.

use std::net::SocketAddr;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::*;
use lightyear::prelude::server::*;

use crate::net::protocol::{PlayerId, PlayerInput, PlayerYaw};
use crate::net::{SERVER_LISTEN_ADDR, TICK_HZ};

/// Where a new player entity starts. Matches the standalone spawn point.
const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 6.0);

/// The player entity for a connected client, stored on the client's link entity so the
/// receive system can find it directly.
#[derive(Component)]
struct PlayerEntity(Entity);

/// The address the server listens on.
#[derive(Resource)]
struct ListenAddr(SocketAddr);

/// Build the headless server app, listening on `listen_addr`.
pub fn build_server_app(listen_addr: SocketAddr) -> App {
    let mut app = App::new();
    app.insert_resource(ListenAddr(listen_addr))
        // Default MinimalPlugins runs with no wait, spinning a core at 100%.
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            core::time::Duration::from_secs_f64(1.0 / TICK_HZ),
        )))
        .add_plugins(StatesPlugin)
        .add_plugins(ServerPlugins {
            tick_duration: core::time::Duration::from_secs_f64(1.0 / TICK_HZ),
        })
        .add_plugins(crate::net::protocol::ProtocolPlugin)
        .add_systems(Startup, spawn_server)
        .add_observer(on_client_connected)
        .add_systems(Update, apply_client_input);
    app
}

/// Build and run the headless server app on the default listen address.
pub fn run_server() {
    build_server_app(SERVER_LISTEN_ADDR.parse().unwrap()).run();
}

/// Spawn the server link entity and start listening.
fn spawn_server(mut commands: Commands, listen_addr: Res<ListenAddr>) {
    let server = commands
        .spawn((
            RawServer,
            ServerUdpIo,
            LocalAddr(listen_addr.0),
        ))
        .id();
    commands.trigger(Start { entity: server });
}

/// When a client connects, enable replication on its link and spawn its player entity.
fn on_client_connected(
    trigger: On<Add<Connected>>,
    mut commands: Commands,
    link: Query<&RemoteId, With<ClientOf>>,
) {
    let Ok(remote_id) = link.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0;
    // This link can now send replication data to the client.
    commands.entity(trigger.entity).insert(ReplicationSender);
    // The client's player, replicated to everyone except the client itself (it renders
    // its own local player instead).
    let player = commands
        .spawn((
            Transform::from_translation(SPAWN_POINT),
            PlayerYaw(0.0),
            PlayerId(client_id),
            Replicate::to_clients(NetworkTarget::AllExceptSingle(client_id)),
        ))
        .id();
    commands.entity(trigger.entity).insert(PlayerEntity(player));
}

/// Mirror each client's reported state onto its player entity.
fn apply_client_input(
    mut links: Query<(&PlayerEntity, &mut MessageReceiver<PlayerInput>)>,
    mut players: Query<(&mut Transform, &mut PlayerYaw)>,
) {
    for (player_entity, mut receiver) in &mut links {
        for input in receiver.receive() {
            if let Ok((mut transform, mut yaw)) = players.get_mut(player_entity.0) {
                transform.translation = input.position;
                yaw.0 = input.yaw;
            }
        }
    }
}
