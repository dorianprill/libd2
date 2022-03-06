use std::fmt;

pub enum Act {
    Act1 = 0,
    Act2 = 1,
    Act3 = 2,
    Act4 = 3,
    Act5 = 4,
}

impl std::fmt::Display for Act {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use Act::*;
        let s = match *self {
            Act1 => "Act I",
            Act2 => "Act II",
            Act3 => "Act III",
            Act4 => "Act IV",
            Act5 => "Act V",
            _    => "None"
        };
        write!(formatter, "{}", s)
    }
}