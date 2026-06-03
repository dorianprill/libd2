#[allow(dead_code)]
use crate::core::entity::Entity;
use crate::core::update::Update;
use crate::{core::coordinate::Coordinate, ServerMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
    id: u32,
    class_id: Option<u16>,
    location: Coordinate,
    life_percent: Option<u8>,
    state: Option<u8>,
}

impl Npc {
    pub fn new(id: u32, location: Coordinate) -> Self {
        Self {
            id,
            class_id: None,
            location,
            life_percent: None,
            state: None,
        }
    }

    pub fn with_class(id: u32, class_id: u16, location: Coordinate, life_percent: u8) -> Self {
        Self {
            id,
            class_id: Some(class_id),
            location,
            life_percent: Some(life_percent),
            state: None,
        }
    }

    pub fn class_id(&self) -> Option<u16> {
        self.class_id
    }

    pub fn life_percent(&self) -> Option<u8> {
        self.life_percent
    }

    pub fn state(&self) -> Option<u8> {
        self.state
    }

    pub fn set_class_id(&mut self, class_id: u16) {
        self.class_id = Some(class_id);
    }

    pub fn set_location(&mut self, location: Coordinate) {
        self.location = location;
    }

    pub fn set_life_percent(&mut self, life_percent: u8) {
        self.life_percent = Some(life_percent);
    }

    pub fn set_state(&mut self, state: u8) {
        self.state = Some(state);
    }
}

impl Update for Npc {
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

impl Entity for Npc {
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

#[allow(dead_code)]
pub enum TradeType {
    Trade = 0x01,
    Gamble = 0x02,
}
