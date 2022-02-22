// TODO Entity should really be a trait/interface

pub mod mercenary;
pub mod missile;
pub mod npc;
pub mod player;

use crate::core::coordinate::Coordinate;

#[allow(dead_code)]
pub enum EntityType {
    Player = 0x00,
    NPC = 0x01,         // NPC, Mercenary, Monster
    WorldEntity = 0x02, // Stash, Waypoint, Chests, Portals, others.
    Missile = 0x03,
    Item = 0x04,
    Entrance = 0x05,
}

pub type EntityId = u32;

#[allow(dead_code)]
pub trait Entity {
    fn initialized(&self) -> bool;

    fn id(&self) -> u32;

    fn location(&self) -> Coordinate;
}
