use crate::core::version::GameEdition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: u8,
    pub height: u8,
}

impl GridSize {
    pub const fn new(width: u8, height: u8) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryProfile {
    pub edition: GameEdition,
    pub inventory: GridSize,
    pub stash: Option<GridSize>,
    pub cube: Option<GridSize>,
    pub belt_columns: u8,
    pub personal_stash_pages: u8,
    pub shared_stash_pages: u8,
    pub resurrected_item_encoding: bool,
}

impl InventoryProfile {
    pub const CLASSIC: Self = Self {
        edition: GameEdition::Classic,
        inventory: GridSize::new(10, 4),
        stash: Some(GridSize::new(6, 4)),
        cube: Some(GridSize::new(3, 4)),
        belt_columns: 4,
        personal_stash_pages: 1,
        shared_stash_pages: 0,
        resurrected_item_encoding: false,
    };

    pub const LORD_OF_DESTRUCTION: Self = Self {
        edition: GameEdition::LordOfDestruction,
        inventory: GridSize::new(10, 4),
        stash: Some(GridSize::new(6, 8)),
        cube: Some(GridSize::new(3, 4)),
        belt_columns: 4,
        personal_stash_pages: 1,
        shared_stash_pages: 0,
        resurrected_item_encoding: false,
    };

    pub const RESURRECTED: Self = Self {
        edition: GameEdition::Resurrected,
        inventory: GridSize::new(10, 4),
        stash: Some(GridSize::new(10, 10)),
        cube: Some(GridSize::new(3, 4)),
        belt_columns: 4,
        personal_stash_pages: 1,
        shared_stash_pages: 3,
        resurrected_item_encoding: true,
    };

    pub const REIGN_OF_THE_WARLOCK: Self = Self {
        edition: GameEdition::ReignOfTheWarlock,
        inventory: GridSize::new(10, 4),
        stash: Some(GridSize::new(10, 8)),
        cube: Some(GridSize::new(3, 4)),
        belt_columns: 4,
        personal_stash_pages: 1,
        shared_stash_pages: 3,
        resurrected_item_encoding: true,
    };

    pub const fn for_edition(edition: GameEdition) -> Self {
        match edition {
            GameEdition::Classic => Self::CLASSIC,
            GameEdition::LordOfDestruction => Self::LORD_OF_DESTRUCTION,
            GameEdition::Resurrected => Self::RESURRECTED,
            GameEdition::ReignOfTheWarlock => Self::REIGN_OF_THE_WARLOCK,
            GameEdition::Custom => Self::LORD_OF_DESTRUCTION,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemParent {
    Stored,
    Equipped,
    Belt,
    Cursor,
    Socketed,
    Unknown(u8),
}

impl ItemParent {
    pub fn from_save_value(value: u8) -> Self {
        match value {
            0 => Self::Stored,
            1 => Self::Equipped,
            2 => Self::Belt,
            4 => Self::Cursor,
            6 => Self::Socketed,
            other => Self::Unknown(other),
        }
    }

    pub fn save_value(self) -> u8 {
        match self {
            Self::Stored => 0,
            Self::Equipped => 1,
            Self::Belt => 2,
            Self::Cursor => 4,
            Self::Socketed => 6,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredItemContainer {
    Inventory,
    Cube,
    Stash,
    Unknown(u8),
}

impl StoredItemContainer {
    pub fn from_save_value(value: u8) -> Self {
        match value {
            1 => Self::Inventory,
            4 => Self::Cube,
            5 => Self::Stash,
            other => Self::Unknown(other),
        }
    }

    pub fn save_value(self) -> u8 {
        match self {
            Self::Inventory => 1,
            Self::Cube => 4,
            Self::Stash => 5,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPosition {
    pub parent: ItemParent,
    pub container: Option<StoredItemContainer>,
    pub x: u8,
    pub y: u8,
}

impl ItemPosition {
    pub const fn new(
        parent: ItemParent,
        container: Option<StoredItemContainer>,
        x: u8,
        y: u8,
    ) -> Self {
        Self {
            parent,
            container,
            x,
            y,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::version::GameEdition;

    use super::{GridSize, InventoryProfile, ItemParent, StoredItemContainer};

    #[test]
    fn inventory_profiles_separate_classic_lod_and_resurrected_storage() {
        assert_eq!(
            InventoryProfile::for_edition(GameEdition::Classic).stash,
            Some(GridSize::new(6, 4))
        );
        assert_eq!(
            InventoryProfile::for_edition(GameEdition::LordOfDestruction).stash,
            Some(GridSize::new(6, 8))
        );

        let resurrected = InventoryProfile::for_edition(GameEdition::Resurrected);
        assert_eq!(resurrected.stash, Some(GridSize::new(10, 10)));
        assert_eq!(resurrected.shared_stash_pages, 3);
        assert!(resurrected.resurrected_item_encoding);
    }

    #[test]
    fn item_location_values_preserve_unknown_variants() {
        assert_eq!(ItemParent::from_save_value(6), ItemParent::Socketed);
        assert_eq!(ItemParent::Unknown(99).save_value(), 99);

        assert_eq!(
            StoredItemContainer::from_save_value(4),
            StoredItemContainer::Cube
        );
        assert_eq!(StoredItemContainer::Unknown(77).save_value(), 77);
    }
}
