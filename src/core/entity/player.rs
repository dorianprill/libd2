// Player struct

use std::collections::HashMap;

use crate::core::character_class::CharacterClass;
use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::unit_stat::UnitStat;
use crate::core::update::Update;
use crate::ServerMessage;

/// Current local-player resource values decoded from D2GS HP/MP packets.
///
/// Diablo II does not send these live values as ordinary `UnitStat` updates.
/// Server packets `0x18` and `0x95` use a compact bitstream with 15-bit life,
/// mana, and stamina fields; `0x96` refreshes stamina without life/mana; `0x18`
/// additionally carries 7-bit regeneration counters. The values are kept as raw
/// packet units here. Converting them to UI percentages requires the
/// corresponding max-life/max-mana/max-stamina stats, which may arrive through
/// different packets or save/static data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerVitals {
    life: Option<u16>,
    mana: Option<u16>,
    stamina: Option<u16>,
    life_regen: Option<u8>,
    mana_regen: Option<u8>,
}

impl PlayerVitals {
    pub fn new(life: u16, mana: u16, stamina: u16) -> Self {
        Self {
            life: Some(life),
            mana: Some(mana),
            stamina: Some(stamina),
            life_regen: None,
            mana_regen: None,
        }
    }

    pub fn stamina_only(stamina: u16) -> Self {
        Self {
            life: None,
            mana: None,
            stamina: Some(stamina),
            life_regen: None,
            mana_regen: None,
        }
    }

    pub fn with_regen(mut self, life_regen: u8, mana_regen: u8) -> Self {
        self.life_regen = Some(life_regen);
        self.mana_regen = Some(mana_regen);
        self
    }

    pub fn life(&self) -> Option<u16> {
        self.life
    }

    pub fn mana(&self) -> Option<u16> {
        self.mana
    }

    pub fn stamina(&self) -> Option<u16> {
        self.stamina
    }

    pub fn life_regen(&self) -> Option<u8> {
        self.life_regen
    }

    pub fn mana_regen(&self) -> Option<u8> {
        self.mana_regen
    }
}

/// Last local-player movement verification decoded from D2GS HP/MP packets.
///
/// Packets `0x18`, `0x95`, and `0x96` all include the server's current player
/// coordinates. Packet tables also name two trailing 8-bit fields as `dX` and
/// `dY`; current public reverse-engineering sources disagree on whether these
/// should be interpreted as signed deltas, raw offsets, or mostly unknown
/// verification bits. The library therefore exposes them as raw bytes while
/// still using the decoded coordinate to keep the local player centered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerMovement {
    location: Coordinate,
    dx: u8,
    dy: u8,
}

impl PlayerMovement {
    pub fn new(location: Coordinate, dx: u8, dy: u8) -> Self {
        Self { location, dx, dy }
    }

    pub fn location(&self) -> Coordinate {
        self.location
    }

    pub fn dx(&self) -> u8 {
        self.dx
    }

    pub fn dy(&self) -> u8 {
        self.dy
    }
}

/// Base skill levels known for one player.
///
/// D2GS packet `0x94` uses global `Skills.txt` ids, while legacy `.d2s` files
/// store exactly 30 bytes in the `if` section for the character's own class.
/// The class-local save slots are simple offsets for vanilla Diablo II:
/// Amazon `6`, Sorceress `36`, Necromancer `66`, Paladin `96`, Barbarian
/// `126`, Druid `221`, and Assassin `251`. Off-class skills can still appear
/// in live traffic through charges or granted skills, so this type preserves
/// the raw ids and filters only when a save table is requested.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSkillLevels {
    by_skill_id: HashMap<u16, u8>,
}

impl PlayerSkillLevels {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, skill_id: u16, level: u8) {
        self.by_skill_id.insert(skill_id, level);
    }

    pub fn get(&self, skill_id: u16) -> Option<u8> {
        self.by_skill_id.get(&skill_id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u16, u8)> + '_ {
        self.by_skill_id
            .iter()
            .map(|(skill_id, level)| (*skill_id, *level))
    }

    /// Builds the 30-byte legacy `.d2s` class-skill table for `class`.
    ///
    /// Save files do not include global skill ids in the `if` section. They
    /// rely on the character class to choose the fixed `Skills.txt` id offset,
    /// then write one byte per class skill. Unknown or off-class ids are
    /// intentionally ignored here rather than projected into the wrong slot.
    pub fn to_legacy_save_table(&self, class: CharacterClass) -> [u8; 30] {
        let mut table = [0; 30];
        for (skill_id, level) in self.iter() {
            if let Some(slot) = Self::legacy_save_slot(class, skill_id) {
                table[slot] = level;
            }
        }
        table
    }

    pub fn legacy_save_slot(class: CharacterClass, skill_id: u16) -> Option<usize> {
        let offset = legacy_skill_offset(class)?;
        let slot = skill_id.checked_sub(offset)?;
        (slot < 30).then_some(slot as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    class: CharacterClass,
    name: String,
    id: u32,
    location: Coordinate,
    has_mercenary: bool,
    directory_known: bool,
    mercenary_id: u32,
    level: u32,
    portal_id: u32,
    stats: HashMap<u16, u32>,
    skills: PlayerSkillLevels,
    vitals: Option<PlayerVitals>,
    movement: Option<PlayerMovement>,
    world_location_known: bool,
    // TODO
    // stash:       Container;
    // cube:        Container;
    // belt:        Belt;
}

impl Player {
    pub fn new(
        id: u32,
        class: CharacterClass,
        name: impl Into<String>,
        location: Coordinate,
    ) -> Self {
        Self {
            class,
            name: name.into(),
            id,
            location,
            has_mercenary: false,
            directory_known: false,
            mercenary_id: 0,
            level: 0,
            portal_id: 0,
            stats: HashMap::new(),
            skills: PlayerSkillLevels::new(),
            vitals: None,
            movement: None,
            world_location_known: true,
        }
    }

    /// Creates a roster-only player entry without a current world position.
    ///
    /// Legacy D2GS distinguishes the game roster (`0x5B PlayerJoined`) from
    /// in-world unit assignment (`0x59 AssignPlayer`). A player can stay in the
    /// game after their unit is removed from the local client's visible area, so
    /// roster entries must be representable without a map marker.
    pub fn new_roster(id: u32, class: CharacterClass, name: impl Into<String>) -> Self {
        Self {
            world_location_known: false,
            ..Self::new(id, class, name, Coordinate::new(0, 0))
        }
    }

    pub fn class(&self) -> CharacterClass {
        self.class
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_class(&mut self, class: CharacterClass) {
        self.class = class;
    }

    pub fn set_location(&mut self, location: Coordinate) {
        self.location = location;
        self.world_location_known = true;
    }

    /// Returns whether the current `location` should be rendered as an in-world
    /// marker.
    ///
    /// `0x0A RemoveObject` with unit type `0` can mean that a remote player unit
    /// left the local visible area, not that the player left the game. In that
    /// case the player remains in the roster but this flag is cleared until a
    /// later assignment, movement, or party automap packet refreshes a position.
    pub fn world_location_known(&self) -> bool {
        self.world_location_known
    }

    pub fn clear_world_location(&mut self) {
        self.world_location_known = false;
    }

    pub fn vitals(&self) -> Option<PlayerVitals> {
        self.vitals
    }

    pub fn set_vitals(&mut self, vitals: PlayerVitals) {
        self.vitals = Some(vitals);
    }

    pub fn set_stamina(&mut self, stamina: u16) {
        self.vitals = Some(match self.vitals {
            Some(mut vitals) => {
                vitals.stamina = Some(stamina);
                vitals
            }
            None => PlayerVitals::stamina_only(stamina),
        });
    }

    pub fn movement(&self) -> Option<PlayerMovement> {
        self.movement
    }

    pub fn set_movement(&mut self, movement: PlayerMovement) {
        self.location = movement.location();
        self.movement = Some(movement);
        self.world_location_known = true;
    }

    pub fn has_mercenary(&self) -> bool {
        self.has_mercenary
    }

    pub fn mercenary_id(&self) -> u32 {
        self.mercenary_id
    }

    pub fn mercenary_id_set(&mut self, merc_id: u32) {
        self.has_mercenary = true;
        self.mercenary_id = merc_id;
    }

    pub fn directory_known(&self) -> bool {
        self.directory_known
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn set_level(&mut self, lvl: u32) -> u32 {
        self.level = lvl;
        self.level
    }

    pub fn portal_id(&self) -> u32 {
        self.portal_id
    }

    pub fn set_portal_id(&mut self, portal_id: u32) -> u32 {
        self.portal_id = portal_id;
        portal_id
    }

    pub fn stat(&self, stat_id: u16) -> Option<u32> {
        self.stats.get(&stat_id).copied()
    }

    pub fn skills(&self) -> &PlayerSkillLevels {
        &self.skills
    }

    pub fn skills_mut(&mut self) -> &mut PlayerSkillLevels {
        &mut self.skills
    }

    pub fn set_skill_level(&mut self, skill_id: u16, level: u8) {
        self.skills.set(skill_id, level);
    }

    pub fn legacy_save_skills(&self) -> [u8; 30] {
        self.skills.to_legacy_save_table(self.class)
    }

    pub fn set_stat(&mut self, stat_id: u16, value: u32) {
        self.stats.insert(stat_id, value);
        if stat_id == UnitStat::Level as u16 {
            self.level = value;
        }
    }

    pub fn add_stat(&mut self, stat_id: u16, value: u32) {
        let current = self.stat(stat_id).unwrap_or_default();
        self.set_stat(stat_id, current.saturating_add(value));
    }
}

fn legacy_skill_offset(class: CharacterClass) -> Option<u16> {
    match class {
        CharacterClass::Amazon => Some(6),
        CharacterClass::Sorceress => Some(36),
        CharacterClass::Necromancer => Some(66),
        CharacterClass::Paladin => Some(96),
        CharacterClass::Barbarian => Some(126),
        CharacterClass::Druid => Some(221),
        CharacterClass::Assassin => Some(251),
        CharacterClass::Warlock => None,
    }
}

impl Entity for Player {
    fn initialized(&self) -> bool {
        return true;
    }

    fn id(&self) -> u32 {
        return self.id;
    }

    fn location(&self) -> Coordinate {
        return self.location;
    }
}

impl Update for Player {
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        return true;
    }
}

pub enum PlayerItemSlot {
    Helm = 0x01,
    Amulet = 0x02,
    Armor = 0x03,
    LeftWeapon1 = 0x04,
    RightWeapon1 = 0x05,
    LeftRing = 0x06,
    RightRing = 0x07,
    Belt = 0x08,
    Boots = 0x09,
    Gloves = 0x0A,
    LeftWeapon2 = 0x0B,
    RightWeapon2 = 0x0C,
}

pub enum EmotePhrase {
    Help = 0x19,
    Follow = 0x1A,
    Gift = 0x1B,
    Thanks = 0x1C,
    Sorry = 0x1D,
    Bye = 0x1E,
    Die = 0x1F,
    Flee = 0x20,
}

// taken from https://bnetdocs.org/packet/98/d2gs-trade
//Press Accept button (unaccept) should be sent when placing items in the trade window as well
pub enum TradeAction {
    CancelTradeRequest = 0x02,
    AcceptTradeRequest = 0x03,
    PressAcceptButton = 0x04,
    UnpressAcceptButton = 0x07,
    RefreshWindow = 0x08,
    CloseStash = 0x12,
    WithdrawGold = 0x13,
    DepositGold = 0x14,
    CloseHoradricCube = 0x17,
}
