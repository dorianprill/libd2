use crate::core::coordinate::Coordinate;
use crate::core::update::Update;
use crate::core::{entity::Entity, network::d2gs::D2GSPacket};

#[allow(dead_code)]
pub struct Mercenary {
    id: u32,
    location: Coordinate,
}

impl Update for Mercenary {
    fn update(&self, packet: D2GSPacket) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

impl Entity for Mercenary {
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
