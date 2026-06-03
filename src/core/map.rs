use crate::core::act::Act;
use crate::core::area::Area;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CollisionFlag {
    None = 0x0000,
    BlockWalk = 0x0001,
    BlockLineOfSight = 0x0002,
    Wall = 0x0004,
    BlockPlayer = 0x0008,
    AlternateTile = 0x0010,
    Blank = 0x0020,
    Missile = 0x0040,
    Player = 0x0080,
    NpcLocation = 0x0100,
    Item = 0x0200,
    Object = 0x0400,
    ClosedDoor = 0x0800,
    NpcCollision = 0x1000,
    FriendlyNpc = 0x2000,
    Unknown = 0x4000,
    DeadBodyOrPortal = 0x8000,
    Special = 0xf000,
    Avoid = 0xffff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSeed(u32);

impl MapSeed {
    pub fn new(seed: u32) -> Option<Self> {
        is_valid_map_seed(seed).then_some(Self(seed))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

pub fn is_valid_map_seed(seed: u32) -> bool {
    seed > 0 && seed < u32::MAX
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

impl MapPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSize {
    pub width: u32,
    pub height: u32,
}

impl MapSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMap {
    pub id: u16,
    pub name: String,
    pub offset: MapPoint,
    pub size: MapSize,
    pub objects: Vec<MapObject>,
    pub collision: CollisionGrid,
}

impl GeneratedMap {
    pub fn is_blocked(&self, x: u32, y: u32) -> Option<bool> {
        self.collision.is_blocked(x, y)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapObject {
    pub id: u32,
    pub kind: MapObjectKind,
    pub position: MapPoint,
    pub name: Option<String>,
    pub operation: Option<u32>,
    pub class: Option<String>,
    pub is_good_exit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectKind {
    Exit,
    Npc,
    Object,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionGrid {
    width: u32,
    height: u32,
    rows: Vec<Vec<u32>>,
}

impl CollisionGrid {
    pub fn from_rle_rows(width: u32, height: u32, rows: Vec<Vec<u32>>) -> Self {
        Self {
            width,
            height,
            rows,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rows(&self) -> &[Vec<u32>] {
        &self.rows
    }

    pub fn is_blocked(&self, x: u32, y: u32) -> Option<bool> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let row = self.rows.get(y as usize)?;
        Some(rle_row_is_blocked(row, self.width, x))
    }

    pub fn expand_row(&self, y: u32) -> Option<Vec<bool>> {
        if y >= self.height {
            return None;
        }
        let row = self.rows.get(y as usize)?;
        Some(expand_rle_row(row, self.width))
    }
}

pub fn rle_row_is_blocked(row: &[u32], width: u32, x: u32) -> bool {
    let mut cursor = 0;
    let mut blocked = true;

    for run in row {
        let next = cursor + *run;
        if x < next.min(width) {
            return blocked;
        }
        cursor = next;
        blocked = !blocked;
        if cursor >= width {
            return blocked;
        }
    }

    blocked
}

pub fn expand_rle_row(row: &[u32], width: u32) -> Vec<bool> {
    (0..width)
        .map(|x| rle_row_is_blocked(row, width, x))
        .collect()
}

pub fn act_from_level_id(level_id: u16) -> Option<Act> {
    match level_id {
        1..=39 => Some(Act::Act1),
        40..=74 => Some(Act::Act2),
        75..=102 => Some(Act::Act3),
        103..=108 => Some(Act::Act4),
        109..=199 => Some(Act::Act5),
        _ => None,
    }
}

pub fn is_good_exit(source: Area, target: Area, staff_tomb: Option<Area>) -> bool {
    use Area::*;

    matches!(
        (source, target),
        (BloodMoor, DenOfEvil)
            | (TamoeHighland, PitLevel1)
            | (BlackMarsh, ForgottenTower)
            | (FarOasis, MaggotLairLevel1)
            | (ValleyOfSnakes, ClawViperTempleLevel1)
            | (RockyWaste, StonyTombLevel1)
            | (LostCity, AncientTunnels)
            | (SpiderForest, SpiderCavern)
            | (FlayerJungle, FlayerDungeonLevel1)
            | (KurastBazaar, RuinedTemple)
            | (CrystallinePassage, FrozenRiver)
    ) || staff_tomb == Some(target)
}

#[cfg(test)]
mod tests {
    use crate::core::act::Act;
    use crate::core::area::Area;

    use super::{
        act_from_level_id, expand_rle_row, is_good_exit, is_valid_map_seed, rle_row_is_blocked,
        CollisionGrid, MapSeed,
    };

    #[test]
    fn validates_unsigned_map_seed_range() {
        assert!(MapSeed::new(1).is_some());
        assert!(is_valid_map_seed(0x3607_656c));
        assert!(!is_valid_map_seed(0));
        assert!(!is_valid_map_seed(u32::MAX));
    }

    #[test]
    fn maps_level_ids_to_acts_like_map_generator() {
        assert_eq!(act_from_level_id(1), Some(Act::Act1));
        assert_eq!(act_from_level_id(39), Some(Act::Act1));
        assert_eq!(act_from_level_id(40), Some(Act::Act2));
        assert_eq!(act_from_level_id(75), Some(Act::Act3));
        assert_eq!(act_from_level_id(103), Some(Act::Act4));
        assert_eq!(act_from_level_id(109), Some(Act::Act5));
        assert_eq!(act_from_level_id(200), None);
    }

    #[test]
    fn decodes_collision_rle_rows_with_implicit_tail() {
        let row = [1, 5, 1];
        let expanded = expand_rle_row(&row, 9);

        assert_eq!(
            expanded,
            vec![true, false, false, false, false, false, true, false, false]
        );
        assert!(rle_row_is_blocked(&[1, 149], 152, 0));
        assert!(!rle_row_is_blocked(&[1, 149], 152, 149));
        assert!(rle_row_is_blocked(&[1, 149], 152, 150));
    }

    #[test]
    fn collision_grid_queries_bounds_and_rows() {
        let grid = CollisionGrid::from_rle_rows(7, 3, vec![vec![1, 5, 1], vec![2, 3, 2]]);

        assert_eq!(grid.is_blocked(0, 0), Some(true));
        assert_eq!(grid.is_blocked(1, 0), Some(false));
        assert_eq!(grid.is_blocked(6, 0), Some(true));
        assert_eq!(grid.is_blocked(0, 2), None);
        assert_eq!(grid.is_blocked(7, 0), None);
    }

    #[test]
    fn classifies_good_exits_from_map_generator_rules() {
        assert!(is_good_exit(Area::BloodMoor, Area::DenOfEvil, None));
        assert!(is_good_exit(Area::FarOasis, Area::MaggotLairLevel1, None));
        assert!(is_good_exit(
            Area::CanyonOfTheMagi,
            Area::TalRashasTomb4,
            Some(Area::TalRashasTomb4)
        ));
        assert!(!is_good_exit(Area::ColdPlains, Area::BurialGrounds, None));
    }
}
