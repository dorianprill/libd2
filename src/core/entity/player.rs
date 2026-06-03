// Player struct

use std::collections::HashMap;

use crate::core::character_class::CharacterClass;
use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::update::Update;
use crate::ServerMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    stats: HashMap<u16, u32>,
    // TODO
    // stash:       Container;
    // cube:        Container;
    // belt:        Belt;
}

impl Player {
    pub fn new(
        id: u32,
        class: CharacterClass,
        name: impl Into<String>,
        location: Coordinate,
    ) -> Self {
        Self {
            class,
            name: name.into(),
            id,
            location,
            has_mercenary: false,
            directory_known: false,
            mercenary_id: 0,
            level: 0,
            portal_id: 0,
            stats: HashMap::new(),
        }
    }

    pub fn class(&self) -> CharacterClass {
        self.class
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_class(&mut self, class: CharacterClass) {
        self.class = class;
    }

    pub fn set_location(&mut self, location: Coordinate) {
        self.location = location;
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

    pub fn stat(&self, stat_id: u16) -> Option<u32> {
        self.stats.get(&stat_id).copied()
    }

    pub fn set_stat(&mut self, stat_id: u16, value: u32) {
        self.stats.insert(stat_id, value);
    }

    pub fn add_stat(&mut self, stat_id: u16, value: u32) {
        let current = self.stat(stat_id).unwrap_or_default();
        self.set_stat(stat_id, current.saturating_add(value));
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
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

pub enum PlayerItemSlot {
    Helm = 0x01,
    Amulet = 0x02,
    Armor = 0x03,
    LeftWeapon1 = 0x04,
    RightWeapon1 = 0x05,
    LeftRing = 0x06,
    RightRing = 0x07,
    Belt = 0x08,
    Boots = 0x09,
    Gloves = 0x0A,
    LeftWeapon2 = 0x0B,
    RightWeapon2 = 0x0C,
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
