use crate::core::coordinate::Coordinate;
use crate::core::update::Update;
#[allow(dead_code)]
use crate::core::{entity::Entity, network::d2gs::D2GSPacket};

pub struct Npc {
    id: u32,
    location: Coordinate,
}

impl Update for Npc {
    fn update(&self, packet: D2GSPacket) -> bool {
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
