// Player struct

use crate::ServerMessage;
use crate::core::coordinate::Coordinate;
use crate::core::update::Update;
use crate::core::{entity::Entity, network::d2gs::D2GSPacket};

#[allow(dead_code)]
pub struct Player {
    class: CharacterClass,
    name: String,
    id: u32,
    location: Coordinate,
    has_mercenary: bool,
    directory_known: bool,
    mercenary_id: u32,
    level: u32,
    portal_id: u32,
    // TODO
    // stash:       Container;
    // cube:        Container;
    // belt:        Belt;
}

impl Player {
    pub fn name(self) -> String {
        self.name
    }

    pub fn has_mercenary(&self) -> bool {
        self.has_mercenary
    }

    pub fn mercenary_id(&self) -> u32 {
        self.mercenary_id
    }

    pub fn mercenary_id_set(&mut self, merc_id: u32) {
        self.has_mercenary = true;
        self.mercenary_id = merc_id;
    }

    pub fn directory_known(&self) -> bool {
        self.directory_known
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn set_level(&mut self, lvl: u32) -> u32 {
        self.level = lvl;
        self.level
    }

    pub fn portal_id(&self) -> u32 {
        self.portal_id
    }

    pub fn set_portal_id(&mut self, portal_id: u32) -> u32 {
        self.portal_id = portal_id;
        portal_id
    }
}

impl Entity for Player {
    fn initialized(&self) -> bool {
        return true;
    }

    fn id(&self) -> u32 {
        return self.id;
    }

    fn location(&self) -> Coordinate {
        return self.location;
    }
}

impl Update for Player {
    fn update(&self, msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

pub enum CharacterClass {
    Amazon = 0,
    Sorceress = 1,
    Necromancer = 2,
    Paladin = 3,
    Barbarian = 4,
    Druid = 5,
    Assassin = 6,
}

pub enum PlayerItemSlot {
    Helm = 0x01,
    Amulet = 0x02,
    Armor = 0x03,
    LeftWeapon = 0x04,
    RightWeapon = 0x05,
    LeftRing = 0x06,
    RightRing = 0x07,
    Belt = 0x08,
    Boots = 0x09,
    Gloves = 0x0A,
}

pub enum EmotePhrase {
    Help = 0x19,
    Follow = 0x1A,
    Gift = 0x1B,
    Thanks = 0x1C,
    Sorry = 0x1D,
    Bye = 0x1E,
    Die = 0x1F,
    Flee = 0x20,
}

// taken from https://bnetdocs.org/packet/98/d2gs-trade
//Press Accept button (unaccept) should be sent when placing items in the trade window as well
pub enum TradeAction {
    CancelTradeRequest = 0x02,
    AcceptTradeRequest = 0x03,
    PressAcceptButton = 0x04,
    UnpressAcceptButton = 0x07,
    RefreshWindow = 0x08,
    CloseStash = 0x12,
    WithdrawGold = 0x13,
    DepositGold = 0x14,
    CloseHoradricCube = 0x17,
}
