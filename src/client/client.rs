use crate::core::game_state::{Difficulty, GameServerType, GameState, Locale};
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
    pub fn new(&self) -> Self {
        Client {
            game_state: GameState {
                players: Vec::with_capacity(8),
                npcs: Vec::with_capacity(1024),
                game_type: None,
                difficulty: None,
                locale: Locale::enUS,
            },
            connection: Connection::new(),
        }
    }

    pub fn start(&mut self) {
        self.connection.init();
        self.connection.listen();
    }
}
