//! The shared network protocol: the types that cross the wire and the plugin that
//! registers them. Must be added after the Client/Server plugins and before any link
//! entity is spawned.

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// Identity of a player. The server gives each connected client a `PlayerId` equal to
/// its `PeerId`, so a client can recognise its own replicated player.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
pub struct PlayerId(pub PeerId);

/// A player's facing. The body's rotation is physics-locked, so yaw is replicated
/// separately from the position.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct PlayerYaw(pub f32);

/// The client's per-tick state, sent to the server. Stopgap "mirror" input: the client
/// is authoritative for its own body for now; the server just echoes it back.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    pub position: Vec3,
    pub yaw: f32,
}

/// Channel for [`PlayerInput`]: only the newest state matters, so older packets are
/// dropped in favour of the latest.
pub struct PlayerStateChannel;

/// Registers the protocol types for replication and messaging.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Replicated components: the player's identity, facing and position.
        app.component::<PlayerId>().replicate();
        app.component::<PlayerYaw>().replicate();
        app.component::<Transform>().replicate();

        // Client -> server state message.
        app.register_message::<PlayerInput>()
            .add_direction(NetworkDirection::ClientToServer);

        // The channel it travels on. The direction is what gives the client a sender for it
        // (a channel with no direction never gets one on the client), so it must match the
        // message's.
        app.add_channel::<PlayerStateChannel>(ChannelSettings {
            mode: ChannelMode::SequencedUnreliable,
            ..Default::default()
        })
        .add_direction(NetworkDirection::ClientToServer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_input_serde_round_trip() {
        let input = PlayerInput {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: -0.5,
        };
        let json = serde_json::to_string(&input).unwrap();
        let decoded: PlayerInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, input);
    }
}
