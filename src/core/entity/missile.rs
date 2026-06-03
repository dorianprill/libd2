use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::update::Update;
use crate::ServerMessage;

pub struct Missile {
    id: u32,
    location: Coordinate,
}

impl Update for Missile {
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

impl Entity for Missile {
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
