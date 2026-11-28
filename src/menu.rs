//! Startup menu: play solo or join a friend's server.
//!
//! Shown at startup when the game is launched without a network flag. The menu sets the
//! `NetModeChoice` resource and transitions to `Playing`; the client plugin spawns the
//! link entity from that resource, so joining needs no plugin changes.

use std::net::SocketAddr;

use bevy::{
    color::palettes::tailwind::SLATE_300,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        AutoFocus, InputFocus,
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{observe, Activate, Button, TextInput},
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::FpsController;

use crate::net::{NetModeChoice, DEFAULT_SERVER_ADDR};
use crate::pausemenu::GameState;

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct AddressInput;

#[derive(Component)]
struct ErrorLine;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), (enter_menu, spawn_menu))
            .add_systems(OnExit(GameState::Menu), (exit_menu, despawn_menu))
            // The player is spawned in Startup, which runs after the OnEnter(Menu)
            // schedule, so the controllers are disabled here rather than in enter_menu.
            .add_systems(Startup, disable_controllers.run_if(in_state(GameState::Menu)))
            .add_systems(Update, submit_join.run_if(in_state(GameState::Menu)));
    }
}

fn enter_menu(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

/// Disable the controllers now that the player exists (the menu is open).
fn disable_controllers(mut controllers: Query<&mut FpsController>) {
    for mut controller in &mut controllers {
        controller.enable_input = false;
    }
}

fn exit_menu(
    mut cursor: Single<&mut CursorOptions>,
    mut controllers: Query<&mut FpsController>,
) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    for mut controller in &mut controllers {
        controller.enable_input = true;
    }
}

fn spawn_menu(mut commands: Commands) {
    // Address field, pre-filled with the default server address.
    let mut editable = EditableText::new(DEFAULT_SERVER_ADDR);
    editable.allow_newlines = false;
    editable.max_characters = Some(64);
    editable.visible_width = Some(24.0);
    let input = commands
        .spawn((
            Node {
                border: px(2).all(),
                ..default()
            },
            BorderColor::from(Color::from(SLATE_300)),
            AddressInput,
            TextInput,
            editable,
            TextLayout::no_wrap(),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextCursorStyle::default(),
            TabIndex(0),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        ))
        .id();

    let input_container = commands
        .spawn((Node::default(), AutoFocus, TabGroup::new(0)))
        .id();
    commands.entity(input_container).add_children(&[input]);

    let title = commands
        .spawn((
            Text::new("Chair Game"),
            TextFont {
                font_size: FontSize::Px(48.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.95)),
        ))
        .id();

    let subtitle = commands
        .spawn((
            Text::new("Play solo or join a friend's server"),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ))
        .id();

    let play_solo = commands
        .spawn((
            Button,
            Node {
                width: px(240),
                height: px(60),
                border: px(2).all(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            children![(
                Text::new("Play solo"),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            )],
            observe(
                |_activate: On<Activate>,
                 mut choice: ResMut<NetModeChoice>,
                 mut next_state: ResMut<NextState<GameState>>| {
                    *choice = NetModeChoice::Standalone;
                    next_state.set(GameState::Playing);
                },
            ),
        ))
        .id();

    let join = commands
        .spawn((
            Button,
            Node {
                width: px(240),
                height: px(60),
                border: px(2).all(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            children![(
                Text::new("Join"),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            )],
            observe(
                |_activate: On<Activate>,
                 mut choice: ResMut<NetModeChoice>,
                 mut next_state: ResMut<NextState<GameState>>,
                 inputs: Query<&EditableText, With<AddressInput>>,
                 mut error_lines: Query<&mut Text, With<ErrorLine>>| {
                    let Some(input) = inputs.iter().next() else {
                        return;
                    };
                    let text = input.value().to_string();
                    match text.trim().parse::<SocketAddr>() {
                        Ok(addr) => {
                            *choice = NetModeChoice::Client(addr);
                            next_state.set(GameState::Playing);
                        }
                        Err(_) => {
                            for mut line in &mut error_lines {
                                line.0 = "Invalid address — expected host:port".to_string();
                            }
                        }
                    }
                },
            ),
        ))
        .id();

    let error_line = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.4, 0.4)),
            ErrorLine,
        ))
        .id();

    let panel = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(16),
            padding: px(24).all(),
            ..default()
        })
        .id();
    commands.entity(panel).add_children(&[
        title,
        subtitle,
        play_solo,
        join,
        input_container,
        error_line,
    ]);

    let root = commands
        .spawn((
            MenuRoot,
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .id();
    commands.entity(root).add_children(&[panel]);
}

fn despawn_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

/// Enter in the address field joins the server (same logic as the Join button).
fn submit_join(
    input_focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    mut choice: ResMut<NetModeChoice>,
    mut next_state: ResMut<NextState<GameState>>,
    inputs: Query<&EditableText, With<AddressInput>>,
    mut error_lines: Query<&mut Text, With<ErrorLine>>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(entity) = input_focus.get() else {
        return;
    };
    let Ok(input) = inputs.get(entity) else {
        return;
    };
    let text = input.value().to_string();
    match text.trim().parse::<SocketAddr>() {
        Ok(addr) => {
            *choice = NetModeChoice::Client(addr);
            next_state.set(GameState::Playing);
        }
        Err(_) => {
            for mut line in &mut error_lines {
                line.0 = "Invalid address — expected host:port".to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_port() {
        assert!("127.0.0.1:5000".parse::<SocketAddr>().is_ok());
        assert!("10.0.0.5:6000".parse::<SocketAddr>().is_ok());
    }

    #[test]
    fn rejects_garbage() {
        assert!("not-an-address".parse::<SocketAddr>().is_err());
        assert!("".parse::<SocketAddr>().is_err());
        assert!("127.0.0.1".parse::<SocketAddr>().is_err());
    }
}
