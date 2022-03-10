use crate::core::game_state::{Difficulty, GameServerType, GameState, Locale, GameMode};
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
        self.connection.listen();
    }
}

impl Default for Client {
    fn default() -> Self {
        Client {
            game_state: GameState {
                players:    Vec::with_capacity(8),
                npcs:       Vec::with_capacity(1024),
                game_type:  GameServerType::SinglePlayer,
                difficulty: Difficulty::Normal,
                locale:     Locale::enUS,
            },
            connection: Connection::new(),
        }
    }
}
