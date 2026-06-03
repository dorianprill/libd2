#[allow(dead_code)]
pub type ItemId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    id: ItemId,
}

impl Item {
    pub fn new(id: ItemId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> ItemId {
        self.id
    }
}

pub enum MercenaryItemSlot {
    // TODO unknown at this time - from bnetdocs:
    // These values have been recorded for mercenary body locations, but aren't confirmed:
    // (Note, each location ID is prefixed with 0x61)
    // Example: 1A 64 00 00 00 61 02 00 00 (Move item 0x64 to Mercenary Right-hand weapon)
}

#[repr(u8)]
pub enum ItemBufferId {
    CharacterInventory = 0x00,
    NpcVendor = 0x01,
    TradeWindow = 0x02,
    HoradricCube = 0x03,
    Stash = 0x04,
}

pub type ItemBufferCoord = u32;
