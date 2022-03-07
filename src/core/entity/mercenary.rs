use std::fmt;

use crate::ServerMessage;
use crate::core::coordinate::Coordinate;
use crate::core::update::Update;
use crate::core::{entity::Entity};

pub enum MercenaryClass {
    FromAct1 = 0x010f,
    FromAct2 = 0x0152,
    FromAct3 = 0x0167,
    FromAct5 = 0x0231
}

impl std::fmt::Display for MercenaryClass {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use MercenaryClass::*;
        let s = match *self {
            FromAct1 => "Rogue Scout",
            FromAct2 => "Desert Mercenary",
            FromAct3 => "Iron Wolf",
            FromAct5 => "Barbarian",
            _    => "None"
        };
        write!(formatter, "{}", s)
    }
}


#[allow(dead_code)]
pub struct Mercenary {
    id: u32,
    class: MercenaryClass,
    location: Coordinate,
}

impl Update for Mercenary {
    fn update(&self, msg: ServerMessage) -> bool {
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
