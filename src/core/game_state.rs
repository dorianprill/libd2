use crate::ServerMessage;
use crate::core::act::Act;
use crate::core::entity::npc::Npc;
use crate::core::entity::player::Player;
use crate::core::object::item::Item;
use crate::core::update::Update;


pub enum GameServerType {
    OpenBattleNet   = 1,
    TCPIP           = 2,
    SinglePlayer    = 3,
}

/// GameMode Flags as sent by GameList server message
#[repr(u32)]
pub enum GameMode {
    Ladder    = 0x00200000,
    Expansion = 0x00100000,
    Hardcore  = 0x00000800
}

/// Difficulty flags as sent by GameList server message 
#[repr(u16)]
pub enum Difficulty {
    Normal    = 0x0000,
    Nightmare = 0x1000,
    Hell      = 0x2000
}


#[allow(non_camel_case_types)]
pub enum Locale {
    enUS = 0,
    esES = 1,
    deDE = 2,
    frFR = 3,
    ptPT = 4,
    itIT = 5,
    ja   = 6,
    ko   = 7,
    si   = 8,
    zhCN = 9,
    pl   = 10,
    ru   = 11,
    enGB = 12,
}

#[allow(dead_code)]
pub struct GameState {
    // skills: Vec<Skills>;
    // item_skills: Vec<ItemSkills>
    //myself: &'a Player, // ref to players[0] ?
    pub(crate) players: Vec<Player>,
    pub(crate) npcs: Vec<Npc>,
    pub(crate) game_type: GameServerType,
    pub(crate) difficulty: Difficulty,
    pub(crate) locale: Locale, //objects: Vec<WorldObject>, // TODO world object are ground items, stashes, corpses etc
}

impl Update for GameState {
    fn update(&self, packet: ServerMessage) -> bool {
        // match on packet ID here (first byte of message)
        // then pass on to respective handlers
        return true;
    }
}
