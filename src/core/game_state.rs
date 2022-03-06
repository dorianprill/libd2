use crate::ServerMessage;
use crate::core::areas::ActId;
use crate::core::entity::npc::Npc;
use crate::core::entity::player::Player;
use crate::core::network::d2gs::D2GSPacket;
use crate::core::object::item::Item;
use crate::core::update::Update;

pub enum GameServerType {
    OpenBattleNet = 1,
    TCPIP = 2,
    SinglePlayer = 3,
}

#[allow(dead_code)]
pub enum Difficulty {
    Normal = 0,
    Nightmare = 1,
    Hell = 2,
}

#[allow(non_camel_case_types)]
pub enum Locale {
    enUS = 0,
    esES = 1,
    deDE = 2,
    frFR = 3,
    ptPT = 4,
    itIT = 5,
    ja = 6,
    ko = 7,
    si = 8,
    zhCN = 9,
    pl = 10,
    ru = 11,
    enGB = 12,
}

#[allow(dead_code)]
pub struct GameState {
    // skills: Vec<Skills>;
    // item_skills: Vec<ItemSkills>
    //myself: &'a Player, // ref to players[0] ?
    pub(crate) players: Vec<Player>,
    pub(crate) npcs: Vec<Npc>,
    pub(crate) game_type: Option<GameServerType>,
    pub(crate) difficulty: Option<Difficulty>,
    pub(crate) locale: Locale, //objects: Vec<WorldObject>, // TODO world object are ground items, stashes, corpses etc
}

impl Update for GameState {
    fn update(&self, packet: ServerMessage) -> bool {
        // match on packet ID here (first byte of message)
        // then pass on to respective handlers
        return true;
    }
}
