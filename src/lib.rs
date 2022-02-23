#![feature(arbitrary_enum_discriminant)]

pub mod client;
pub mod core;

pub use self::client::client::Client;
pub use self::core::coordinate::Coordinate;
pub use self::core::game_state::GameState;
pub use self::core::object::item::{Item, ItemBufferCoord, ItemBufferId};
pub use self::core::party::PartyAction;
pub use self::core::protocol::ClientMessage;
pub use self::core::protocol::ServerMessage;
