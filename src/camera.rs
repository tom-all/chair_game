use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::*;

use crate::pausemenu::GameState;

//click to capture the mouse (and enable the controller), esc to release;
//the capturing click is consumed, so a shot only fires once the cursor is locked
pub fn manage_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut cursor: Single<&mut CursorOptions>,
    mut controllers: Query<&mut FpsController>,
) {
    // The menu owns the cursor: a click there must not lock it or enable the controllers.
    if *state.get() == GameState::Menu {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode != CursorGrabMode::Locked {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        for mut controller in &mut controllers {
            controller.enable_input = true;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
        for mut controller in &mut controllers {
            controller.enable_input = false;
        }
    }
}
