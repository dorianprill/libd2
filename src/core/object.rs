pub mod item;

use crate::core::coordinate::Coordinate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObject {
    id: u32,
    object_type: u8,
    class_id: u16,
    location: Coordinate,
    state: u8,
    interaction: u8,
}

impl WorldObject {
    pub fn new(
        id: u32,
        object_type: u8,
        class_id: u16,
        location: Coordinate,
        state: u8,
        interaction: u8,
    ) -> Self {
        Self {
            id,
            object_type,
            class_id,
            location,
            state,
            interaction,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn object_type(&self) -> u8 {
        self.object_type
    }

    pub fn class_id(&self) -> u16 {
        self.class_id
    }

    pub fn location(&self) -> Coordinate {
        self.location
    }

    pub fn state(&self) -> u8 {
        self.state
    }

    pub fn interaction(&self) -> u8 {
        self.interaction
    }
}
