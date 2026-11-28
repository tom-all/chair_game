//! The client plugin: connects to the server, reports the local player's state each
//! tick, and renders the other players the server replicates back.
//!
//! The plugin is dormant until the [`NetModeChoice`] resource says `Client(addr)`: the
//! startup menu sets that at runtime, so the link entity is spawned from an `Update`
//! system rather than at startup.

use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
use lightyear::prelude::*;
use lightyear::prelude::client::*;

use crate::net::protocol::{PlayerId, PlayerInput, PlayerStateChannel, PlayerYaw};
use crate::net::{NetModeChoice, TICK_HZ};

/// Client-side visual for a remote player: a capsule following the replicated state.
const CAPSULE_RADIUS: f32 = 0.4;
const CAPSULE_HALF_HEIGHT: f32 = 0.6;

/// Adds the networking client to the app. Dormant until `NetModeChoice` is `Client`.
pub struct NetClientPlugin;

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: core::time::Duration::from_secs_f64(1.0 / TICK_HZ),
        })
        .add_plugins(crate::net::protocol::ProtocolPlugin)
        .add_systems(Startup, spawn_status_text)
        .add_systems(Update, (spawn_client, update_status_text))
        .add_systems(Update, send_player_input)
        .add_systems(
            PostUpdate,
            (spawn_remote_capsules, update_remote_capsules).chain(),
        );
    }
}

/// Spawn the client link entity and connect, once the menu has chosen `Client`.
fn spawn_client(
    mut commands: Commands,
    choice: Res<NetModeChoice>,
    existing: Query<Entity, With<Client>>,
) {
    let NetModeChoice::Client(addr) = *choice else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let client = commands
        .spawn((
            Client,
            RawClient,
            Link::default(),
            UdpIo::default(),
            ReplicationReceiver,
            LocalAddr("0.0.0.0:0".parse().unwrap()),
            PeerAddr(addr),
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

/// The status text shown top-left, below the debug HUD.
#[derive(Component)]
struct StatusText;

/// Spawn the status text entity (empty until a client link exists).
fn spawn_status_text(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(110.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));
}

/// Show the link's connection state: Connecting / Connected / Disconnected.
fn update_status_text(
    links: Query<(Has<Connected>, Has<Connecting>, Has<Disconnected>), With<Client>>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
    let Some((connected, connecting, disconnected)) = links.iter().next() else {
        text.0 = String::new();
        return;
    };
    text.0 = if connected {
        "Connected".to_string()
    } else if connecting {
        "Connecting...".to_string()
    } else if disconnected {
        "Disconnected".to_string()
    } else {
        String::new()
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::NetModeChoice;

    fn client_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(NetClientPlugin)
            .insert_resource(NetModeChoice::Idle);
        app
    }

    /// Count the link entities (those carrying a `Client` component).
    fn link_count(world: &mut World) -> usize {
        world
            .query::<Entity>()
            .iter(world)
            .filter(|&e| world.get::<Client>(e).is_some())
            .count()
    }

    #[test]
    fn does_not_spawn_client_while_idle() {
        let mut app = client_app();
        app.finish();
        app.update();
        assert_eq!(link_count(app.world_mut()), 0);
    }

    #[test]
    fn spawns_client_when_choice_is_client() {
        let mut app = client_app();
        app.insert_resource(NetModeChoice::Client(
            "127.0.0.1:5000".parse().unwrap(),
        ));
        app.finish();
        app.update();
        assert_eq!(link_count(app.world_mut()), 1);
    }

    #[test]
    fn does_not_spawn_client_when_standalone() {
        let mut app = client_app();
        app.insert_resource(NetModeChoice::Standalone);
        app.finish();
        app.update();
        assert_eq!(link_count(app.world_mut()), 0);
    }

    #[test]
    fn does_not_double_spawn() {
        let mut app = client_app();
        app.insert_resource(NetModeChoice::Client(
            "127.0.0.1:5000".parse().unwrap(),
        ));
        app.finish();
        app.update();
        app.update();
        assert_eq!(link_count(app.world_mut()), 1);
    }
}
