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
        if let Some(seed) = response.seed {
            if seed != self.generator_seed() as u64 {
                return Err(MapGenerationError::ResponseSeedMismatch {
                    expected: self.generator_seed(),
                    actual: seed,
                });
            }
        }

        if let Some(difficulty) = response.difficulty {
            if difficulty != self.generator_difficulty() as u64 {
                return Err(MapGenerationError::ResponseDifficultyMismatch {
                    expected: self.generator_difficulty(),
                    actual: difficulty,
                });
            }
        }

        if let Some(act) = response.act {
            if act != -1 && act != self.generator_act() as i64 {
                return Err(MapGenerationError::ResponseActMismatch {
                    expected: self.generator_act(),
                    actual: act,
                });
            }
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

/// Diablo II map-generation implementation target.
///
/// Public reverse-engineering sources agree that LoD map generation is broadly
/// stable across the post-1.10 legacy line, but libd2 treats patch compatibility
/// as fixture-proven rather than assumed. The first native implementation target
/// is therefore the currently playable legacy Battle.net client, LoD 1.14d.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapGeneratorProfile {
    /// Legacy Diablo II: Lord of Destruction 1.14d.
    Lod114d,
}

impl MapGeneratorProfile {
    /// Human-readable patch label for diagnostics and fixture metadata.
    pub const fn patch_label(self) -> &'static str {
        match self {
            Self::Lod114d => "LoD 1.14d",
        }
    }

    /// Stable lowercase label used in fixture metadata.
    pub const fn fixture_label(self) -> &'static str {
        match self {
            Self::Lod114d => "lod_1_14d",
        }
    }

    /// Parses the stable label used in fixture metadata.
    pub fn from_fixture_label(label: &str) -> Option<Self> {
        match label {
            "lod_1_14d" => Some(Self::Lod114d),
            _ => None,
        }
    }
}

impl fmt::Display for MapGeneratorProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.patch_label())
    }
}

/// Native seed-to-map generator facade.
///
/// Diablo II's generated map state has three distinct layers:
///
/// - DRLG layout: room graph, room coordinates, exits, and level bounds.
/// - Static assets: `Levels.txt`, `LvlMaze.txt`, `LvlPrest.txt`, `LvlTypes.txt`,
///   `LvlWarp.txt`, `Objects.txt`, `MonStats.txt`, DS1 files, and DT1 collision.
/// - Runtime extraction: room collision and preset units after D2Common has
///   initialized an act/level for a seed.
///
/// Existing resource projects mostly use the third path by loading the original
/// D2 DLLs and walking `Level -> Room2 -> Room1 -> CollMap`. This facade is the
/// starting point for a clean Rust implementation of the first two layers. It
/// intentionally reports unsupported areas until each area family is ported and
/// cross-checked against 1.14d fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMapGenerator {
    profile: MapGeneratorProfile,
}

impl NativeMapGenerator {
    /// Creates a generator targeting LoD 1.14d map semantics.
    pub const fn lod_1_14d() -> Self {
        Self {
            profile: MapGeneratorProfile::Lod114d,
        }
    }

    /// Returns the patch profile this generator promises to match.
    pub const fn profile(self) -> MapGeneratorProfile {
        self.profile
    }

    /// Generates one area map from a validated seed/difficulty/area request.
    ///
    /// This is the stable call site that d2helper/pathfinding will eventually
    /// use. The current branch only establishes the API and patch boundary; the
    /// first real implementation should land as a narrow area-family port with
    /// fixtures from LoD 1.14d and at least one independent resource output.
    pub fn generate(
        &self,
        request: MapGenerationRequest,
    ) -> Result<GeneratedMap, NativeMapGenerationError> {
        Err(NativeMapGenerationError::UnsupportedArea {
            profile: self.profile,
            area: request.area(),
        })
    }
}

impl Default for NativeMapGenerator {
    fn default() -> Self {
        Self::lod_1_14d()
    }
}

/// Error returned by the native Rust map generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMapGenerationError {
    /// The selected area family has not been ported for this profile yet.
    UnsupportedArea {
        profile: MapGeneratorProfile,
        area: Area,
    },
    /// Required static game data is not loaded or not yet decoded.
    MissingStaticData {
        profile: MapGeneratorProfile,
        table: &'static str,
    },
    /// Native output disagreed with a fixture or compatibility guard.
    IncompatibleOutput {
        profile: MapGeneratorProfile,
        reason: String,
    },
}

impl fmt::Display for NativeMapGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArea { profile, area } => write!(
                formatter,
                "native {} map generation does not support area {} yet",
                profile, area
            ),
            Self::MissingStaticData { profile, table } => write!(
                formatter,
                "native {} map generation requires static table {}",
                profile, table
            ),
            Self::IncompatibleOutput { profile, reason } => {
                write!(
                    formatter,
                    "native {} map generation produced incompatible output: {}",
                    profile, reason
                )
            }
        }
    }
}

impl std::error::Error for NativeMapGenerationError {}

/// Diablo II's map-generation random stream.
///
/// The legacy engine uses a 32-bit multiply-with-carry generator in several
/// gameplay systems, including DRLG map layout. The seed half is multiplied by
/// `0x6AC690C5`, the carry half is added, the low 32 bits become the next seed,
/// and the high 32 bits become the next carry. Public reverse-engineering
/// notes and D2BS offsets refer to this helper as `D2GAME_Rand`/`D2Rand`.
///
/// This type is intentionally tiny: area generators should own their local
/// stream state explicitly instead of hiding seed advancement in global state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrlgSeed {
    seed: u32,
    carry: u32,
}

impl DrlgSeed {
    /// Initial carry used by the legacy game when a map seed is expanded.
    pub const INITIAL_CARRY: u32 = 666;

    const MULTIPLIER: u64 = 0x6AC6_90C5;

    /// Creates a DRLG random stream from the packet/save map seed.
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            carry: Self::INITIAL_CARRY,
        }
    }

    /// Creates a stream from an explicit seed/carry pair.
    ///
    /// This is mainly useful for fixtures that capture intermediate engine
    /// state. Normal map-generation code should start from [`DrlgSeed::new`].
    pub const fn from_parts(seed: u32, carry: u32) -> Self {
        Self { seed, carry }
    }

    /// Current low 32-bit seed value.
    pub const fn seed(self) -> u32 {
        self.seed
    }

    /// Current high 32-bit carry value.
    pub const fn carry(self) -> u32 {
        self.carry
    }

    /// Current packed state as `carry << 32 | seed`.
    pub const fn packed_state(self) -> u64 {
        ((self.carry as u64) << 32) | self.seed as u64
    }

    /// Advances the stream and returns the new seed half.
    pub fn next_u32(&mut self) -> u32 {
        let product = (self.seed as u64)
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.carry as u64);
        self.seed = product as u32;
        self.carry = (product >> 32) as u32;
        self.seed
    }

    /// Advances the stream and returns `next_u32() % upper_bound`.
    ///
    /// The game helper is used with positive, usually small, bounds. A bound of
    /// zero returns zero so callers can mirror the defensive legacy behavior
    /// without panicking.
    pub fn next_bounded(&mut self, upper_bound: u32) -> u32 {
        if upper_bound == 0 {
            return 0;
        }

        self.next_u32() % upper_bound
    }

    /// Advances the stream `count` times.
    pub fn advance(&mut self, count: usize) {
        for _ in 0..count {
            self.next_u32();
        }
    }
}

/// Fixture metadata plus one normalized generated map.
///
/// Native map generation should be tested against captured/generated outputs
/// that state exactly which profile, seed, difficulty, act, and area produced
/// the map. This wrapper parses that metadata and then reuses
/// [`MapGenerationRequest`] normalization so fixture mistakes fail before a
/// generator comparison runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapGenerationFixture {
    profile: MapGeneratorProfile,
    source: String,
    request: MapGenerationRequest,
    map: GeneratedMap,
}

impl MapGenerationFixture {
    /// Parses a map-generation fixture JSON document.
    ///
    /// The expected shape is:
    ///
    /// ```text
    /// {
    ///   "profile": "lod_1_14d",
    ///   "source": "...",
    ///   "seed": 906454380,
    ///   "difficulty": 2,
    ///   "act": 1,
    ///   "area": 74,
    ///   "level": { ... generated-map level JSON ... }
    /// }
    /// ```
    pub fn from_json(input: &str) -> Result<Self, MapGenerationFixtureError> {
        let fixture: MapgenFixture = serde_json::from_str(input)?;
        let profile = MapGeneratorProfile::from_fixture_label(&fixture.profile)
            .ok_or_else(|| MapGenerationFixtureError::InvalidProfile(fixture.profile.clone()))?;
        let seed = u32::try_from(fixture.seed)
            .map_err(|_| MapGenerationFixtureError::InvalidSeed(fixture.seed))?;
        let difficulty_value = u8::try_from(fixture.difficulty)
            .map_err(|_| MapGenerationFixtureError::InvalidDifficulty(fixture.difficulty))?;
        let difficulty = Difficulty::from_packet_value(difficulty_value).ok_or(
            MapGenerationFixtureError::InvalidDifficulty(fixture.difficulty),
        )?;
        let area_id = u16::try_from(fixture.area)
            .map_err(|_| MapGenerationFixtureError::InvalidArea(fixture.area))?;
        let area =
            Area::from_id(area_id).ok_or(MapGenerationFixtureError::InvalidArea(fixture.area))?;
        let request = MapGenerationRequest::for_area(seed, difficulty, area)?;

        if fixture.act != request.generator_act() as i64 {
            return Err(MapGenerationFixtureError::Map(
                MapGenerationError::ResponseActMismatch {
                    expected: request.generator_act(),
                    actual: fixture.act,
                },
            ));
        }

        let map = GeneratedMap::try_from(fixture.level)?;
        let map = request.validate_generated_map(map)?;

        Ok(Self {
            profile,
            source: fixture.source,
            request,
            map,
        })
    }

    pub fn profile(&self) -> MapGeneratorProfile {
        self.profile
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn request(&self) -> MapGenerationRequest {
        self.request
    }

    pub fn map(&self) -> &GeneratedMap {
        &self.map
    }

    pub fn into_map(self) -> GeneratedMap {
        self.map
    }
}

/// Error returned while parsing generated-map fixture metadata.
#[derive(Debug)]
pub enum MapGenerationFixtureError {
    Json(serde_json::Error),
    InvalidProfile(String),
    InvalidSeed(u64),
    InvalidDifficulty(u64),
    InvalidArea(u64),
    Request(MapGenerationRequestError),
    Map(MapGenerationError),
}

impl fmt::Display for MapGenerationFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to parse map fixture JSON: {}", error),
            Self::InvalidProfile(profile) => {
                write!(formatter, "invalid map fixture profile {}", profile)
            }
            Self::InvalidSeed(seed) => write!(formatter, "invalid map fixture seed {}", seed),
            Self::InvalidDifficulty(difficulty) => {
                write!(formatter, "invalid map fixture difficulty {}", difficulty)
            }
            Self::InvalidArea(area) => write!(formatter, "invalid map fixture area {}", area),
            Self::Request(error) => write!(formatter, "invalid map fixture request: {}", error),
            Self::Map(error) => write!(formatter, "invalid map fixture output: {}", error),
        }
    }
}

impl std::error::Error for MapGenerationFixtureError {}

impl From<serde_json::Error> for MapGenerationFixtureError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<MapGenerationRequestError> for MapGenerationFixtureError {
    fn from(error: MapGenerationRequestError) -> Self {
        Self::Request(error)
    }
}

impl From<MapGenerationError> for MapGenerationFixtureError {
    fn from(error: MapGenerationError) -> Self {
        Self::Map(error)
    }
}

impl From<GeneratedMapJsonError> for MapGenerationFixtureError {
    fn from(error: GeneratedMapJsonError) -> Self {
        Self::Map(MapGenerationError::GeneratedMap(error))
    }
}

/// One decoded room template entry from a Tower Cellar levelgraph record.
///
/// emmericp's `diablo2-maps` exports Tower Cellar levels 1 through 4 as eight
/// compact room slots. The room encoding is not the full map; it is a
/// deterministic fixture-friendly summary of room cell, room id, variant, and
/// grave-mask metadata extracted from D2's initialized `Room2` graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TowerCellarRoom {
    room_id: u16,
    variant: u8,
    graves: u8,
}

impl TowerCellarRoom {
    /// Decodes emmericp's two-byte Tower Cellar room encoding.
    pub fn from_encoded(encoded: u16) -> Self {
        let low = (encoded & 0x00ff) as u8;
        let high = (encoded >> 8) as u8;
        Self {
            room_id: u16::from(high & 0x7f) + 100,
            variant: (high >> 7) & 1,
            graves: low,
        }
    }

    /// Encodes the room metadata back into emmericp's two-byte representation.
    pub fn encode(self) -> u16 {
        let high = ((self.variant & 1) << 7) | ((self.room_id - 100) as u8 & 0x7f);
        u16::from(self.graves) | (u16::from(high) << 8)
    }

    pub fn room_id(self) -> u16 {
        self.room_id
    }

    pub fn variant(self) -> u8 {
        self.variant
    }

    pub fn graves(self) -> u8 {
        self.graves
    }

    /// Returns true for room ids present in emmericp's Tower Cellar table.
    pub fn is_known_room_id(room_id: u16) -> bool {
        matches!(
            room_id,
            109 | 110
                | 111
                | 112
                | 113
                | 114
                | 115
                | 116
                | 117
                | 118
                | 119
                | 120
                | 121
                | 122
                | 123
                | 124
                | 125
                | 126
                | 127
                | 128
                | 129
                | 130
                | 131
                | 132
                | 133
                | 134
                | 135
                | 136
                | 137
                | 139
                | 140
                | 141
                | 142
                | 143
                | 144
                | 145
                | 146
        )
    }
}

/// One occupied cell in a Tower Cellar levelgraph record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TowerCellarRoomSlot {
    cell_x: u8,
    cell_y: u8,
    room: TowerCellarRoom,
}

impl TowerCellarRoomSlot {
    /// Builds a room slot for a 4-bit `(cell_x, cell_y)` graph position.
    pub const fn new(cell_x: u8, cell_y: u8, room: TowerCellarRoom) -> Option<Self> {
        if cell_x < 16 && cell_y < 16 {
            Some(Self {
                cell_x,
                cell_y,
                room,
            })
        } else {
            None
        }
    }

    pub fn cell_x(self) -> u8 {
        self.cell_x
    }

    pub fn cell_y(self) -> u8 {
        self.cell_y
    }

    pub fn room(self) -> TowerCellarRoom {
        self.room
    }

    fn position_byte(self) -> u8 {
        (self.cell_y << 4) | self.cell_x
    }
}

/// Decoded Tower Cellar levelgraph record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TowerCellarLevelGraph {
    rooms: Vec<TowerCellarRoomSlot>,
}

impl TowerCellarLevelGraph {
    pub const MAX_ROOMS: usize = 8;
    pub const RECORD_SIZE: usize = Self::MAX_ROOMS * 3;

    /// Decodes one fixed-size Tower Cellar levelgraph record.
    ///
    /// Each record has eight 3-byte slots. Occupied slots store
    /// `(cell_y << 4) | cell_x` followed by a little-endian
    /// [`TowerCellarRoom`] encoding. Unused slots are exactly `FF FF FF`.
    pub fn from_levelgraph_record(record: &[u8]) -> Result<Self, TowerCellarLevelGraphError> {
        if record.len() != Self::RECORD_SIZE {
            return Err(TowerCellarLevelGraphError::InvalidRecordLength {
                expected: Self::RECORD_SIZE,
                actual: record.len(),
            });
        }

        let mut rooms = Vec::new();
        for (slot, bytes) in record.chunks_exact(3).enumerate() {
            let position = bytes[0];
            let encoded = u16::from_le_bytes([bytes[1], bytes[2]]);

            if position == 0xff || encoded == 0xffff {
                if position == 0xff && encoded == 0xffff {
                    continue;
                }

                return Err(TowerCellarLevelGraphError::MalformedSlot { slot });
            }

            let room = TowerCellarRoom::from_encoded(encoded);
            if !TowerCellarRoom::is_known_room_id(room.room_id()) {
                return Err(TowerCellarLevelGraphError::UnknownRoomId {
                    slot,
                    room_id: room.room_id(),
                });
            }

            let cell_x = position & 0x0f;
            let cell_y = position >> 4;
            let room_slot = TowerCellarRoomSlot::new(cell_x, cell_y, room)
                .expect("nibble-split cell coordinates are always below 16");
            rooms.push(room_slot);
        }

        Ok(Self { rooms })
    }

    pub fn rooms(&self) -> &[TowerCellarRoomSlot] {
        &self.rooms
    }

    /// Encodes the graph back into the fixed 24-byte fixture record.
    pub fn to_levelgraph_record(&self) -> [u8; Self::RECORD_SIZE] {
        let mut record = [0xff; Self::RECORD_SIZE];
        for (slot, room) in self.rooms.iter().take(Self::MAX_ROOMS).enumerate() {
            let offset = slot * 3;
            let encoded = room.room().encode().to_le_bytes();
            record[offset] = room.position_byte();
            record[offset + 1] = encoded[0];
            record[offset + 2] = encoded[1];
        }
        record
    }
}

/// Error returned while decoding Tower Cellar levelgraph fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerCellarLevelGraphError {
    InvalidRecordLength { expected: usize, actual: usize },
    MalformedSlot { slot: usize },
    UnknownRoomId { slot: usize, room_id: u16 },
}

impl fmt::Display for TowerCellarLevelGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRecordLength { expected, actual } => write!(
                formatter,
                "Tower Cellar levelgraph record has {} bytes, expected {}",
                actual, expected
            ),
            Self::MalformedSlot { slot } => {
                write!(
                    formatter,
                    "Tower Cellar levelgraph slot {} is malformed",
                    slot
                )
            }
            Self::UnknownRoomId { slot, room_id } => write!(
                formatter,
                "Tower Cellar levelgraph slot {} uses unknown room id {}",
                slot, room_id
            ),
        }
    }
}

impl std::error::Error for TowerCellarLevelGraphError {}

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
struct MapgenFixture {
    profile: String,
    source: String,
    seed: u64,
    difficulty: u64,
    act: i64,
    area: u64,
    level: MapgenLevel,
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

/// Returns true for the Tower Cellar levels covered by the levelgraph fixture
/// format ported from emmericp's tooling.
///
/// The compact levelgraph records currently describe Tower Cellar levels 1
/// through 4. Tower Cellar Level 5 has special Countess/end-room behavior and
/// should get its own fixture contract when that area is ported.
pub fn is_tower_cellar_levelgraph_area(area: Area) -> bool {
    matches!(
        area,
        Area::TowerCellarLevel1
            | Area::TowerCellarLevel2
            | Area::TowerCellarLevel3
            | Area::TowerCellarLevel4
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::core::act::Act;
    use crate::core::area::Area;
    use crate::core::game_state::Difficulty;

    use super::{
        act_from_level_id, expand_rle_row, is_good_exit, is_tower_cellar_levelgraph_area,
        is_valid_map_seed, rle_row_is_blocked, CollisionGrid, DrlgSeed, GeneratedMap,
        GeneratedMapJsonError, MapGenerationError, MapGenerationFixture, MapGenerationRequest,
        MapGenerationRequestError, MapGeneratorProfile, MapObjectKind, MapSeed,
        NativeMapGenerationError, NativeMapGenerator, TowerCellarLevelGraph,
        TowerCellarLevelGraphError, TowerCellarRoom,
    };

    #[test]
    fn validates_unsigned_map_seed_range() {
        assert!(MapSeed::new(1).is_some());
        assert!(is_valid_map_seed(0x3607_656c));
        assert!(!is_valid_map_seed(0));
        assert!(!is_valid_map_seed(u32::MAX));
    }

    #[test]
    fn drlg_seed_matches_lod_multiply_with_carry_vectors() {
        let mut seed = DrlgSeed::new(0x1234_5678);

        assert_eq!(seed.carry(), DrlgSeed::INITIAL_CARRY);
        assert_eq!(seed.next_u32(), 0x03ba_0cf2);
        assert_eq!(seed.carry(), 0x0797_ca94);
        assert_eq!(seed.next_u32(), 0xc437_e0ce);
        assert_eq!(seed.carry(), 0x018d_ed5d);
        assert_eq!(seed.next_u32(), 0x9a55_cbe3);
        assert_eq!(seed.carry(), 0x51d7_5543);
    }

    #[test]
    fn drlg_seed_bounded_values_advance_like_legacy_helper() {
        let mut seed = DrlgSeed::new(0x1234_5678);

        assert_eq!(seed.next_bounded(100), 58);
        assert_eq!(seed.next_bounded(100), 66);
        assert_eq!(seed.next_bounded(100), 19);
        assert_eq!(seed.next_bounded(0), 0);
        assert_eq!(seed.next_bounded(100), 54);
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
    fn native_generator_defaults_to_lod_1_14d_profile() {
        let generator = NativeMapGenerator::default();

        assert_eq!(generator.profile(), MapGeneratorProfile::Lod114d);
        assert_eq!(generator.profile().patch_label(), "LoD 1.14d");
        assert_eq!(generator.profile().fixture_label(), "lod_1_14d");
        assert_eq!(
            MapGeneratorProfile::from_fixture_label("lod_1_14d"),
            Some(MapGeneratorProfile::Lod114d)
        );
    }

    #[test]
    fn native_generator_reports_unsupported_area_until_ported() {
        let request =
            MapGenerationRequest::for_area(0x3607_656c, Difficulty::Hell, Area::ArcaneSanctuary)
                .expect("area request should be valid");

        let error = NativeMapGenerator::lod_1_14d()
            .generate(request)
            .expect_err("area has not been ported yet");

        assert_eq!(
            error,
            NativeMapGenerationError::UnsupportedArea {
                profile: MapGeneratorProfile::Lod114d,
                area: Area::ArcaneSanctuary,
            }
        );
        assert_eq!(
            error.to_string(),
            "native LoD 1.14d map generation does not support area Arcane Sanctuary yet"
        );
    }

    #[test]
    fn map_generation_fixture_json_validates_profile_and_request_metadata() {
        let fixture = MapGenerationFixture::from_json(include_str!(
            "../../tests/fixtures/mapgen/lod_1_14d/arcane_sanctuary_0x3607656c_hell.json"
        ))
        .expect("fixture should parse");

        assert_eq!(fixture.profile(), MapGeneratorProfile::Lod114d);
        assert!(fixture.source().contains("libd2"));
        assert_eq!(fixture.request().generator_seed(), 0x3607_656c);
        assert_eq!(fixture.request().difficulty(), Difficulty::Hell);
        assert_eq!(fixture.request().area(), Area::ArcaneSanctuary);
        assert_eq!(fixture.map().id, Area::ArcaneSanctuary as u16);
        assert_eq!(fixture.map().objects.len(), 2);
        assert_eq!(fixture.map().is_blocked(1, 0), Some(false));
    }

    #[test]
    fn tower_cellar_levelgraph_fixture_decodes_room_slots() {
        let fixture = TowerLevelGraphFixture::from_json(include_str!(
            "../../tests/fixtures/mapgen/lod_1_14d/tower_cellar_levelgraph_record.json"
        ));
        let graph = TowerCellarLevelGraph::from_levelgraph_record(&fixture.record)
            .expect("levelgraph record should decode");

        assert_eq!(fixture.profile, "lod_1_14d");
        assert!(fixture.source.contains("levelgraph"));
        assert_eq!(fixture.seed, 0x3607_656c);
        assert_eq!(fixture.difficulty, 0);
        assert_eq!(fixture.act, 0);
        assert_eq!(fixture.area, Area::TowerCellarLevel1 as u16);
        assert!(is_tower_cellar_levelgraph_area(
            Area::from_id(fixture.area).expect("fixture area should exist")
        ));
        assert_eq!(graph.rooms().len(), 3);

        let first = graph.rooms()[0];
        assert_eq!((first.cell_x(), first.cell_y()), (0, 0));
        assert_eq!(first.room().room_id(), 109);
        assert_eq!(first.room().variant(), 0);
        assert_eq!(first.room().graves(), 0x15);

        let second = graph.rooms()[1];
        assert_eq!((second.cell_x(), second.cell_y()), (1, 1));
        assert_eq!(second.room().room_id(), 111);
        assert_eq!(second.room().variant(), 1);
        assert_eq!(second.room().graves(), 0x0a);

        assert_eq!(graph.to_levelgraph_record().as_slice(), fixture.record);
    }

    #[test]
    fn tower_cellar_levelgraph_rejects_malformed_slots_and_unknown_rooms() {
        let malformed = [0xff, 0x00, 0x27];
        assert_eq!(
            TowerCellarLevelGraph::from_levelgraph_record(&malformed),
            Err(TowerCellarLevelGraphError::InvalidRecordLength {
                expected: TowerCellarLevelGraph::RECORD_SIZE,
                actual: malformed.len(),
            })
        );

        let mut record = [0xff; TowerCellarLevelGraph::RECORD_SIZE];
        record[0] = 0xff;
        record[1] = 0x00;
        record[2] = 0x27;
        assert_eq!(
            TowerCellarLevelGraph::from_levelgraph_record(&record),
            Err(TowerCellarLevelGraphError::MalformedSlot { slot: 0 })
        );

        let unknown_room = TowerCellarRoom::from_encoded(0x0100);
        record[0] = 0x00;
        record[1..3].copy_from_slice(&unknown_room.encode().to_le_bytes());
        assert_eq!(
            TowerCellarLevelGraph::from_levelgraph_record(&record),
            Err(TowerCellarLevelGraphError::UnknownRoomId {
                slot: 0,
                room_id: 101,
            })
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
    fn classifies_tower_cellar_levelgraph_area_family() {
        assert!(is_tower_cellar_levelgraph_area(Area::TowerCellarLevel1));
        assert!(is_tower_cellar_levelgraph_area(Area::TowerCellarLevel4));
        assert!(!is_tower_cellar_levelgraph_area(Area::TowerCellarLevel5));
        assert!(!is_tower_cellar_levelgraph_area(Area::ForgottenTower));
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

    #[derive(Debug, Deserialize)]
    struct TowerLevelGraphFixture {
        profile: String,
        source: String,
        seed: u32,
        difficulty: u8,
        act: u8,
        area: u16,
        #[serde(rename = "record_hex", deserialize_with = "hex_bytes")]
        record: Vec<u8>,
    }

    impl TowerLevelGraphFixture {
        fn from_json(input: &str) -> Self {
            serde_json::from_str(input).expect("tower levelgraph fixture JSON should parse")
        }
    }

    fn hex_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        decode_hex_bytes(&value).map_err(serde::de::Error::custom)
    }

    fn decode_hex_bytes(input: &str) -> Result<Vec<u8>, String> {
        let mut compact = String::new();
        for character in input.chars() {
            if character.is_ascii_hexdigit() {
                compact.push(character);
            }
        }

        if compact.len() % 2 != 0 {
            return Err("hex fixture has an odd number of digits".to_string());
        }

        compact
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text =
                    std::str::from_utf8(pair).map_err(|error| format!("invalid UTF-8: {error}"))?;
                u8::from_str_radix(text, 16).map_err(|error| format!("invalid hex byte: {error}"))
            })
            .collect()
    }
}
