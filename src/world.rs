//use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

//blender map :)
pub fn spawn_world(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("maps/lansdowne.glb"),
        )),
        AsyncSceneCollider::default(),
    ));

    //the map carries no lights of its own: a sun so it is visible
    // commands.spawn((
    //     DirectionalLight::default(),
    //     Transform::from_rotation(Quat::from_euler(
    //         EulerRot::YXZ,
    //         -FRAC_PI_4,
    //         -FRAC_PI_2 + 0.5,
    //         0.0,
    //     )),
    // ));
}
