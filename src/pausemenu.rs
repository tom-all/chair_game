use bevy::{
    color::palettes::tailwind::SLATE_300,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        AutoFocus, InputFocus,
    },
    prelude::*,
    text::{EditableText, EditableTextFilter, TextCursorStyle},
    ui_widgets::TextInput,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::FpsController;

const BASE_SENSITIVITY: f32 = 0.001; // check the crate's default before relying on this
const MIN_MULTIPLIER: f32 = 0.1;
const MAX_MULTIPLIER: f32 = 10.0;

#[derive(States, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
    /// Startup menu: play solo or join a friend's server.
    Menu,
}

#[derive(Resource)]
pub struct MouseSettings {
    pub multiplier: f32, // 1.0 = the crate's default feel
}

impl Default for MouseSettings {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct SensitivityInput;

#[derive(Component)]
struct CurrentValueLabel;

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        // GameState is registered in main.rs (insert_state) so the initial state can be
        // Menu or Playing; the menu plugin is added only when the menu is shown.
        app.add_plugins(TabNavigationPlugin)
            .init_resource::<MouseSettings>()
            .add_systems(
                Update,
                (
                    toggle_pause,
                    apply_sensitivity,
                    submit_sensitivity,
                    update_value_label,
                ),
            )
            .add_systems(OnEnter(GameState::Paused), (enter_pause, spawn_menu))
            .add_systems(OnExit(GameState::Paused), (exit_pause, despawn_menu));
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Escape does nothing in the menu: the menu owns the cursor and the buttons.
    if *state.get() == GameState::Menu {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(match state.get() {
            GameState::Playing => GameState::Paused,
            GameState::Paused => GameState::Playing,
            GameState::Menu => GameState::Menu,
        });
    }
}

fn enter_pause(
    mut cursor: Single<&mut CursorOptions>,
    mut controllers: Query<&mut FpsController>,
) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
    for mut controller in &mut controllers {
        controller.enable_input = false;
    }
}

fn exit_pause(
    mut cursor: Single<&mut CursorOptions>,
    mut controllers: Query<&mut FpsController>,
) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    for mut controller in &mut controllers {
        controller.enable_input = true;
    }
}

fn spawn_menu(mut commands: Commands, settings: Res<MouseSettings>) {
    // The text field
    let input = commands
        .spawn((
            Node {
                border: px(2).all(),
                ..default()
            },
            BorderColor::from(Color::from(SLATE_300)),
            SensitivityInput,
            TextInput,
            EditableText {
                visible_width: Some(10.0),
                allow_newlines: false,
                max_characters: Some(6),
                ..default()
            },
            EditableTextFilter::new(|c| c.is_ascii_digit() || c == '.'),
            TextLayout::no_wrap(),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextCursorStyle::default(),
            TabIndex(0),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        ))
        .id();

    // Container that gives the field focus as soon as the menu opens
    let input_container = commands
        .spawn((Node::default(), AutoFocus, TabGroup::new(0)))
        .id();
    commands.entity(input_container).add_children(&[input]);

    // Text lines
    let title = commands
        .spawn((
            Text::new("Paused"),
            TextFont {
                font_size: FontSize::Px(40.0),
                ..default()
            },
        ))
        .id();

    let hint = commands
        .spawn((
            Text::new("Mouse sensitivity (0.1 - 10). Type a number and press Enter."),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
        ))
        .id();

    let value_label = commands
        .spawn((
            Text::new(format!("Current: {:.2}", settings.multiplier)),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            CurrentValueLabel,
        ))
        .id();

    // Column holding everything
    let panel = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(16),
            padding: px(24).all(),
            ..default()
        })
        .id();
    commands
        .entity(panel)
        .add_children(&[title, hint, input_container, value_label]);

    // Full-screen dimmed backdrop that centers the panel
    let root = commands
        .spawn((
            PauseMenuRoot,
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

fn despawn_menu(mut commands: Commands, roots: Query<Entity, With<PauseMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn apply_sensitivity(
    settings: Res<MouseSettings>,
    mut controllers: Query<&mut FpsController>,
) {
    for mut controller in &mut controllers {
        controller.sensitivity = BASE_SENSITIVITY * settings.multiplier;
    }
}

fn submit_sensitivity(
    input_focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    mut inputs: Query<&mut EditableText, With<SensitivityInput>>,
    mut settings: ResMut<MouseSettings>,
) {
    if keys.just_pressed(KeyCode::Enter)
        && let Some(entity) = input_focus.get()
        && let Ok(mut text) = inputs.get_mut(entity)
    {
        if let Ok(value) = text.value().to_string().trim().parse::<f32>() {
            settings.multiplier = value.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
        }
        text.clear();
    }
}

fn update_value_label(
    settings: Res<MouseSettings>,
    mut labels: Query<&mut Text, With<CurrentValueLabel>>,
) {
    if settings.is_changed() {
        for mut text in &mut labels {
            text.0 = format!("Current: {:.2}", settings.multiplier);
        }
    }
}