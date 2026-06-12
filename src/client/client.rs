use crate::core::game_state::GameState;
use crate::core::network::connection::{Connection, ConnectionEvent};

pub enum Status {
    Uninitialized,
    InvalidCdKey,
    InvalidExpCdKey,
    KeyInUse,
    ExpKeyInUse,
    BannedCdKey,
    BannedExpCdKey,
    LoginError,
    McpLogonFail,
    RealmDown,
    OnMcp,
    NotInGame,
}

#[derive(Default)]
pub struct Client {
    game_state: GameState,
    connection: Connection,
}

impl Client {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn start(&mut self) {
        self.start_with_events(|_, _| {});
    }

    /// Starts the blocking packet listener and emits capture events.
    ///
    /// `d2helper`-style UI tools should run this method on a worker thread and
    /// forward either [`ConnectionEvent`] values or compact snapshots derived
    /// from `game_state` to the UI thread. The callback is invoked after a
    /// parsed message has been applied to the internal [`GameState`].
    pub fn start_with_events<F>(&mut self, on_event: F)
    where
        F: FnMut(ConnectionEvent, &GameState),
    {
        self.connection.init();
        self.connection
            .listen_with_events(&mut self.game_state, on_event);
    }

    /// Starts the blocking packet listener and gives the callback mutable access
    /// to the internal [`GameState`] after each event.
    pub fn start_with_mut_events<F>(&mut self, on_event: F)
    where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        self.connection.init();
        self.connection
            .listen_with_mut_events(&mut self.game_state, on_event);
    }

    /// Processes a legacy D2GS payload without using live packet capture.
    ///
    /// This is useful for fixture replay and for tools that provide their own
    /// packet source but still want `Client` to own the [`GameState`].
    pub fn process_d2gs_payload(&mut self, payload: &[u8]) -> Vec<ConnectionEvent> {
        self.connection
            .process_d2gs_payload(payload, &mut self.game_state)
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn game_state_mut(&mut self) -> &mut GameState {
        &mut self.game_state
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::ConnectionEvent;

    #[test]
    fn client_can_process_fixture_payload_into_state() {
        let mut client = Client::new();
        let mut packet = vec![0x59, 0x04, 0x03, 0x02, 0x01, 0x03];
        packet.extend_from_slice(b"Rusty\0\0\0\0\0\0\0\0\0\0\0");
        packet.extend_from_slice(&1234u16.to_le_bytes());
        packet.extend_from_slice(&5678u16.to_le_bytes());

        let events = client.process_d2gs_payload(&packet);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ConnectionEvent::ServerMessage { .. }));
        assert!(client.game_state().player(0x0102_0304).is_some());
        assert_eq!(client.game_state().local_player_id(), None);
    }
}
