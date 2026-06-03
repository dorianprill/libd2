use crate::core::game_state::GameState;
use crate::core::network::connection::Connection;

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

pub struct Client {
    game_state: GameState,
    connection: Connection,
}

impl Client {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn start(&mut self) {
        self.connection.init();
        self.connection.listen(&mut self.game_state);
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }
}

impl Default for Client {
    fn default() -> Self {
        Client {
            game_state: GameState::default(),
            connection: Connection::new(),
        }
    }
}
