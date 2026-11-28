//! Networking: the game can run standalone (default), as a headless server (`--server`),
//! or as a client (`--client [addr]`). The protocol is shared between the two.

pub mod client;
pub mod protocol;
pub mod server;

use std::net::SocketAddr;

use bevy::prelude::Resource;

/// How the game runs: standalone (default), a headless server, or a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetMode {
    Standalone,
    Server,
    Client(SocketAddr),
}

/// What the player chose at the startup menu (or passed on the command line).
///
/// The menu sets this at runtime and the client plugin reads it each frame, so a client
/// can be spawned after the app has started. `Idle` means the menu is still open.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NetModeChoice {
    /// The menu is still open; nothing has been chosen yet.
    #[default]
    Idle,
    /// Play solo.
    Standalone,
    /// Join a friend's server.
    Client(SocketAddr),
}

/// Default server address for `--client` with no explicit address.
pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:5000";

/// Address the server binds to.
pub const SERVER_LISTEN_ADDR: &str = "0.0.0.0:5000";

/// Network tick rate: matches the game's 60 Hz frame rate.
pub const TICK_HZ: f64 = 60.0;

impl NetMode {
    /// Parse the network mode from the command-line args.
    ///
    /// Scans all args (not just positional ones), so it coexists with the betty script
    /// path at arg 1: a scripted client run passes the flag *after* the script.
    pub fn from_args<I>(args: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut is_client = false;
        let mut client_addr: Option<SocketAddr> = None;
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--server" => return NetMode::Server,
                "--client" => {
                    is_client = true;
                    // An address may follow; only consume it if it parses as one, so a
                    // stray non-address arg doesn't get swallowed.
                    if let Some(next) = args.peek() {
                        if let Ok(addr) = next.parse::<SocketAddr>() {
                            client_addr = Some(addr);
                            args.next();
                        }
                    }
                }
                _ => {}
            }
        }
        if is_client {
            let addr = client_addr
                .unwrap_or_else(|| DEFAULT_SERVER_ADDR.parse().unwrap());
            NetMode::Client(addr)
        } else {
            NetMode::Standalone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flags_is_standalone() {
        assert_eq!(NetMode::from_args(args(&[])), NetMode::Standalone);
        // A betty script path at arg 1 must not be mistaken for a flag.
        assert_eq!(
            NetMode::from_args(args(&["betty/walk.txt"])),
            NetMode::Standalone
        );
    }

    #[test]
    fn server_flag() {
        assert_eq!(NetMode::from_args(args(&["--server"])), NetMode::Server);
    }

    #[test]
    fn client_flag_uses_default_addr() {
        assert_eq!(
            NetMode::from_args(args(&["--client"])),
            NetMode::Client("127.0.0.1:5000".parse().unwrap())
        );
    }

    #[test]
    fn client_flag_with_addr() {
        assert_eq!(
            NetMode::from_args(args(&["--client", "10.0.0.5:6000"])),
            NetMode::Client("10.0.0.5:6000".parse().unwrap())
        );
    }

    #[test]
    fn client_flag_after_script_path() {
        // Scripted client run: the script path is arg 1, the flag follows.
        assert_eq!(
            NetMode::from_args(args(&["betty/net_client.txt", "--client"])),
            NetMode::Client("127.0.0.1:5000".parse().unwrap())
        );
    }

    #[test]
    fn client_flag_does_not_swallow_non_address() {
        // `--client foo` — "foo" is not an address, so the default is used and "foo"
        // is left alone.
        assert_eq!(
            NetMode::from_args(args(&["--client", "foo"])),
            NetMode::Client("127.0.0.1:5000".parse().unwrap())
        );
    }
}

/// Headless integration test: a server and two clients over localhost UDP, advanced by
/// hand. Proves the full loop — input → server → replication → remote player — without a
/// window or GPU.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy::{
        asset::AssetPlugin,
        prelude::*,
        state::app::StatesPlugin,
        time::TimeUpdateStrategy,
    };
    use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
    use std::time::Duration;

    const TICK: Duration = Duration::from_nanos(1_000_000_000 / 60);

    /// Build a minimal client app connected to `addr`, with a local player at `position`
    /// to report to the server.
    fn build_client_app(addr: std::net::SocketAddr, position: Vec3) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(client::NetClientPlugin)
            .insert_resource(NetModeChoice::Client(addr))
            .insert_resource(TimeUpdateStrategy::ManualDuration(TICK));
        app.world_mut().spawn((
            LogicalPlayer,
            FpsController::default(),
            Transform::from_translation(position),
        ));
        app
    }

    fn player_count(world: &mut bevy::prelude::World) -> usize {
        world
            .query::<Entity>()
            .iter(world)
            .filter(|&e| world.get::<protocol::PlayerId>(e).is_some())
            .count()
    }

    /// The positions of the player entities (those carrying a `PlayerId`) in `world`.
    fn player_positions(world: &mut bevy::prelude::World) -> Vec<Vec3> {
        world
            .query::<Entity>()
            .iter(world)
            .filter_map(|e| {
                world
                    .get::<protocol::PlayerId>(e)
                    .and_then(|_| world.get::<Transform>(e))
                    .map(|t| t.translation)
            })
            .collect()
    }

    /// Pick a free localhost UDP port by binding to port 0 and reading back the
    /// OS-assigned one. Avoids clashing with a port another run (or process) holds.
    fn free_port() -> u16 {
        use std::net::UdpSocket;
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        port
    }

    #[test]
    fn two_clients_see_each_other() {
        let listen: std::net::SocketAddr =
            format!("127.0.0.1:{}", free_port()).parse().unwrap();
        let mut server = server::build_server_app(listen);
        server.insert_resource(TimeUpdateStrategy::ManualDuration(TICK));
        // Give each client a distinct local position so we can verify the server mirrored
        // its reported input (the client -> server path), not just that the player was
        // replicated. The spawn point is (0, 1.625, 6); anything else means input arrived.
        let pos_a = Vec3::new(1.0, 1.625, 6.0);
        let pos_b = Vec3::new(2.0, 1.625, 6.0);
        let mut client_a = build_client_app(listen, pos_a);
        let mut client_b = build_client_app(listen, pos_b);

        // Finalize the apps so plugin `finish` hooks (which build the replication channel
        // map) run before the first update.
        server.finish();
        client_a.finish();
        client_b.finish();

        // Run all three apps for enough frames to connect and replicate.
        for _ in 0..180 {
            server.update();
            client_a.update();
            client_b.update();
        }

        // The server should have one player entity per connected client.
        assert_eq!(
            player_count(server.world_mut()),
            2,
            "server should have one player per client"
        );
        // Each client should receive the other client's replicated player (not its own).
        assert_eq!(
            player_count(client_a.world_mut()),
            1,
            "client A should see client B's player"
        );
        assert_eq!(
            player_count(client_b.world_mut()),
            1,
            "client B should see client A's player"
        );
        // The server should have mirrored each client's reported position onto its player.
        // This is the input path (client -> server -> player); without it the players stay
        // at the spawn point and a missing channel would go undetected.
        let positions = player_positions(server.world_mut());
        assert!(
            positions.iter().any(|p| (p - pos_a).length() < 0.1),
            "server should have mirrored client A's position, got {positions:?}"
        );
        assert!(
            positions.iter().any(|p| (p - pos_b).length() < 0.1),
            "server should have mirrored client B's position, got {positions:?}"
        );
    }
}
