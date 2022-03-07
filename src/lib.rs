#![feature(arbitrary_enum_discriminant)]

pub mod client;
pub mod core;

pub use self::client::client::Client;

pub use crate::core::act::Act;
pub use crate::core::area::Area;
pub use crate::core::character_class::CharacterClass;
pub use crate::core::coordinate::Coordinate;
pub use crate::core::game_state::GameState;
pub use crate::core::object::item::{Item, ItemBufferCoord, ItemBufferId};
pub use crate::core::party::PartyAction;
pub use crate::core::protocol::ClientMessage;
pub use crate::core::protocol::ServerMessage;
pub use crate::core::unit_stat::UnitStat;

