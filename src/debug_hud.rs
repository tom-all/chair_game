use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::MouseMotion,
    prelude::*,
};
use bevy_fps_controller::controller::*;

#[derive(Component)]
pub struct DebugHud;

//debug HUD: shows look angles + raw mouse motion
pub fn spawn_debug_hud(commands: &mut Commands) {
    commands.spawn((
        Text::new("debug"),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugHud,
    ));
}

//shows the look angles and raw mouse counts so we can see what's actually happening
pub fn update_debug_hud(
    mut mouse_events: MessageReader<MouseMotion>,
    input: Single<&FpsControllerInput>,
    diagnostics: Res<DiagnosticsStore>,
    mut text: Single<&mut Text, With<DebugHud>>,
) {
    let (mut dx, mut dy) = (0.0_f32, 0.0_f32);
    for event in mouse_events.read() {
        dx += event.delta.x;
        dy += event.delta.y;
    }
    //fps averaged over the recent frame history (0 until the first frame is measured)
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.average())
        .unwrap_or(0.0);
    text.0 = format!(
        "{:6.0} fps\nyaw: {:7.1}   pitch: {:7.1}\nmouse: {:+9.0} , {:+9.0}\narrow keys: look (RDP test)",
        fps,
        input.yaw.to_degrees(),
        input.pitch.to_degrees(),
        dx,
        dy
    );
}

#[cfg(test)]
mod tests {
    use bevy::{
        diagnostic::FrameTimeDiagnosticsPlugin,
        input::InputPlugin,
        prelude::*,
        time::TimeUpdateStrategy,
    };
    use bevy_fps_controller::controller::FpsControllerInput;
    use std::time::Duration;

    use super::{update_debug_hud, DebugHud};

    //the HUD shows the frame rate from the frame-time diagnostics
    #[test]
    fn hud_shows_fps() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .add_systems(Update, update_debug_hud);

        app.world_mut().spawn(FpsControllerInput::default());
        let hud = app.world_mut().spawn((Text::new("debug"), DebugHud)).id();

        //a few frames so the fps diagnostic has values to average
        for _ in 0..5 {
            app.update();
        }

        let text = app.world().entity(hud).get::<Text>().unwrap().0.clone();
        assert!(
            text.contains("fps"),
            "HUD should contain the fps line: {text}"
        );
        assert!(
            text.contains("60"),
            "HUD should show ~60 fps under the 1/60 s manual clock: {text}"
        );
    }
}
