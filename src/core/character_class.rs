use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterClass {
    Amazon = 0,
    Sorceress = 1,
    Necromancer = 2,
    Paladin = 3,
    Barbarian = 4,
    Druid = 5,
    Assassin = 6,
    Warlock = 7,
}

impl CharacterClass {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Amazon),
            1 => Some(Self::Sorceress),
            2 => Some(Self::Necromancer),
            3 => Some(Self::Paladin),
            4 => Some(Self::Barbarian),
            5 => Some(Self::Druid),
            6 => Some(Self::Assassin),
            7 => Some(Self::Warlock),
            _ => None,
        }
    }
}

impl std::fmt::Display for CharacterClass {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use CharacterClass::*;
        let s = match *self {
            Amazon => "Amazon",
            Sorceress => "Sorceress",
            Necromancer => "Necromancer",
            Paladin => "Paladin",
            Barbarian => "Barbarian",
            Druid => "Druid",
            Assassin => "Assassin",
            Warlock => "Warlock",
        };
        write!(formatter, "{}", s)
    }
}
