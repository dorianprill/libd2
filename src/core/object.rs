pub mod item;

use crate::core::coordinate::Coordinate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObject {
    id: u32,
    object_type: u8,
    class_id: u16,
    location: Coordinate,
    state: u32,
    interaction: u8,
    portal_flags: Option<u8>,
    is_targetable: Option<u8>,
}

impl WorldObject {
    pub fn new(
        id: u32,
        object_type: u8,
        class_id: u16,
        location: Coordinate,
        state: u32,
        interaction: u8,
    ) -> Self {
        Self {
            id,
            object_type,
            class_id,
            location,
            state,
            interaction,
            portal_flags: None,
            is_targetable: None,
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

    pub fn state(&self) -> u32 {
        self.state
    }

    pub fn interaction(&self) -> u8 {
        self.interaction
    }

    pub fn portal_flags(&self) -> Option<u8> {
        self.portal_flags
    }

    pub fn is_targetable(&self) -> Option<u8> {
        self.is_targetable
    }

    pub fn set_state(&mut self, state: u32) {
        self.state = state;
    }

    pub fn set_portal_flags(&mut self, portal_flags: u8) {
        self.portal_flags = Some(portal_flags);
    }

    pub fn set_targetable(&mut self, is_targetable: u8) {
        self.is_targetable = Some(is_targetable);
    }
}
