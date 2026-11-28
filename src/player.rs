use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

/// Moves the gun model from where it sits in its file into the camera's view.
const GUN_VIEW_OFFSET: Vec3 = Vec3::new(-0.16, 0.09, 0.0);
const SPAWN_POINT: Vec3 = Vec3::new(10.0, 1.625, 6.0);   //x,y,z
const VERTICAL_FOV_DEGREES: f32 = 59.0; // roughly 90° horizontal on 16:9

//player: two entities.
//the "logical" player is the invisible physics body the controller drives,
//the "render" player is the camera that follows it.
pub fn spawn_player(commands: &mut Commands, asset_server: &AssetServer) {
    let logical_entity = commands
        .spawn((
            Collider::cylinder(1.5, 0.5),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            ActiveEvents::COLLISION_EVENTS,
            Velocity::zero(),
            RigidBody::Dynamic,
            Sleeping::disabled(),
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            //the controller does its own Source-style gravity
            GravityScale(0.0),
            Ccd { enabled: true }, //prevents clipping through thin geometry at speed
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                ..default()
            },
            FpsController {
                //radians per mouse count — lower = slower look
                //NOTE: 100x reduction because testing over RDP inflates mouse counts
                sensitivity: 0.001,
                air_acceleration: 100.0,    //default 20
                gravity: 23.0,      //default 23
                walk_speed: 14.0,   //default 9
                run_speed: 20.0,    //default 14
                crouched_speed: 9.0,    //default 5
                crouch_speed: 20.0,  //default 6
                uncrouch_speed: 20.0,   //default 8
                acceleration: 10.0,    //default 10
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: VERTICAL_FOV_DEGREES.to_radians(),
            ..default()
        }),
        //ambient fill so the shadowed side of the map isn't black
        AmbientLight {
            color: Color::srgb(0.5, 0.55, 0.65),
            brightness: 30.0,
            affects_lightmapped_meshes: true,
        },
        RenderPlayer { logical_entity },
        //the gun sits at (0.27, -0.12, -0.16) in its file, which is outside
        //bevy's default 45 degree fov, so shift it into the lower right of the view
        children![(
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/weapon1.glb"),
            )),
            Transform::from_translation(GUN_VIEW_OFFSET),
        )],
    ));
}
