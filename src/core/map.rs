use std::fmt;

use serde::Deserialize;

use crate::core::act::Act;
use crate::core::area::Area;
use crate::core::game_state::Difficulty;

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

/// Returns the act containing a typed area.
///
/// Diablo II's level ids are grouped by act. External map generators use the
/// numeric act as a generation scope, while packet/save code usually talks in
/// terms of level ids. This helper keeps the conversion in one place.
pub fn act_for_area(area: Area) -> Option<Act> {
    act_from_level_id(area as u16)
}

/// A validated request for generating or importing one Diablo II area map.
///
/// The external map generator contract uses the same primitive inputs as the
/// game: map seed, difficulty index, act index, and level id. This type keeps
/// those values tied together and rejects impossible combinations before a
/// caller invokes a native generator, an external helper, or JSON normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGenerationRequest {
    seed: MapSeed,
    difficulty: Difficulty,
    act: Act,
    area: Area,
}

impl MapGenerationRequest {
    /// Builds a request from an already validated seed and explicit act.
    pub fn new(
        seed: MapSeed,
        difficulty: Difficulty,
        act: Act,
        area: Area,
    ) -> Result<Self, MapGenerationRequestError> {
        let actual_act = act_for_area(area).ok_or(MapGenerationRequestError::UnknownArea(area))?;
        if actual_act != act {
            return Err(MapGenerationRequestError::AreaActMismatch {
                area,
                requested_act: act,
                actual_act,
            });
        }

        Ok(Self {
            seed,
            difficulty,
            act,
            area,
        })
    }

    /// Builds a request from raw packet/game values and derives the act from
    /// the selected area.
    pub fn for_area(
        seed: u32,
        difficulty: Difficulty,
        area: Area,
    ) -> Result<Self, MapGenerationRequestError> {
        let seed = MapSeed::new(seed).ok_or(MapGenerationRequestError::InvalidSeed(seed))?;
        let act = act_for_area(area).ok_or(MapGenerationRequestError::UnknownArea(area))?;
        Self::new(seed, difficulty, act, area)
    }

    pub fn seed(self) -> MapSeed {
        self.seed
    }

    pub fn difficulty(self) -> Difficulty {
        self.difficulty
    }

    pub fn act(self) -> Act {
        self.act
    }

    pub fn area(self) -> Area {
        self.area
    }

    /// Numeric seed value expected by generator helpers.
    pub fn generator_seed(self) -> u32 {
        self.seed.get()
    }

    /// Numeric difficulty value used by external generators: normal `0`,
    /// nightmare `1`, hell `2`.
    pub fn generator_difficulty(self) -> u8 {
        difficulty_index(self.difficulty)
    }

    /// Numeric act value used by external generators: Act I `0` through Act V
    /// `4`.
    pub fn generator_act(self) -> u8 {
        self.act as u8
    }

    /// Numeric level id used by external generators.
    pub fn generator_level(self) -> u16 {
        self.area as u16
    }

    /// Imports either a single generated level JSON object or an HTTP-style
    /// generator response containing `seed`, `difficulty`, `act`, and `levels`.
    ///
    /// The returned [`GeneratedMap`] is guaranteed to match this request's area
    /// id. When the wrapper includes seed/difficulty/act metadata, those values
    /// are checked as well. A wrapper act of `-1` is accepted as the external
    /// generator's "whole world" scope.
    pub fn normalize_mapgen_json(self, input: &str) -> Result<GeneratedMap, MapGenerationError> {
        let output: MapgenOutput =
            serde_json::from_str(input).map_err(GeneratedMapJsonError::from)?;
        match output {
            MapgenOutput::Level(level) => {
                let map = GeneratedMap::try_from(level)?;
                self.validate_generated_map(map)
            }
            MapgenOutput::Response(response) => self.normalize_response(response),
        }
    }

    fn normalize_response(
        self,
        response: MapgenResponse,
    ) -> Result<GeneratedMap, MapGenerationError> {
        if let Some(seed) = response.seed
            && seed != self.generator_seed() as u64
        {
            return Err(MapGenerationError::ResponseSeedMismatch {
                expected: self.generator_seed(),
                actual: seed,
            });
        }

        if let Some(difficulty) = response.difficulty
            && difficulty != self.generator_difficulty() as u64
        {
            return Err(MapGenerationError::ResponseDifficultyMismatch {
                expected: self.generator_difficulty(),
                actual: difficulty,
            });
        }

        if let Some(act) = response.act
            && act != -1
            && act != self.generator_act() as i64
        {
            return Err(MapGenerationError::ResponseActMismatch {
                expected: self.generator_act(),
                actual: act,
            });
        }

        let requested_level = self.generator_level() as u64;
        let Some(level) = response
            .levels
            .into_iter()
            .find(|level| level.id == requested_level)
        else {
            return Err(MapGenerationError::MissingRequestedLevel {
                area: self.area,
                level_id: self.generator_level(),
            });
        };

        let map = GeneratedMap::try_from(level)?;
        self.validate_generated_map(map)
    }

    fn validate_generated_map(self, map: GeneratedMap) -> Result<GeneratedMap, MapGenerationError> {
        if map.id != self.generator_level() {
            return Err(MapGenerationError::UnexpectedLevel {
                expected_area: self.area,
                expected_level_id: self.generator_level(),
                actual_level_id: map.id,
            });
        }

        Ok(map)
    }
}

/// Error returned when a map-generation request is internally inconsistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapGenerationRequestError {
    InvalidSeed(u32),
    UnknownArea(Area),
    AreaActMismatch {
        area: Area,
        requested_act: Act,
        actual_act: Act,
    },
}

impl fmt::Display for MapGenerationRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSeed(seed) => write!(formatter, "invalid map seed {}", seed),
            Self::UnknownArea(area) => write!(formatter, "area {} has no map-generator act", area),
            Self::AreaActMismatch {
                area,
                requested_act,
                actual_act,
            } => write!(
                formatter,
                "area {} belongs to {}, not {}",
                area, actual_act, requested_act
            ),
        }
    }
}

impl std::error::Error for MapGenerationRequestError {}

/// Error returned while normalizing generated-map output for a request.
#[derive(Debug)]
pub enum MapGenerationError {
    GeneratedMap(GeneratedMapJsonError),
    ResponseSeedMismatch {
        expected: u32,
        actual: u64,
    },
    ResponseDifficultyMismatch {
        expected: u8,
        actual: u64,
    },
    ResponseActMismatch {
        expected: u8,
        actual: i64,
    },
    MissingRequestedLevel {
        area: Area,
        level_id: u16,
    },
    UnexpectedLevel {
        expected_area: Area,
        expected_level_id: u16,
        actual_level_id: u16,
    },
}

impl fmt::Display for MapGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GeneratedMap(error) => write!(formatter, "{}", error),
            Self::ResponseSeedMismatch { expected, actual } => write!(
                formatter,
                "generated-map response seed {} does not match requested seed {}",
                actual, expected
            ),
            Self::ResponseDifficultyMismatch { expected, actual } => write!(
                formatter,
                "generated-map response difficulty {} does not match requested difficulty {}",
                actual, expected
            ),
            Self::ResponseActMismatch { expected, actual } => write!(
                formatter,
                "generated-map response act {} does not match requested act {}",
                actual, expected
            ),
            Self::MissingRequestedLevel { area, level_id } => write!(
                formatter,
                "generated-map response did not contain requested area {} ({})",
                area, level_id
            ),
            Self::UnexpectedLevel {
                expected_area,
                expected_level_id,
                actual_level_id,
            } => write!(
                formatter,
                "generated level {} does not match requested area {} ({})",
                actual_level_id, expected_area, expected_level_id
            ),
        }
    }
}

impl std::error::Error for MapGenerationError {}

impl From<GeneratedMapJsonError> for MapGenerationError {
    fn from(error: GeneratedMapJsonError) -> Self {
        Self::GeneratedMap(error)
    }
}

fn difficulty_index(difficulty: Difficulty) -> u8 {
    match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 1,
        Difficulty::Hell => 2,
    }
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

    /// Parses the JSON map contract emitted by external Diablo II map
    /// generators such as `@diablo2/map`.
    ///
    /// The external generator already performed the game-specific seed
    /// expansion and room loading. This method only imports the resulting level
    /// description: level id/name, world offset, dimensions, important preset
    /// objects, and the alternating blocked/open RLE collision rows. It is the
    /// boundary we use while native Rust seed generation is incomplete.
    pub fn from_mapgen_json(input: &str) -> Result<Self, GeneratedMapJsonError> {
        let parsed: MapgenLevel = serde_json::from_str(input)?;
        parsed.try_into()
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

impl MapObjectKind {
    fn from_mapgen_type(value: &str) -> Self {
        match value {
            "exit" => Self::Exit,
            "npc" => Self::Npc,
            "object" => Self::Object,
            _ => Self::Unknown,
        }
    }
}

/// Error returned while importing generated-map JSON.
#[derive(Debug)]
pub enum GeneratedMapJsonError {
    Json(serde_json::Error),
    InvalidLevelId(u64),
    InvalidObjectId(u64),
    InvalidCoordinate { field: &'static str, value: i64 },
    InvalidSize { field: &'static str, value: u64 },
    InvalidOperation(u64),
}

impl fmt::Display for GeneratedMapJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to parse generated map JSON: {}", error),
            Self::InvalidLevelId(value) => {
                write!(formatter, "invalid generated-map level id {}", value)
            }
            Self::InvalidObjectId(value) => {
                write!(formatter, "invalid generated-map object id {}", value)
            }
            Self::InvalidCoordinate { field, value } => {
                write!(
                    formatter,
                    "invalid generated-map coordinate {}={}",
                    field, value
                )
            }
            Self::InvalidSize { field, value } => {
                write!(formatter, "invalid generated-map size {}={}", field, value)
            }
            Self::InvalidOperation(value) => {
                write!(
                    formatter,
                    "invalid generated-map object operation {}",
                    value
                )
            }
        }
    }
}

impl std::error::Error for GeneratedMapJsonError {}

impl From<serde_json::Error> for GeneratedMapJsonError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Deserialize)]
struct MapgenLevel {
    id: u64,
    name: String,
    offset: MapgenPoint,
    size: MapgenSize,
    #[serde(default)]
    objects: Vec<MapgenObject>,
    #[serde(rename = "map")]
    collision_rows: Vec<Vec<u32>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MapgenOutput {
    Level(MapgenLevel),
    Response(MapgenResponse),
}

#[derive(Debug, Deserialize)]
struct MapgenResponse {
    seed: Option<u64>,
    difficulty: Option<u64>,
    act: Option<i64>,
    levels: Vec<MapgenLevel>,
}

#[derive(Debug, Deserialize)]
struct MapgenPoint {
    x: i64,
    y: i64,
}

#[derive(Debug, Deserialize)]
struct MapgenSize {
    width: u64,
    height: u64,
}

#[derive(Debug, Deserialize)]
struct MapgenObject {
    id: u64,
    #[serde(rename = "type")]
    object_type: String,
    x: i64,
    y: i64,
    name: Option<String>,
    #[serde(rename = "op")]
    operation: Option<u64>,
    #[serde(rename = "class")]
    class_name: Option<String>,
    #[serde(default, rename = "isGoodExit")]
    is_good_exit: bool,
}

impl TryFrom<MapgenLevel> for GeneratedMap {
    type Error = GeneratedMapJsonError;

    fn try_from(value: MapgenLevel) -> Result<Self, Self::Error> {
        let id =
            u16::try_from(value.id).map_err(|_| GeneratedMapJsonError::InvalidLevelId(value.id))?;
        let width =
            u32::try_from(value.size.width).map_err(|_| GeneratedMapJsonError::InvalidSize {
                field: "width",
                value: value.size.width,
            })?;
        let height =
            u32::try_from(value.size.height).map_err(|_| GeneratedMapJsonError::InvalidSize {
                field: "height",
                value: value.size.height,
            })?;

        Ok(Self {
            id,
            name: value.name,
            offset: MapPoint::new(
                i32_coord("offset.x", value.offset.x)?,
                i32_coord("offset.y", value.offset.y)?,
            ),
            size: MapSize::new(width, height),
            objects: value
                .objects
                .into_iter()
                .map(MapObject::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            collision: CollisionGrid::from_rle_rows(width, height, value.collision_rows),
        })
    }
}

impl TryFrom<MapgenObject> for MapObject {
    type Error = GeneratedMapJsonError;

    fn try_from(value: MapgenObject) -> Result<Self, Self::Error> {
        let id = u32::try_from(value.id)
            .map_err(|_| GeneratedMapJsonError::InvalidObjectId(value.id))?;
        let operation = value
            .operation
            .map(|operation| {
                u32::try_from(operation)
                    .map_err(|_| GeneratedMapJsonError::InvalidOperation(operation))
            })
            .transpose()?;

        Ok(Self {
            id,
            kind: MapObjectKind::from_mapgen_type(&value.object_type),
            position: MapPoint::new(
                i32_coord("object.x", value.x)?,
                i32_coord("object.y", value.y)?,
            ),
            name: value.name,
            operation,
            class: value.class_name,
            is_good_exit: value.is_good_exit,
        })
    }
}

fn i32_coord(field: &'static str, value: i64) -> Result<i32, GeneratedMapJsonError> {
    i32::try_from(value).map_err(|_| GeneratedMapJsonError::InvalidCoordinate { field, value })
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
    use crate::core::game_state::Difficulty;

    use super::{
        CollisionGrid, GeneratedMap, GeneratedMapJsonError, MapGenerationError,
        MapGenerationRequest, MapGenerationRequestError, MapObjectKind, MapSeed, act_from_level_id,
        expand_rle_row, is_good_exit, is_valid_map_seed, rle_row_is_blocked,
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
    fn map_generation_request_derives_generator_values_for_area() {
        let request =
            MapGenerationRequest::for_area(0x3607_656c, Difficulty::Hell, Area::ArcaneSanctuary)
                .expect("area request should be valid");

        assert_eq!(request.generator_seed(), 0x3607_656c);
        assert_eq!(request.difficulty(), Difficulty::Hell);
        assert_eq!(request.generator_difficulty(), 2);
        assert_eq!(request.act(), Act::Act2);
        assert_eq!(request.generator_act(), 1);
        assert_eq!(request.area(), Area::ArcaneSanctuary);
        assert_eq!(request.generator_level(), 74);
    }

    #[test]
    fn map_generation_request_rejects_invalid_seed_and_act_area_mismatch() {
        let invalid_seed = MapGenerationRequest::for_area(0, Difficulty::Normal, Area::BloodMoor)
            .expect_err("zero seed is invalid");
        assert_eq!(invalid_seed, MapGenerationRequestError::InvalidSeed(0));

        let mismatch = MapGenerationRequest::new(
            MapSeed::new(1).expect("seed is valid"),
            Difficulty::Normal,
            Act::Act1,
            Area::ArcaneSanctuary,
        )
        .expect_err("Arcane Sanctuary is not Act I");
        assert_eq!(
            mismatch,
            MapGenerationRequestError::AreaActMismatch {
                area: Area::ArcaneSanctuary,
                requested_act: Act::Act1,
                actual_act: Act::Act2,
            }
        );
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

    #[test]
    fn imports_blacha_map_generator_json() {
        let map =
            GeneratedMap::from_mapgen_json(arcane_level_json()).expect("map JSON should parse");

        assert_eq!(map.id, 74);
        assert_eq!(map.name, "Arcane Sanctuary");
        assert_eq!(map.offset.x, 25000);
        assert_eq!(map.size.width, 7);
        assert_eq!(map.objects.len(), 2);
        assert_eq!(map.objects[0].kind, MapObjectKind::Exit);
        assert!(map.objects[0].is_good_exit);
        assert_eq!(map.objects[1].operation, Some(23));
        assert_eq!(map.objects[1].class.as_deref(), Some("waypoint"));
        assert_eq!(map.is_blocked(0, 0), Some(true));
        assert_eq!(map.is_blocked(1, 0), Some(false));
        assert_eq!(map.is_blocked(6, 0), Some(true));
    }

    #[test]
    fn map_generation_request_normalizes_single_level_json() {
        let request = MapGenerationRequest::for_area(
            0x3607_656c,
            Difficulty::Nightmare,
            Area::ArcaneSanctuary,
        )
        .expect("area request should be valid");

        let map = request
            .normalize_mapgen_json(arcane_level_json())
            .expect("single generated level should normalize");

        assert_eq!(map.id, request.generator_level());
        assert_eq!(map.name, "Arcane Sanctuary");
    }

    #[test]
    fn map_generation_request_selects_level_from_wrapped_response() {
        let request =
            MapGenerationRequest::for_area(0x3607_656c, Difficulty::Hell, Area::ArcaneSanctuary)
                .expect("area request should be valid");
        let response = format!(
            r#"{{
                "id": "fixture",
                "seed": {},
                "difficulty": {},
                "act": {},
                "levels": [{}]
            }}"#,
            request.generator_seed(),
            request.generator_difficulty(),
            request.generator_act(),
            arcane_level_json()
        );

        let map = request
            .normalize_mapgen_json(&response)
            .expect("wrapped response should normalize");

        assert_eq!(map.id, 74);
        assert_eq!(map.objects[0].kind, MapObjectKind::Exit);
        assert_eq!(map.is_blocked(1, 0), Some(false));
    }

    #[test]
    fn map_generation_request_rejects_response_metadata_mismatch() {
        let request =
            MapGenerationRequest::for_area(0x3607_656c, Difficulty::Normal, Area::ArcaneSanctuary)
                .expect("area request should be valid");
        let response = format!(
            r#"{{
                "seed": {},
                "difficulty": 0,
                "act": 1,
                "levels": [{}]
            }}"#,
            request.generator_seed() + 1,
            arcane_level_json()
        );

        let error = request
            .normalize_mapgen_json(&response)
            .expect_err("response seed should not match request");

        assert!(matches!(
            error,
            MapGenerationError::ResponseSeedMismatch { .. }
        ));
    }

    #[test]
    fn generated_map_json_rejects_out_of_range_ids() {
        let error = GeneratedMap::from_mapgen_json(
            r#"{
                "id": 70000,
                "name": "Too Large",
                "offset": { "x": 0, "y": 0 },
                "size": { "width": 1, "height": 1 },
                "objects": [],
                "map": [[]]
            }"#,
        )
        .expect_err("level id should not fit in u16");

        assert!(matches!(
            error,
            GeneratedMapJsonError::InvalidLevelId(70000)
        ));
    }

    fn arcane_level_json() -> &'static str {
        r#"{
            "type": "map",
            "id": 74,
            "name": "Arcane Sanctuary",
            "offset": { "x": 25000, "y": 5000 },
            "size": { "width": 7, "height": 3 },
            "objects": [
                {
                    "id": 53,
                    "type": "exit",
                    "x": 137,
                    "y": 0,
                    "name": "Palace Cellar Level 2",
                    "isGoodExit": true
                },
                {
                    "id": 402,
                    "type": "object",
                    "x": 449,
                    "y": 449,
                    "name": "Waypoint",
                    "op": 23,
                    "class": "waypoint"
                }
            ],
            "map": [
                [1, 5, 1],
                [2, 3, 2],
                [1, 5, 1]
            ]
        }"#
    }
}
