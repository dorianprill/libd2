use std::collections::HashMap;
use std::fmt;

use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::update::Update;
use crate::ServerMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MercenaryClass {
    FromAct1 = 0x010f,
    FromAct2 = 0x0152,
    FromAct3 = 0x0167,
    FromAct5 = 0x0231,
}

impl MercenaryClass {
    pub fn from_class_id(class_id: u16) -> Option<Self> {
        match class_id {
            0x010f => Some(Self::FromAct1),
            0x0152 => Some(Self::FromAct2),
            0x0167 => Some(Self::FromAct3),
            0x0231 => Some(Self::FromAct5),
            _ => None,
        }
    }
}

impl std::fmt::Display for MercenaryClass {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use MercenaryClass::*;
        let s = match *self {
            FromAct1 => "Rogue Scout",
            FromAct2 => "Desert Mercenary",
            FromAct3 => "Iron Wolf",
            FromAct5 => "Barbarian",
        };
        write!(formatter, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mercenary {
    id: u32,
    class_id: u16,
    owner_id: u32,
    skill_id: u8,
    seed2: u32,
    init_seed: u32,
    location: Coordinate,
    world_location_known: bool,
    revive_cost: Option<u16>,
    revive_name_id: Option<u16>,
    life_percent: Option<u8>,
    stats: HashMap<u16, u32>,
}

impl Mercenary {
    pub fn new(
        id: u32,
        class_id: u16,
        owner_id: u32,
        skill_id: u8,
        seed2: u32,
        init_seed: u32,
    ) -> Self {
        Self {
            id,
            class_id,
            owner_id,
            skill_id,
            seed2,
            init_seed,
            location: Coordinate::new(0, 0),
            world_location_known: false,
            revive_cost: None,
            revive_name_id: None,
            life_percent: None,
            stats: HashMap::new(),
        }
    }

    pub fn class_id(&self) -> u16 {
        self.class_id
    }

    pub fn class(&self) -> Option<MercenaryClass> {
        MercenaryClass::from_class_id(self.class_id)
    }

    pub fn refresh_assignment(
        &mut self,
        class_id: u16,
        owner_id: u32,
        skill_id: u8,
        seed2: u32,
        init_seed: u32,
    ) {
        self.class_id = class_id;
        self.owner_id = owner_id;
        self.skill_id = skill_id;
        self.seed2 = seed2;
        self.init_seed = init_seed;
    }

    pub fn owner_id(&self) -> u32 {
        self.owner_id
    }

    pub fn skill_id(&self) -> u8 {
        self.skill_id
    }

    pub fn seed2(&self) -> u32 {
        self.seed2
    }

    pub fn init_seed(&self) -> u32 {
        self.init_seed
    }

    pub fn world_location_known(&self) -> bool {
        self.world_location_known
    }

    pub fn set_location(&mut self, location: Coordinate) {
        self.location = location;
        self.world_location_known = true;
    }

    pub fn clear_world_location(&mut self) {
        self.world_location_known = false;
    }

    pub fn revive_cost(&self) -> Option<u16> {
        self.revive_cost
    }

    pub fn revive_name_id(&self) -> Option<u16> {
        self.revive_name_id
    }

    pub fn set_revive_info(&mut self, revive_name_id: u16, revive_cost: u16) {
        self.revive_name_id = Some(revive_name_id);
        self.revive_cost = Some(revive_cost);
    }

    pub fn life_percent(&self) -> Option<u8> {
        self.life_percent
    }

    pub fn set_life_percent(&mut self, life_percent: u8) {
        self.life_percent = Some(life_percent);
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

impl Update for Mercenary {
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        true
    }
}

impl Entity for Mercenary {
    fn initialized(&self) -> bool {
        true
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn location(&self) -> Coordinate {
        self.location
    }
}
