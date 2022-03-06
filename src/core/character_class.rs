use std::fmt;

pub enum CharacterClass {
    Amazon      = 0,
    Sorceress   = 1,
    Necromancer = 2,
    Paladin     = 3,
    Barbarian   = 4,
    Druid       = 5,
    Assassin    = 6,
}

impl std::fmt::Display for CharacterClass {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use CharacterClass::*;
        let s = match *self {
            Amazon      => "Amazon",
            Sorceress   => "Sorceress",
            Necromancer => "Necromancer",
            Paladin     => "Paladin",
            Barbarian   => "Barbarian",
            Druid       => "Druid",
            Assassin    => "Assassin",
            _           => "None"
        };
        write!(formatter, "{}", s)
    }
}