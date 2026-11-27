mod camera;
mod debug_hud;
mod net;
mod player;
mod weapon;
mod world;
mod crosshair;
mod pausemenu;

use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

fn main() {
    let mode = net::NetMode::from_args(std::env::args().skip(1));
    //The server is a separate headless app: no window, no map, no player.
    if let net::NetMode::Server = mode {
        net::server::run_server();
        return;
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(RapierPhysicsPlugin::<()>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            //fire_bullet first: it reads the lock state of this frame, so the click
            //that locks the cursor is consumed and only fires once already locked
            (weapon::fire_bullet, camera::manage_cursor, debug_hud::update_debug_hud),
        )
        .add_systems(Update, (weapon::move_bullets, weapon::spawn_impact_markers))
        .add_plugins(crosshair::CrosshairPlugin)
        //character still moves and can shoot while paused, not a big deal rn but the whole click to lock cursor thing will have to go when we want to fix it
        .add_plugins(pausemenu::PauseMenuPlugin);
    //Client mode adds the networking client on top of the usual game.
    if let net::NetMode::Client(addr) = mode {
        app.add_plugins(net::client::NetClientPlugin { addr });
    }
    app
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    player::spawn_player(&mut commands, &asset_server);
    world::spawn_world(&mut commands, asset_server);
    debug_hud::spawn_debug_hud(&mut commands);
}
