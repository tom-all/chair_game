//! The client plugin: connects to the server, reports the local player's state each
//! tick, and renders the other players the server replicates back.

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
use lightyear::prelude::*;
use lightyear::prelude::client::*;

use crate::net::protocol::{PlayerId, PlayerInput, PlayerStateChannel, PlayerYaw};
use crate::net::TICK_HZ;

/// Client-side visual for a remote player: a capsule following the replicated state.
const CAPSULE_RADIUS: f32 = 0.4;
const CAPSULE_HALF_HEIGHT: f32 = 0.6;

/// Adds the networking client to the app.
pub struct NetClientPlugin {
    pub addr: SocketAddr,
}

/// The server address the client connects to.
#[derive(Resource)]
struct ServerAddr(SocketAddr);

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerAddr(self.addr))
            .add_plugins(ClientPlugins {
                tick_duration: core::time::Duration::from_secs_f64(1.0 / TICK_HZ),
            })
            .add_plugins(crate::net::protocol::ProtocolPlugin)
            .add_systems(Startup, spawn_client)
            .add_systems(Update, send_player_input)
            .add_systems(
                PostUpdate,
                (spawn_remote_capsules, update_remote_capsules).chain(),
            );
    }
}

/// Spawn the client link entity and connect.
fn spawn_client(mut commands: Commands, addr: Res<ServerAddr>) {
    let client = commands
        .spawn((
            Client,
            RawClient,
            Link::default(),
            UdpIo::default(),
            ReplicationReceiver,
            LocalAddr("0.0.0.0:0".parse().unwrap()),
            PeerAddr(addr.0),
        ))
        .id();
    commands.trigger(Connect { entity: client });
}

/// Report the local player's state to the server, once connected.
fn send_player_input(
    mut links: Query<&mut MessageSender<PlayerInput>, (With<Client>, With<Connected>)>,
    player: Single<(&Transform, &FpsController), With<LogicalPlayer>>,
) {
    let input = PlayerInput {
        position: player.0.translation,
        yaw: player.1.yaw,
    };
    for mut sender in &mut links {
        sender.send::<PlayerStateChannel>(input);
    }
}

/// The capsule entity for a remote player, stored on the player entity.
#[derive(Component)]
struct CapsuleEntity(Entity);

/// Spawn a capsule for each newly-replicated remote player.
fn spawn_remote_capsules(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<(Entity, &PlayerYaw), (Added<PlayerId>, With<Remote>)>,
) {
    for (entity, yaw) in &players {
        let mesh = meshes.add(Cylinder {
            radius: CAPSULE_RADIUS,
            half_height: CAPSULE_HALF_HEIGHT,
        });
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.6, 1.0),
            ..Default::default()
        });
        let capsule = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_rotation(Quat::from_rotation_y(yaw.0)),
            ))
            .id();
        commands.entity(entity).insert(CapsuleEntity(capsule));
    }
}

/// Point each remote player's capsule where the server says it is.
fn update_remote_capsules(
    players: Query<(&Transform, &PlayerYaw, &CapsuleEntity)>,
    // Without<CapsuleEntity> keeps this disjoint from the players query above (the
    // capsules never carry that component).
    mut capsules: Query<&mut Transform, Without<CapsuleEntity>>,
) {
    for (transform, yaw, capsule_entity) in &players {
        if let Ok(mut capsule) = capsules.get_mut(capsule_entity.0) {
            capsule.translation = transform.translation;
            capsule.rotation = Quat::from_rotation_y(yaw.0);
        }
    }
}
