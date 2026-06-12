use std::collections::{HashMap, HashSet};

use crate::ServerMessage;
use crate::core::character_class::CharacterClass;
use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::entity::mercenary::Mercenary;
use crate::core::entity::missile::Missile;
use crate::core::entity::npc::Npc;
use crate::core::entity::player::{
    PartyAffiliation, PartyLifeFraction, Player, PlayerMovement, PlayerVitals, RemotePartyInfo,
};
use crate::core::network::d2gs::D2GSPacket;
use crate::core::object::WorldObject;
use crate::core::object::item::{Item, ItemStateFlags};
use crate::core::protocol::server_message::{ServerMessageParseError, SkillDescription};
use crate::core::quest::PlayerQuestLog;
use crate::core::unit_stat::UnitStat;
use crate::core::update::Update;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameServerType {
    OpenBattleNet = 1,
    TCPIP = 2,
    SinglePlayer = 3,
}

/// GameMode Flags as sent by GameList server message
#[repr(u32)]
pub enum GameMode {
    Ladder = 0x00200000,
    Expansion = 0x00100000,
    Hardcore = 0x00000800,
}

/// Difficulty flags as sent by GameList server message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Difficulty {
    Normal = 0x0000,
    Nightmare = 0x1000,
    Hell = 0x2000,
}

impl Difficulty {
    pub fn from_packet_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::Nightmare),
            2 => Some(Self::Hell),
            _ => None,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    enUS = 0,
    esES = 1,
    deDE = 2,
    frFR = 3,
    ptPT = 4,
    itIT = 5,
    ja = 6,
    ko = 7,
    si = 8,
    zhCN = 9,
    pl = 10,
    ru = 11,
    enGB = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapTile {
    pub x: u16,
    pub y: u16,
    pub area_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameMapState {
    pub act: Option<u8>,
    pub map_id: Option<u32>,
    pub area_id: Option<u16>,
    pub automap: Option<u32>,
    pub revealed_tiles: HashSet<MapTile>,
}

/// Raw server item-stat update captured from D2GS packet `0x3E`.
///
/// Public packet tables for legacy Diablo II expose `0x3E` as a declared-size
/// stat bitstream, with 1.14d adding fixed padding out to 34 bytes. They do not
/// expose a stable item GUID in the packet envelope. Until the item-stat
/// bitstream itself is decoded with `ItemStatCost` metadata, the library keeps
/// these events in arrival order instead of guessing which [`Item`] owns them.
///
/// This is intentionally a raw-format exception inside `GameState`; decoded
/// player, mercenary, and world state should otherwise store semantic game
/// values rather than wire or save-file encodings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStatUpdate {
    packet_size: u8,
    bitstream: Vec<u8>,
}

impl ItemStatUpdate {
    pub fn new(packet_size: u8, bitstream: Vec<u8>) -> Self {
        Self {
            packet_size,
            bitstream,
        }
    }

    pub fn packet_size(&self) -> u8 {
        self.packet_size
    }

    pub fn bitstream(&self) -> &[u8] {
        &self.bitstream
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitKey {
    unit_type: u8,
    unit_id: u32,
}

impl UnitKey {
    pub fn new(unit_type: u8, unit_id: u32) -> Self {
        Self { unit_type, unit_id }
    }

    pub fn unit_type(&self) -> u8 {
        self.unit_type
    }

    pub fn unit_id(&self) -> u32 {
        self.unit_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCorpse {
    owner_id: u32,
    corpse_id: u32,
}

impl PlayerCorpse {
    pub fn new(owner_id: u32, corpse_id: u32) -> Self {
        Self {
            owner_id,
            corpse_id,
        }
    }

    pub fn owner_id(&self) -> u32 {
        self.owner_id
    }

    pub fn corpse_id(&self) -> u32 {
        self.corpse_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitStateSet {
    explicit_states: HashMap<u8, Vec<u8>>,
    multi_state_effects: Option<Vec<u8>>,
}

impl UnitStateSet {
    pub fn state_effects(&self, state: u8) -> Option<&[u8]> {
        self.explicit_states.get(&state).map(Vec::as_slice)
    }

    pub fn explicit_states(&self) -> &HashMap<u8, Vec<u8>> {
        &self.explicit_states
    }

    pub fn multi_state_effects(&self) -> Option<&[u8]> {
        self.multi_state_effects.as_deref()
    }

    fn set_state(&mut self, state: u8, state_effects: Vec<u8>) {
        self.explicit_states.insert(state, state_effects);
    }

    fn clear_state(&mut self, state: u8) -> bool {
        self.explicit_states.remove(&state).is_some()
    }

    fn set_multi_state_effects(&mut self, state_effects: Vec<u8>) {
        self.multi_state_effects = Some(state_effects);
    }

    fn is_empty(&self) -> bool {
        self.explicit_states.is_empty() && self.multi_state_effects.is_none()
    }
}

#[derive(Debug)]
pub struct GameState {
    pub(crate) players: HashMap<u32, Player>,
    /// Packet id aliases for the same character.
    ///
    /// Legacy D2GS traffic can describe players through roster packets and
    /// through in-world unit assignment packets. Public packet tables name both
    /// values as player/unit GUIDs, but live captures can expose them through
    /// different packet paths before the full identity has converged. Keeping a
    /// small alias table lets `0x5C PlayerLeft`, `0x0A RemoveObject`, party-map
    /// pulses, and movement/stat updates all hit the same canonical [`Player`].
    pub(crate) player_aliases: HashMap<u32, u32>,
    pub(crate) pending_player_stats: HashMap<u32, HashMap<u16, u32>>,
    pub(crate) pending_player_parties: HashMap<u32, PartyAffiliation>,
    pub(crate) pending_remote_party_info: HashMap<u32, RemotePartyInfo>,
    pub(crate) npcs: HashMap<u32, Npc>,
    pub(crate) mercenaries: HashMap<u32, Mercenary>,
    pub(crate) missiles: HashMap<u32, Missile>,
    pub(crate) objects: HashMap<u32, WorldObject>,
    pub(crate) player_corpses: HashMap<u32, PlayerCorpse>,
    pub(crate) unit_states: HashMap<UnitKey, UnitStateSet>,
    pub(crate) items: HashMap<u32, Item>,
    pub(crate) item_stat_updates: Vec<ItemStatUpdate>,
    pub(crate) game_type: GameServerType,
    pub(crate) difficulty: Difficulty,
    pub(crate) locale: Locale,
    pub(crate) map: GameMapState,
    pub(crate) local_player_id: Option<u32>,
    pub(crate) player_quest_log: Option<PlayerQuestLog>,
    pub(crate) is_expansion: bool,
    pub(crate) is_ladder: bool,
    pub(crate) is_hardcore: bool,
}

impl GameState {
    pub fn new(game_type: GameServerType, difficulty: Difficulty, locale: Locale) -> Self {
        Self {
            players: HashMap::with_capacity(8),
            player_aliases: HashMap::with_capacity(8),
            pending_player_stats: HashMap::with_capacity(8),
            pending_player_parties: HashMap::with_capacity(8),
            pending_remote_party_info: HashMap::with_capacity(8),
            npcs: HashMap::with_capacity(1024),
            mercenaries: HashMap::with_capacity(8),
            missiles: HashMap::with_capacity(256),
            objects: HashMap::with_capacity(256),
            player_corpses: HashMap::with_capacity(16),
            unit_states: HashMap::with_capacity(64),
            items: HashMap::with_capacity(256),
            item_stat_updates: Vec::with_capacity(64),
            game_type,
            difficulty,
            locale,
            map: GameMapState::default(),
            local_player_id: None,
            player_quest_log: None,
            is_expansion: false,
            is_ladder: false,
            is_hardcore: false,
        }
    }

    pub fn apply_packet(&mut self, packet: &D2GSPacket) -> Result<bool, ServerMessageParseError> {
        let message = ServerMessage::try_from(packet)?;
        Ok(self.update(message))
    }

    pub fn players(&self) -> &HashMap<u32, Player> {
        &self.players
    }

    pub fn player(&self, id: u32) -> Option<&Player> {
        self.players.get(&self.resolve_player_id(id))
    }

    pub fn npcs(&self) -> &HashMap<u32, Npc> {
        &self.npcs
    }

    pub fn mercenaries(&self) -> &HashMap<u32, Mercenary> {
        &self.mercenaries
    }

    pub fn mercenary(&self, id: u32) -> Option<&Mercenary> {
        self.mercenaries.get(&id)
    }

    pub fn missiles(&self) -> &HashMap<u32, Missile> {
        &self.missiles
    }

    pub fn missile(&self, id: u32) -> Option<&Missile> {
        self.missiles.get(&id)
    }

    pub fn npc(&self, id: u32) -> Option<&Npc> {
        self.npcs.get(&id)
    }

    pub fn objects(&self) -> &HashMap<u32, WorldObject> {
        &self.objects
    }

    pub fn object(&self, id: u32) -> Option<&WorldObject> {
        self.objects.get(&id)
    }

    pub fn items(&self) -> &HashMap<u32, Item> {
        &self.items
    }

    pub fn item(&self, id: u32) -> Option<&Item> {
        self.items.get(&id)
    }

    pub fn item_stat_updates(&self) -> &[ItemStatUpdate] {
        &self.item_stat_updates
    }

    pub fn player_corpses(&self) -> &HashMap<u32, PlayerCorpse> {
        &self.player_corpses
    }

    pub fn player_corpse(&self, corpse_id: u32) -> Option<&PlayerCorpse> {
        self.player_corpses.get(&corpse_id)
    }

    pub fn unit_states(&self) -> &HashMap<UnitKey, UnitStateSet> {
        &self.unit_states
    }

    pub fn unit_state(&self, unit_type: u8, unit_id: u32) -> Option<&UnitStateSet> {
        self.unit_states.get(&UnitKey::new(unit_type, unit_id))
    }

    pub fn map(&self) -> &GameMapState {
        &self.map
    }

    pub fn local_player_id(&self) -> Option<u32> {
        self.local_player_id.map(|id| self.resolve_player_id(id))
    }

    pub fn player_quest_log(&self) -> Option<&PlayerQuestLog> {
        self.player_quest_log.as_ref()
    }

    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub fn game_type(&self) -> GameServerType {
        self.game_type
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn is_expansion(&self) -> bool {
        self.is_expansion
    }

    pub fn is_ladder(&self) -> bool {
        self.is_ladder
    }

    pub fn is_hardcore(&self) -> bool {
        self.is_hardcore
    }

    fn local_player_mut(&mut self) -> Option<&mut Player> {
        let id = self.resolve_player_id(self.local_player_id?);
        self.players.get_mut(&id)
    }

    fn add_local_experience(&mut self, amount: u32) -> bool {
        let Some(player) = self.local_player_mut() else {
            return false;
        };
        player.add_stat(UnitStat::Experience as u16, amount);
        true
    }

    fn set_local_stat(&mut self, stat: u16, amount: u32) -> bool {
        let Some(player) = self.local_player_mut() else {
            return false;
        };
        player.set_stat(stat, game_state_stat_value(stat, amount));
        true
    }

    fn set_player_stat(&mut self, unit_id: u32, stat: u16, amount: u32) -> bool {
        let unit_id = self.resolve_player_id(unit_id);
        let amount = game_state_stat_value(stat, amount);
        if let Some(player) = self.players.get_mut(&unit_id) {
            player.set_stat(stat, amount);
        } else {
            self.pending_player_stats
                .entry(unit_id)
                .or_default()
                .insert(stat, amount);
        }
        true
    }

    fn apply_pending_player_stats(&mut self, unit_id: u32, canonical_id: u32) {
        let mut pending = self
            .pending_player_stats
            .remove(&unit_id)
            .unwrap_or_default();
        if unit_id != canonical_id {
            if let Some(canonical_pending) = self.pending_player_stats.remove(&canonical_id) {
                pending.extend(canonical_pending);
            }
        }
        if !pending.is_empty() {
            if let Some(player) = self.players.get_mut(&canonical_id) {
                for (stat, amount) in pending {
                    player.set_stat(stat, amount);
                }
            } else {
                self.pending_player_stats
                    .entry(canonical_id)
                    .or_default()
                    .extend(pending);
            }
        }

        let mut pending_party = self.pending_player_parties.remove(&unit_id);
        if unit_id != canonical_id {
            pending_party = self
                .pending_player_parties
                .remove(&canonical_id)
                .or(pending_party);
        }
        if let Some(party_affiliation) = pending_party {
            if let Some(player) = self.players.get_mut(&canonical_id) {
                player.set_party_affiliation(party_affiliation);
            } else {
                self.pending_player_parties
                    .insert(canonical_id, party_affiliation);
            }
        }

        let mut pending_remote_party_info = self.pending_remote_party_info.remove(&unit_id);
        if unit_id != canonical_id {
            pending_remote_party_info = self
                .pending_remote_party_info
                .remove(&canonical_id)
                .or(pending_remote_party_info);
        }
        if let Some(remote_party_info) = pending_remote_party_info {
            if let Some(player) = self.players.get_mut(&canonical_id) {
                player.set_remote_party_info(remote_party_info);
            } else {
                self.pending_remote_party_info
                    .insert(canonical_id, remote_party_info);
            }
        }
    }

    fn set_player_party_affiliation(
        &mut self,
        player_id: u32,
        party_affiliation: PartyAffiliation,
    ) -> bool {
        let player_id = self.resolve_player_id(player_id);
        if let Some(player) = self.players.get_mut(&player_id) {
            player.set_party_affiliation(party_affiliation);
        } else {
            self.pending_player_parties
                .insert(player_id, party_affiliation);
        }
        true
    }

    fn update_ally_party_info(
        &mut self,
        unit_type: u8,
        unit_id: u32,
        unit_life: u16,
        unit_area: u16,
    ) -> bool {
        match unit_type {
            0x00 => {
                let Some(life) = PartyLifeFraction::from_packet_value(unit_life) else {
                    return false;
                };
                let unit_id = self.resolve_player_id(unit_id);
                if let Some(player) = self.players.get_mut(&unit_id) {
                    player.set_remote_party_life(life);
                    player.set_remote_party_area_id(unit_area);
                } else {
                    let pending = self
                        .pending_remote_party_info
                        .entry(unit_id)
                        .or_insert_with(RemotePartyInfo::default);
                    pending.set_life(life);
                    pending.set_area_id(unit_area);
                }
                true
            }
            0x01 => {
                if unit_life > u8::MAX as u16 {
                    return false;
                }
                if let Some(mercenary) = self.mercenaries.get_mut(&unit_id) {
                    mercenary.set_life_percent(unit_life as u8);
                    true
                } else if let Some(npc) = self.npcs.get_mut(&unit_id) {
                    npc.set_life_percent(unit_life as u8);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn set_player_skills(&mut self, unit_id: u32, skills: Vec<SkillDescription>) -> bool {
        let unit_id = self.resolve_player_id(unit_id);
        let Some(player) = self.players.get_mut(&unit_id) else {
            return false;
        };
        for skill in skills {
            player.set_skill_level(skill.skill, skill.level);
        }
        true
    }

    fn set_local_vitals_and_movement(
        &mut self,
        vitals: PlayerVitals,
        movement: PlayerMovement,
    ) -> bool {
        let Some(player) = self.local_player_mut() else {
            return false;
        };
        player.set_vitals(vitals);
        player.set_movement(movement);
        true
    }

    fn set_local_stamina_and_movement(&mut self, stamina: u16, movement: PlayerMovement) -> bool {
        let Some(player) = self.local_player_mut() else {
            return false;
        };
        player.set_stamina(stamina);
        player.set_movement(movement);
        true
    }

    fn upsert_assigned_player(
        &mut self,
        unit_id: u32,
        class: CharacterClass,
        name: impl Into<String>,
        location: Coordinate,
    ) {
        let name = name.into();
        let canonical_id = self.resolve_player_id(unit_id);
        if let Some(player) = self.players.get_mut(&canonical_id) {
            player.set_class(class);
            player.set_name(name.clone());
            player.set_location(location);
            self.apply_pending_player_stats(unit_id, canonical_id);
            self.reattach_mercenaries_by_owner_identity(canonical_id, class, &name);
            return;
        }

        self.player_aliases.remove(&unit_id);

        if let Some(existing_id) = self.find_player_id_by_identity(class, &name) {
            self.link_player_alias(unit_id, existing_id);
            if let Some(player) = self.players.get_mut(&existing_id) {
                player.set_class(class);
                player.set_name(name.clone());
                player.set_location(location);
            }
            self.apply_pending_player_stats(unit_id, existing_id);
            self.reattach_mercenaries_by_owner_identity(existing_id, class, &name);
            return;
        }

        self.players
            .insert(unit_id, Player::new(unit_id, class, name.clone(), location));
        self.apply_pending_player_stats(unit_id, unit_id);
        self.reattach_mercenaries_by_owner_identity(unit_id, class, &name);
    }

    fn upsert_roster_player(
        &mut self,
        player_id: u32,
        class: CharacterClass,
        name: impl Into<String>,
        level: u32,
        party_affiliation: PartyAffiliation,
    ) {
        let name = name.into();
        let canonical_id = self.resolve_player_id(player_id);
        if let Some(player) = self.players.get_mut(&canonical_id) {
            player.set_class(class);
            player.set_name(name.clone());
            player.set_level(level);
            player.set_party_affiliation(party_affiliation);
            self.apply_pending_player_stats(player_id, canonical_id);
            self.reattach_mercenaries_by_owner_identity(canonical_id, class, &name);
            return;
        }

        self.player_aliases.remove(&player_id);

        if let Some(existing_id) = self.find_player_id_by_identity(class, &name) {
            self.link_player_alias(player_id, existing_id);
            if let Some(player) = self.players.get_mut(&existing_id) {
                player.set_class(class);
                player.set_name(name.clone());
                player.set_level(level);
                player.set_party_affiliation(party_affiliation);
            }
            self.apply_pending_player_stats(player_id, existing_id);
            self.reattach_mercenaries_by_owner_identity(existing_id, class, &name);
            return;
        }

        let mut player = Player::new_roster(player_id, class, name.clone());
        player.set_level(level);
        player.set_party_affiliation(party_affiliation);
        self.players.insert(player_id, player);
        self.apply_pending_player_stats(player_id, player_id);
        self.reattach_mercenaries_by_owner_identity(player_id, class, &name);
    }

    fn move_player(&mut self, unit_id: u32, location: Coordinate) -> bool {
        let unit_id = self.resolve_player_id(unit_id);
        let Some(player) = self.players.get_mut(&unit_id) else {
            return false;
        };
        player.set_location(location);
        true
    }

    fn move_or_create_npc(&mut self, unit_id: u32, location: Coordinate, life: Option<u8>) {
        if let Some(mercenary) = self.mercenaries.get_mut(&unit_id) {
            mercenary.set_location(location);
            if let Some(life) = life {
                mercenary.set_life_percent(life);
            }
            return;
        }

        if life == Some(0) {
            self.npcs.remove(&unit_id);
            return;
        }

        self.npcs
            .entry(unit_id)
            .and_modify(|npc| {
                npc.set_location(location);
                if let Some(life) = life {
                    npc.set_life_percent(life);
                }
            })
            .or_insert_with(|| {
                let mut npc = Npc::new(unit_id, location);
                if let Some(life) = life {
                    npc.set_life_percent(life);
                }
                npc
            });
    }

    fn set_unit_location(&mut self, unit_type: u8, unit_id: u32, location: Coordinate) -> bool {
        match unit_type {
            0x00 => self.move_player(unit_id, location),
            0x01 => {
                self.move_or_create_npc(unit_id, location, None);
                true
            }
            0x02 | 0x05 => {
                let Some(object) = self.objects.get_mut(&unit_id) else {
                    return false;
                };
                object.set_location(location);
                true
            }
            0x03 => {
                self.missiles
                    .entry(unit_id)
                    .and_modify(|missile| missile.set_location(location))
                    .or_insert_with(|| Missile::new(unit_id, location));
                true
            }
            _ => false,
        }
    }

    fn unit_location(&self, unit_type: u8, unit_id: u32) -> Option<Coordinate> {
        match unit_type {
            0x00 => {
                let player = self.player(unit_id)?;
                player.world_location_known().then_some(player.location())
            }
            0x01 => self
                .mercenaries
                .get(&unit_id)
                .and_then(|mercenary| {
                    mercenary
                        .world_location_known()
                        .then_some(mercenary.location())
                })
                .or_else(|| self.npcs.get(&unit_id).map(Entity::location)),
            0x02 | 0x05 => self.objects.get(&unit_id).map(WorldObject::location),
            0x03 => self.missiles.get(&unit_id).map(Entity::location),
            _ => None,
        }
    }

    fn upsert_missile(&mut self, missile: Missile) {
        self.missiles
            .entry(missile.id())
            .and_modify(|existing| {
                existing.class_id = missile.class_id;
                existing.location = missile.location;
                existing.target = missile.target;
                existing.current_frame = missile.current_frame;
                existing.owner_type = missile.owner_type;
                existing.owner_id = missile.owner_id;
                existing.skill_level = missile.skill_level;
                existing.pierce_level = missile.pierce_level;
            })
            .or_insert(missile);
    }

    fn upsert_skill_cast_location(
        &mut self,
        attacker_type: u8,
        attacker_id: u32,
        skill_id: u16,
        skill_level: u8,
        target: Coordinate,
    ) -> bool {
        let owner_id = self.resolve_unit_owner_id(attacker_type, attacker_id);
        let origin = self
            .unit_location(attacker_type, attacker_id)
            .unwrap_or(target);
        let mut marker = Missile::new(
            skill_cast_marker_id(attacker_type, owner_id, skill_id),
            origin,
        );
        marker.set_class_id(skill_id);
        marker.set_owner_type(attacker_type);
        marker.set_owner_id(owner_id);
        marker.set_skill_level(skill_level);
        marker.set_target(target);
        self.upsert_missile(marker);
        true
    }

    fn upsert_skill_cast_target(
        &mut self,
        attacker_type: u8,
        attacker_id: u32,
        skill_id: u16,
        skill_level: u8,
        target_type: u8,
        target_id: u32,
    ) -> bool {
        let owner_id = self.resolve_unit_owner_id(attacker_type, attacker_id);
        let attacker_location = self.unit_location(attacker_type, attacker_id);
        let target_location = self.unit_location(target_type, target_id);
        let Some(location) = attacker_location.or(target_location) else {
            return false;
        };

        let mut marker = Missile::new(
            skill_cast_marker_id(attacker_type, owner_id, skill_id),
            location,
        );
        marker.set_class_id(skill_id);
        marker.set_owner_type(attacker_type);
        marker.set_owner_id(owner_id);
        marker.set_skill_level(skill_level);
        if let Some(target_location) = target_location {
            marker.set_target(target_location);
        }
        self.upsert_missile(marker);
        true
    }

    fn resolve_unit_owner_id(&self, unit_type: u8, unit_id: u32) -> u32 {
        if unit_type == 0x00 {
            self.resolve_player_id(unit_id)
        } else {
            unit_id
        }
    }

    fn upsert_mercenary(
        &mut self,
        skill_id: u8,
        summon_type: u16,
        player_id: u32,
        merc_id: u32,
        seed2: u32,
        init_seed: u32,
    ) {
        let player_id = self.resolve_player_id(player_id);
        let owner_identity = self
            .players
            .get(&player_id)
            .map(|player| (player.class(), player.name().to_owned()));
        if let Some(player) = self.players.get_mut(&player_id) {
            player.mercenary_id_set(merc_id);
        }
        let assigned_npc = self.npcs.remove(&merc_id);
        let mercenary = self
            .mercenaries
            .entry(merc_id)
            .and_modify(|mercenary| {
                mercenary.refresh_assignment(summon_type, player_id, skill_id, seed2, init_seed);
            })
            .or_insert_with(|| {
                Mercenary::new(merc_id, summon_type, player_id, skill_id, seed2, init_seed)
            });
        if let Some((class, name)) = owner_identity {
            mercenary.set_owner_identity(class, name);
        }
        if let Some(npc) = assigned_npc {
            mercenary.set_location(npc.location());
            if let Some(life_percent) = npc.life_percent() {
                mercenary.set_life_percent(life_percent);
            }
        }
    }

    fn reattach_mercenaries_by_owner_identity(
        &mut self,
        player_id: u32,
        class: CharacterClass,
        name: &str,
    ) {
        if name.is_empty() {
            return;
        }

        let mut mercenary_ids = Vec::new();
        for mercenary in self.mercenaries.values_mut() {
            if mercenary.owner_matches(class, name) {
                mercenary.set_owner_id(player_id);
                mercenary_ids.push(mercenary.id());
            }
        }

        if let Some(player) = self.players.get_mut(&player_id) {
            for mercenary_id in mercenary_ids {
                player.mercenary_id_set(mercenary_id);
            }
        }
    }

    fn current_mercenary_mut(&mut self) -> Option<&mut Mercenary> {
        if let Some(local_player_id) = self.local_player_id {
            let local_player_id = self.resolve_player_id(local_player_id);
            if let Some(player) = self.players.get(&local_player_id)
                && player.has_mercenary()
            {
                return self.mercenaries.get_mut(&player.mercenary_id());
            }
        }

        if self.mercenaries.len() == 1 {
            let mercenary_id = *self.mercenaries.keys().next()?;
            return self.mercenaries.get_mut(&mercenary_id);
        }

        None
    }

    fn set_mercenary_stat(&mut self, merc_id: u32, stat_id: u16, amount: u32) -> bool {
        let Some(mercenary) = self.mercenaries.get_mut(&merc_id) else {
            return false;
        };
        mercenary.set_stat(stat_id, game_state_stat_value(stat_id, amount));
        true
    }

    fn add_mercenary_stat(&mut self, merc_id: u32, stat_id: u16, amount: u32) -> bool {
        let Some(mercenary) = self.mercenaries.get_mut(&merc_id) else {
            return false;
        };
        mercenary.add_stat(stat_id, game_state_stat_value(stat_id, amount));
        true
    }

    fn set_current_mercenary_revive_cost(&mut self, revive_name_id: u16, revive_cost: u16) -> bool {
        let Some(mercenary) = self.current_mercenary_mut() else {
            return false;
        };
        mercenary.set_revive_info(revive_name_id, revive_cost);
        true
    }

    fn update_player_corpse(&mut self, assign: u8, owner_id: u32, corpse_id: u32) -> bool {
        if assign == 0 {
            return self.player_corpses.remove(&corpse_id).is_some();
        }
        self.player_corpses
            .insert(corpse_id, PlayerCorpse::new(owner_id, corpse_id));
        true
    }

    fn set_unit_state(
        &mut self,
        unit_type: u8,
        unit_id: u32,
        state: u8,
        state_effects: Vec<u8>,
    ) -> bool {
        self.unit_states
            .entry(UnitKey::new(unit_type, unit_id))
            .or_default()
            .set_state(state, state_effects);
        true
    }

    fn set_multi_states(&mut self, unit_type: u8, unit_id: u32, state_effects: Vec<u8>) -> bool {
        self.unit_states
            .entry(UnitKey::new(unit_type, unit_id))
            .or_default()
            .set_multi_state_effects(state_effects);
        true
    }

    fn end_unit_state(&mut self, unit_type: u8, unit_id: u32, state: u8) -> bool {
        let key = UnitKey::new(unit_type, unit_id);
        let Some(state_set) = self.unit_states.get_mut(&key) else {
            return false;
        };
        let removed = state_set.clear_state(state);
        if state_set.is_empty() {
            self.unit_states.remove(&key);
        }
        removed
    }

    fn remove_unit(&mut self, unit_type: u8, unit_id: u32) -> bool {
        self.player_corpses.remove(&unit_id);
        self.unit_states.remove(&UnitKey::new(unit_type, unit_id));
        match unit_type {
            0x00 => self.clear_player_world_location(unit_id),
            0x01 => {
                if let Some(mercenary) = self.mercenaries.get_mut(&unit_id) {
                    mercenary.clear_world_location();
                    true
                } else {
                    self.npcs.remove(&unit_id).is_some()
                }
            }
            0x02 | 0x05 => self.objects.remove(&unit_id).is_some(),
            0x03 => self.missiles.remove(&unit_id).is_some(),
            0x04 => self.items.remove(&unit_id).is_some(),
            _ => false,
        }
    }

    fn update_object_state(
        &mut self,
        unit_id: u32,
        portal_flags: u8,
        is_targetable: u8,
        unit_state: u32,
    ) -> bool {
        let Some(object) = self.objects.get_mut(&unit_id) else {
            return false;
        };
        object.set_portal_flags(portal_flags);
        object.set_targetable(is_targetable);
        object.set_state(unit_state);
        true
    }

    fn resolve_player_id(&self, unit_id: u32) -> u32 {
        let mut current = unit_id;
        for _ in 0..8 {
            let Some(next) = self.player_aliases.get(&current).copied() else {
                break;
            };
            if next == current {
                break;
            }
            current = next;
        }
        current
    }

    fn find_player_id_by_identity(&self, class: CharacterClass, name: &str) -> Option<u32> {
        if name.is_empty() {
            return None;
        }

        self.players.iter().find_map(|(id, player)| {
            (player.class() == class && player.name().eq_ignore_ascii_case(name)).then_some(*id)
        })
    }

    fn link_player_alias(&mut self, alias_id: u32, canonical_id: u32) {
        if alias_id != canonical_id {
            self.player_aliases.insert(alias_id, canonical_id);
            let mut reassigned_mercenary_ids = Vec::new();
            for mercenary in self.mercenaries.values_mut() {
                if mercenary.owner_id() == alias_id {
                    reassigned_mercenary_ids.push(mercenary.id());
                    mercenary.set_owner_id(canonical_id);
                }
            }
            if let Some(player) = self.players.get_mut(&canonical_id) {
                for mercenary_id in reassigned_mercenary_ids {
                    player.mercenary_id_set(mercenary_id);
                }
            }
            self.apply_pending_player_stats(alias_id, canonical_id);
        }
    }

    fn remove_player(&mut self, unit_id: u32) -> bool {
        let canonical_id = self.resolve_player_id(unit_id);
        let local_removed = self.local_player_id.is_some_and(|local_id| {
            local_id == unit_id || self.resolve_player_id(local_id) == canonical_id
        });
        let removed = self.players.remove(&canonical_id).is_some();

        self.player_aliases.retain(|alias_id, target_id| {
            *alias_id != unit_id
                && *alias_id != canonical_id
                && *target_id != unit_id
                && *target_id != canonical_id
        });
        self.pending_player_stats
            .retain(|pending_id, _| *pending_id != unit_id && *pending_id != canonical_id);
        self.pending_player_parties
            .retain(|pending_id, _| *pending_id != unit_id && *pending_id != canonical_id);
        self.pending_remote_party_info
            .retain(|pending_id, _| *pending_id != unit_id && *pending_id != canonical_id);

        if local_removed {
            self.local_player_id = None;
        }

        self.player_corpses
            .retain(|_, corpse| corpse.owner_id() != unit_id && corpse.owner_id() != canonical_id);
        self.unit_states.remove(&UnitKey::new(0, unit_id));
        self.unit_states.remove(&UnitKey::new(0, canonical_id));

        removed
    }

    fn clear_player_world_location(&mut self, unit_id: u32) -> bool {
        let canonical_id = self.resolve_player_id(unit_id);
        let Some(player) = self.players.get_mut(&canonical_id) else {
            return false;
        };
        player.clear_world_location();
        true
    }

    fn clear_area_world_state(&mut self, preserve_local_player_location: bool) {
        self.npcs.clear();
        self.missiles.clear();
        self.objects.clear();
        self.player_corpses.clear();
        self.unit_states.clear();
        self.items.clear();
        self.item_stat_updates.clear();
        self.map.revealed_tiles.clear();
        for mercenary in self.mercenaries.values_mut() {
            mercenary.clear_world_location();
        }
        let retained_player_id = if preserve_local_player_location {
            self.local_player_id
                .map(|local_id| self.resolve_player_id(local_id))
        } else {
            None
        };
        for (player_id, player) in self.players.iter_mut() {
            if Some(*player_id) != retained_player_id {
                player.clear_world_location();
            }
        }
    }

    pub(crate) fn reset_session(&mut self) {
        self.players.clear();
        self.player_aliases.clear();
        self.pending_player_stats.clear();
        self.pending_player_parties.clear();
        self.pending_remote_party_info.clear();
        self.npcs.clear();
        self.mercenaries.clear();
        self.missiles.clear();
        self.objects.clear();
        self.player_corpses.clear();
        self.unit_states.clear();
        self.items.clear();
        self.item_stat_updates.clear();
        self.map = GameMapState::default();
        self.local_player_id = None;
        self.player_quest_log = None;
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(
            GameServerType::SinglePlayer,
            Difficulty::Normal,
            Locale::enUS,
        )
    }
}

impl Update for GameState {
    fn update(&mut self, packet: ServerMessage) -> bool {
        match packet {
            ServerMessage::GameLoading => {
                if self.local_player_id.is_some() {
                    false
                } else {
                    self.reset_session();
                    true
                }
            }
            ServerMessage::GameFlags {
                difficulty,
                arena_flags,
                is_expansion,
                is_ladder,
            } => {
                if let Some(difficulty) = Difficulty::from_packet_value(difficulty) {
                    self.difficulty = difficulty;
                }
                self.is_hardcore = arena_flags & GameMode::Hardcore as u32 != 0;
                self.is_expansion =
                    is_expansion != 0 || arena_flags & GameMode::Expansion as u32 != 0;
                self.is_ladder = is_ladder != 0 || arena_flags & GameMode::Ladder as u32 != 0;
                true
            }
            ServerMessage::LoadAct {
                act,
                map_id,
                area_id,
                automap,
            } => {
                self.map.act = Some(act);
                self.map.map_id = Some(map_id);
                self.map.area_id = Some(area_id);
                self.map.automap = Some(automap);
                self.clear_area_world_state(true);
                true
            }
            ServerMessage::UnloadComplete => {
                self.clear_area_world_state(false);
                true
            }
            ServerMessage::GameExitSuccessful => {
                self.reset_session();
                true
            }
            ServerMessage::GameConnectionTerminated => {
                self.reset_session();
                true
            }
            ServerMessage::MapReveal {
                tile_x,
                tile_y,
                area_id,
            } => self.map.revealed_tiles.insert(MapTile {
                x: tile_x,
                y: tile_y,
                area_id,
            }),
            ServerMessage::MapHide {
                tile_x,
                tile_y,
                area_id,
            } => self.map.revealed_tiles.remove(&MapTile {
                x: tile_x,
                y: tile_y,
                area_id,
            }),
            ServerMessage::AssignLevelWarp {
                unit_type,
                unit_id,
                warp_class_id,
                warp_x,
                warp_y,
            } => {
                self.objects.insert(
                    unit_id,
                    WorldObject::new(
                        unit_id,
                        unit_type,
                        warp_class_id as u16,
                        Coordinate::new(warp_x, warp_y),
                        0,
                        0,
                    ),
                );
                true
            }
            ServerMessage::GameHandshake { unit_type, unit_id } => {
                if unit_type == 0 {
                    self.local_player_id = Some(unit_id);
                    true
                } else {
                    false
                }
            }
            ServerMessage::AssignPlayer {
                unit_id,
                class,
                szname,
                x,
                y,
            } => {
                let Some(class) = CharacterClass::from_id(class) else {
                    return false;
                };
                self.upsert_assigned_player(
                    unit_id,
                    class,
                    fixed_c_string(&szname),
                    Coordinate::new(x, y),
                );
                true
            }
            ServerMessage::PlayerMove {
                unit_id,
                current_x,
                current_y,
                ..
            }
            | ServerMessage::PlayerStop {
                unit_id,
                x: current_x,
                y: current_y,
                ..
            }
            | ServerMessage::ReassignPlayer {
                unit_id,
                x: current_x,
                y: current_y,
                ..
            } => self.move_player(unit_id, Coordinate::new(current_x, current_y)),
            ServerMessage::MultipleUnitsCoordsUpdate { units, .. } => {
                let mut updated = false;
                for unit in units {
                    updated = self.set_unit_location(
                        unit.unit_type,
                        unit.unit_id,
                        Coordinate::new(unit.x, unit.y),
                    ) || updated;
                }
                updated
            }
            ServerMessage::PlayerJoined {
                player_id,
                character_class,
                character_name,
                character_level,
                party_id,
                ..
            } => {
                let Some(class) = CharacterClass::from_id(character_class) else {
                    return false;
                };
                self.upsert_roster_player(
                    player_id,
                    class,
                    fixed_c_string(&character_name),
                    character_level as u32,
                    PartyAffiliation::from_packet_id(party_id),
                );
                true
            }
            ServerMessage::PlayerLeft { player_id } => self.remove_player(player_id),
            ServerMessage::PlayerPartyInfo {
                unit_id,
                party_id,
                character_level,
                in_party,
                ..
            } => {
                let unit_id = self.resolve_player_id(unit_id);
                let party_affiliation = PartyAffiliation::from_party_info(party_id, in_party);
                if let Some(player) = self.players.get_mut(&unit_id) {
                    player.set_level(character_level as u32);
                    player.set_party_affiliation(party_affiliation);
                    true
                } else {
                    self.pending_player_stats
                        .entry(unit_id)
                        .or_default()
                        .insert(UnitStat::Level as u16, character_level as u32);
                    self.pending_player_parties
                        .insert(unit_id, party_affiliation);
                    true
                }
            }
            ServerMessage::AllyPartyInfo {
                unit_type,
                unit_life,
                unit_id,
                unit_area,
            } => self.update_ally_party_info(unit_type, unit_id, unit_life, unit_area),
            ServerMessage::PlayerInProximity { .. } => false,
            ServerMessage::PlayerMapUpdate {
                player_id,
                player_x,
                player_y,
            } => {
                if player_x > u16::MAX as u32 || player_y > u16::MAX as u32 {
                    return false;
                }
                self.move_player(player_id, Coordinate::new(player_x as u16, player_y as u16))
            }
            ServerMessage::PlayerSkillsInfo {
                player_id, skills, ..
            } => self.set_player_skills(player_id, skills),
            ServerMessage::PlayerPartyUpdate {
                unit_id,
                party_state,
            } => {
                if party_state == 0 {
                    self.set_player_party_affiliation(unit_id, PartyAffiliation::Unpartied)
                } else {
                    self.set_player_party_affiliation(unit_id, PartyAffiliation::LocalParty)
                }
            }
            ServerMessage::AssignPlayerToParty {
                player_id,
                party_id,
            } => self.set_player_party_affiliation(
                player_id,
                PartyAffiliation::from_packet_id(party_id),
            ),
            ServerMessage::MissileData {
                missile_id,
                missile_class,
                missile_x,
                missile_y,
                target_x,
                target_y,
                current_frame,
                owner_type,
                owner_id,
                skill_level,
                pierce_level,
            } => {
                let mut missile = Missile::new(
                    missile_id,
                    Coordinate::new(missile_x as u16, missile_y as u16),
                );
                missile.class_id = Some(missile_class);
                missile.target = Some(Coordinate::new(target_x as u16, target_y as u16));
                missile.current_frame = Some(current_frame);
                missile.owner_type = Some(owner_type);
                missile.owner_id = Some(owner_id);
                missile.skill_level = Some(skill_level);
                missile.pierce_level = Some(pierce_level);
                self.upsert_missile(missile);
                true
            }
            ServerMessage::UnitSkillOnTarget {
                unit_type,
                unit_id,
                skill_id,
                skill_level,
                target_type,
                target_id,
                ..
            }
            | ServerMessage::SkillTriggerOnTarget {
                attacker_type: unit_type,
                attacker_id: unit_id,
                skill_id,
                skill_level,
                target_type,
                target_id,
                ..
            } => self.upsert_skill_cast_target(
                unit_type,
                unit_id,
                skill_id,
                skill_level,
                target_type,
                target_id,
            ),
            ServerMessage::UnitSkillOnLocation {
                unit_type,
                unit_id,
                skill: skill_id,
                skill_level,
                x: target_x,
                y: target_y,
                ..
            }
            | ServerMessage::SkillTriggerOnLocation {
                attacker_type: unit_type,
                attacker_id: unit_id,
                skill_id,
                skill_level,
                target_x,
                target_y,
                ..
            } => self.upsert_skill_cast_location(
                unit_type,
                unit_id,
                skill_id,
                skill_level,
                Coordinate::new(target_x, target_y),
            ),
            ServerMessage::PlayerCorpseAssign {
                assign,
                owner_id,
                corpse_id,
            } => self.update_player_corpse(assign, owner_id, corpse_id),
            ServerMessage::RemoveObject { unit_type, unit_id } => {
                self.remove_unit(unit_type, unit_id)
            }
            ServerMessage::ObjectState {
                unit_type: _,
                unit_id,
                portal_flags,
                is_targetable,
                unit_state,
            } => self.update_object_state(unit_id, portal_flags, is_targetable, unit_state),
            ServerMessage::WorldObject {
                object_type,
                object_id,
                object_class,
                x,
                y,
                state,
                interaction,
            } => {
                self.objects.insert(
                    object_id,
                    WorldObject::new(
                        object_id,
                        object_type,
                        object_class,
                        Coordinate::new(x, y),
                        state as u32,
                        interaction,
                    ),
                );
                true
            }
            ServerMessage::PlayerQuestLogInfo { quest_bits } => {
                self.player_quest_log = Some(PlayerQuestLog::new(quest_bits));
                true
            }
            ServerMessage::NpcMove {
                unit_id,
                target_x,
                target_y,
                ..
            }
            | ServerMessage::NpcMoveToEntity {
                unit_id,
                target_x,
                target_y,
                ..
            }
            | ServerMessage::NpcAction {
                unit_id,
                x: target_x,
                y: target_y,
                ..
            }
            | ServerMessage::NpcAttack {
                unit_id,
                target_x,
                target_y,
                ..
            } => {
                self.move_or_create_npc(unit_id, Coordinate::new(target_x, target_y), None);
                true
            }
            ServerMessage::NpcStateUpdate {
                unit_id,
                state,
                x,
                y,
                unit_life,
                ..
            } => {
                let life = if state == 0x08 || state == 0x09 {
                    0
                } else {
                    unit_life
                };
                self.move_or_create_npc(unit_id, Coordinate::new(x, y), Some(life));
                if let Some(npc) = self.npcs.get_mut(&unit_id) {
                    npc.set_state(state);
                }
                true
            }
            ServerMessage::NpcStop {
                unit_id,
                x,
                y,
                unit_life,
            } => {
                self.move_or_create_npc(unit_id, Coordinate::new(x, y), Some(unit_life));
                true
            }
            ServerMessage::NpcHeal {
                unit_id, unit_life, ..
            } => {
                if let Some(mercenary) = self.mercenaries.get_mut(&unit_id) {
                    mercenary.set_life_percent(unit_life);
                    true
                } else if let Some(npc) = self.npcs.get_mut(&unit_id) {
                    npc.set_life_percent(unit_life);
                    true
                } else {
                    false
                }
            }
            ServerMessage::MonsterAssign {
                unit_id,
                unit_code,
                unit_x,
                unit_y,
                life_percent,
                ..
            } => {
                if let Some(mercenary) = self.mercenaries.get_mut(&unit_id) {
                    mercenary.set_location(Coordinate::new(unit_x, unit_y));
                    mercenary.set_life_percent(life_percent);
                } else {
                    self.npcs.insert(
                        unit_id,
                        Npc::with_class(
                            unit_id,
                            unit_code,
                            Coordinate::new(unit_x, unit_y),
                            life_percent,
                        ),
                    );
                }
                true
            }
            ServerMessage::AssignMerc {
                skill_id,
                summon_type,
                player_id,
                merc_id,
                seed2,
                init_seed,
            } => {
                self.upsert_mercenary(skill_id, summon_type, player_id, merc_id, seed2, init_seed);
                true
            }
            ServerMessage::MercReviveCost {
                merc_name_id,
                revive_cost,
                ..
            } => self.set_current_mercenary_revive_cost(merc_name_id, revive_cost),
            ServerMessage::MercAttributeU8 {
                attribute,
                merc_id,
                amount,
            } => self.set_mercenary_stat(merc_id, attribute as u16, amount as u32),
            ServerMessage::MercAttributeU16 {
                attribute,
                merc_id,
                amount,
            } => self.set_mercenary_stat(merc_id, attribute as u16, amount as u32),
            ServerMessage::MercAttributeU32 {
                attribute,
                merc_id,
                amount,
            } => self.set_mercenary_stat(merc_id, attribute as u16, amount),
            ServerMessage::MercAddExpU8 {
                stat_id,
                merc_id,
                value,
            } => self.add_mercenary_stat(merc_id, stat_id as u16, value as u32),
            ServerMessage::MercAddExpU16 {
                stat_id,
                merc_id,
                value,
            } => self.add_mercenary_stat(merc_id, stat_id as u16, value as u32),
            ServerMessage::SetState {
                unit_type,
                unit_id,
                state,
                state_effects,
                ..
            } => self.set_unit_state(unit_type, unit_id, state, state_effects),
            ServerMessage::EndState {
                unit_type,
                unit_id,
                state,
            } => self.end_unit_state(unit_type, unit_id, state),
            ServerMessage::MultiStates {
                unit_type,
                unit_id,
                state_effects,
                ..
            } => self.set_multi_states(unit_type, unit_id, state_effects),
            ServerMessage::ItemActionWorld {
                action,
                category,
                item_id,
                bitstream,
                ..
            } => {
                self.items.insert(
                    item_id,
                    Item::from_world_packet(item_id, action, category, bitstream),
                );
                true
            }
            ServerMessage::ItemActionOwned {
                action,
                category,
                item_id,
                owner_type,
                owner_id,
                bitstream,
                ..
            } => {
                self.items.insert(
                    item_id,
                    Item::from_owned_packet(
                        item_id, action, category, owner_type, owner_id, bitstream,
                    ),
                );
                true
            }
            ServerMessage::UpdateItemStats {
                packet_size,
                bitstream,
            } => {
                self.item_stat_updates
                    .push(ItemStatUpdate::new(packet_size, bitstream));
                true
            }
            ServerMessage::SetItemState {
                unit_type,
                unit_id,
                item_id,
                and_value,
                flags,
            } => {
                let state_flags = ItemStateFlags::new(unit_type, unit_id, and_value, flags);
                self.items
                    .entry(item_id)
                    .or_insert_with(|| Item::new(item_id))
                    .set_state_flags(state_flags);
                true
            }
            ServerMessage::HPMPUPDATE { packed_bits } => {
                let Some((vitals, movement)) = decode_hpmp_update(&packed_bits) else {
                    return false;
                };
                self.set_local_vitals_and_movement(vitals, movement)
            }
            ServerMessage::LifeManaUpdate { bitfield } => {
                let Some((vitals, movement)) = decode_life_mana_update(&bitfield) else {
                    return false;
                };
                self.set_local_vitals_and_movement(vitals, movement)
            }
            ServerMessage::WalkUpdate { bitfield } => {
                let Some((stamina, movement)) = decode_walk_update(&bitfield) else {
                    return false;
                };
                self.set_local_stamina_and_movement(stamina, movement)
            }
            ServerMessage::AddExpU8 { amount } => self.add_local_experience(amount as u32),
            ServerMessage::AddExpU16 { amount } => self.add_local_experience(amount as u32),
            ServerMessage::AddExpU32 { amount } => self.add_local_experience(amount),
            ServerMessage::SetAttributeU8 { attribute, amount } => {
                self.set_local_stat(attribute as u16, amount as u32)
            }
            ServerMessage::SetAttributeU16 { attribute, amount } => {
                self.set_local_stat(attribute as u16, amount as u32)
            }
            ServerMessage::SetAttributeU32 { attribute, amount } => {
                self.set_local_stat(attribute as u16, amount)
            }
            ServerMessage::AttributeUpdate {
                unit_id,
                attribute,
                amount,
            } => self.set_player_stat(unit_id, attribute as u16, amount),
            _ => false,
        }
    }
}

fn fixed_c_string(bytes: &[u8]) -> String {
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

fn game_state_stat_value(stat_id: u16, wire_value: u32) -> u32 {
    if is_resource_stat(stat_id) {
        wire_value >> 8
    } else {
        wire_value
    }
}

fn is_resource_stat(stat_id: u16) -> bool {
    (UnitStat::Life as u16..=UnitStat::StaminaMax as u16).contains(&stat_id)
}

fn skill_cast_marker_id(unit_type: u8, unit_id: u32, skill_id: u16) -> u32 {
    let mut hash = 0x811C_9DC5u32;
    for byte in [unit_type]
        .into_iter()
        .chain(unit_id.to_le_bytes())
        .chain(skill_id.to_le_bytes())
    {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash | 0x8000_0000
}

fn decode_hpmp_update(raw: &[u8; 14]) -> Option<(PlayerVitals, PlayerMovement)> {
    let mut reader = StatusBitReader::new(raw);
    let life = reader.bits(15)? as u16;
    let mana = reader.bits(15)? as u16;
    let stamina = reader.bits(15)? as u16;
    let life_regen = reader.bits(7)? as u8;
    let mana_regen = reader.bits(7)? as u8;
    let movement = decode_status_movement(&mut reader)?;

    Some((
        PlayerVitals::new(life, mana, stamina).with_regen(life_regen, mana_regen),
        movement,
    ))
}

fn decode_life_mana_update(raw: &[u8; 12]) -> Option<(PlayerVitals, PlayerMovement)> {
    let mut reader = StatusBitReader::new(raw);
    let life = reader.bits(15)? as u16;
    let mana = reader.bits(15)? as u16;
    let stamina = reader.bits(15)? as u16;
    let movement = decode_status_movement(&mut reader)?;

    Some((PlayerVitals::new(life, mana, stamina), movement))
}

fn decode_walk_update(raw: &[u8; 8]) -> Option<(u16, PlayerMovement)> {
    let mut reader = StatusBitReader::new(raw);
    let stamina = reader.bits(15)? as u16;
    let movement = decode_status_movement(&mut reader)?;

    Some((stamina, movement))
}

fn decode_status_movement(reader: &mut StatusBitReader<'_>) -> Option<PlayerMovement> {
    let x = reader.bits(16)? as u16;
    let y = reader.bits(16)? as u16;
    let dx = reader.bits(8)? as u8;
    let dy = reader.bits(8)? as u8;
    Some(PlayerMovement::new(Coordinate::new(x, y), dx, dy))
}

struct StatusBitReader<'a> {
    raw: &'a [u8],
    bit_offset: usize,
}

impl<'a> StatusBitReader<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { raw, bit_offset: 0 }
    }

    fn bits(&mut self, count: usize) -> Option<u32> {
        if count > 32 || self.bit_offset.checked_add(count)? > self.raw.len() * 8 {
            return None;
        }

        let mut value = 0;
        for index in 0..count {
            let position = self.bit_offset + index;
            let bit = (self.raw[position / 8] >> (position % 8)) & 1;
            value |= (bit as u32) << index;
        }
        self.bit_offset += count;
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::skill_cast_marker_id;
    use crate::core::entity::Entity;
    use crate::core::network::d2gs::D2GSPacket;
    use crate::core::quest::{QuestLogEntry, QuestLogEntryState};
    use crate::core::unit_stat::UnitStat;
    use crate::core::update::Update;
    use crate::{
        CharacterClass, Coordinate, Difficulty, GameState, ItemDestination, ItemOwner,
        ItemPlacement, PartyAffiliation, PartyId, ServerMessage, SkillDescription,
    };

    #[test]
    fn game_flags_update_difficulty_and_mode_flags() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::GameFlags {
            difficulty: 2,
            arena_flags: 0x0010_0800,
            is_expansion: 0,
            is_ladder: 1,
        }));

        assert_eq!(state.difficulty(), Difficulty::Hell);
        assert!(state.is_hardcore());
        assert!(state.is_expansion());
        assert!(state.is_ladder());
    }

    #[test]
    fn assign_player_packet_creates_player_memory() {
        let mut state = GameState::default();
        let mut packet = vec![0x59, 0x04, 0x03, 0x02, 0x01, 0x03];
        packet.extend_from_slice(b"Rusty\0\0\0\0\0\0\0\0\0\0\0");
        packet.extend_from_slice(&1234u16.to_le_bytes());
        packet.extend_from_slice(&5678u16.to_le_bytes());

        let applied = state
            .apply_packet(&D2GSPacket { data: packet })
            .expect("assign player should parse");

        let player = state.player(0x0102_0304).expect("player exists");
        assert!(applied);
        assert_eq!(state.local_player_id(), None);
        assert_eq!(player.class(), CharacterClass::Paladin);
        assert_eq!(player.name(), "Rusty");
        assert_eq!(player.location().x(), 1234);
        assert_eq!(player.location().y(), 5678);
        assert!(player.world_location_known());
    }

    #[test]
    fn player_move_updates_existing_player_position() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 4,
            szname: *b"Barb\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 11,
        });

        assert!(state.update(ServerMessage::PlayerMove {
            unit_type: 0,
            unit_id: 7,
            move_type: 0x17,
            target_x: 20,
            target_y: 21,
            unit_hit_class: 0,
            current_x: 18,
            current_y: 19,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.location().x(), 18);
        assert_eq!(player.location().y(), 19);
        assert!(player.world_location_known());
    }

    #[test]
    fn player_map_update_moves_existing_player() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });

        assert!(state.update(ServerMessage::PlayerMapUpdate {
            player_id: 7,
            player_x: 5118,
            player_y: 5168,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.location().x(), 5118);
        assert_eq!(player.location().y(), 5168);
    }

    #[test]
    fn player_party_info_updates_known_player_level() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });

        assert!(state.update(ServerMessage::PlayerPartyInfo {
            unit_id: 7,
            party_id: 0xffff,
            character_level: 88,
            relationship: 0,
            in_party: 0,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.level(), 88);
        assert_eq!(player.party_affiliation(), PartyAffiliation::Unpartied);
    }

    #[test]
    fn player_party_info_marks_local_party_when_party_id_is_not_assigned_yet() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });

        assert!(state.update(ServerMessage::PlayerPartyInfo {
            unit_id: 7,
            party_id: 0xffff,
            character_level: 88,
            relationship: 0,
            in_party: 1,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.party_affiliation(), PartyAffiliation::LocalParty);
    }

    #[test]
    fn party_assignment_packets_update_player_affiliation() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 7,
            character_class: 3,
            character_name: name16("Remote"),
            character_level: 42,
            party_id: 0xffff,
            unknown: [0; 8],
        }));

        assert!(state.update(ServerMessage::AssignPlayerToParty {
            player_id: 7,
            party_id: 0x1234,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(
            player.party_affiliation(),
            PartyAffiliation::Party(PartyId::new(0x1234))
        );

        assert!(state.update(ServerMessage::PlayerPartyUpdate {
            unit_id: 7,
            party_state: 0,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.party_affiliation(), PartyAffiliation::Unpartied);

        assert!(state.update(ServerMessage::PlayerPartyUpdate {
            unit_id: 7,
            party_state: 1,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.party_affiliation(), PartyAffiliation::LocalParty);
    }

    #[test]
    fn local_level_stat_updates_player_level() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::Level as u8,
            amount: 42,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.level(), 42);
    }

    #[test]
    fn resource_stat_updates_are_stored_as_game_values() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::LifeMax as u8,
            amount: 66 * 256,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::ManaMax as u8,
            amount: 35 * 256,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.stat(UnitStat::LifeMax as u16), Some(66));
        assert_eq!(player.stat(UnitStat::ManaMax as u16), Some(35));
    }

    #[test]
    fn player_skills_info_updates_raw_skills_and_legacy_save_table() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: CharacterClass::Sorceress as u8,
            szname: *b"Sorc\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });

        assert!(state.update(ServerMessage::PlayerSkillsInfo {
            skills_count: 3,
            player_id: 7,
            skills: vec![
                SkillDescription {
                    skill: 36,
                    level: 3,
                },
                SkillDescription {
                    skill: 64,
                    level: 1,
                },
                SkillDescription {
                    skill: 6,
                    level: 20,
                },
            ],
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.skills().get(36), Some(3));
        assert_eq!(player.skills().get(64), Some(1));
        assert_eq!(player.skills().get(6), Some(20));
        let save_table = player.legacy_save_skills();
        assert_eq!(save_table[0], 3);
        assert_eq!(save_table[28], 1);
        assert!(!save_table.contains(&20));
    }

    #[test]
    fn ally_party_info_updates_remote_player_life_and_area() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Remote"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));

        assert!(state.update(ServerMessage::AllyPartyInfo {
            unit_type: 0,
            unit_life: 87,
            unit_id: 0x1000,
            unit_area: 2,
        }));

        let player = state.player(0x1000).expect("remote player exists");
        let remote_party_info = player
            .remote_party_info()
            .expect("party info should be tracked separately");
        assert_eq!(remote_party_info.life().map(|life| life.raw()), Some(87));
        assert_eq!(remote_party_info.area_id(), Some(2));
        assert_eq!(player.stat(UnitStat::Life as u16), None);
        assert_eq!(player.stat(UnitStat::LifeMax as u16), None);
        assert_eq!(player.area_id(), None);
    }

    #[test]
    fn early_ally_party_info_applies_after_player_is_known() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::AllyPartyInfo {
            unit_type: 0,
            unit_life: 87,
            unit_id: 0x1000,
            unit_area: 2,
        }));
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Remote"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));

        let player = state.player(0x1000).expect("remote player exists");
        let remote_party_info = player
            .remote_party_info()
            .expect("party info should be tracked separately");
        assert_eq!(remote_party_info.life().map(|life| life.raw()), Some(87));
        assert_eq!(remote_party_info.area_id(), Some(2));
        assert_eq!(player.stat(UnitStat::Life as u16), None);
        assert_eq!(player.stat(UnitStat::LifeMax as u16), None);
        assert_eq!(player.area_id(), None);
    }

    #[test]
    fn early_player_stat_updates_apply_after_player_alias_is_known() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Remote"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AttributeUpdate {
            unit_id: 0x2000,
            attribute: UnitStat::Mana as u8,
            amount: 320 * 256,
        }));

        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 1,
            szname: name16("Remote"),
            x: 10,
            y: 20,
        }));

        let player = state.player(0x1000).expect("player exists");
        assert_eq!(player.stat(UnitStat::Mana as u16), Some(320));
    }

    #[test]
    fn assign_merc_preserves_prior_npc_location_and_life() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: name16("Owner"),
            x: 10,
            y: 20,
        });
        assert!(state.update(ServerMessage::MonsterAssign {
            unit_id: 0x5566_7788,
            unit_code: 0x0152,
            unit_x: 5200,
            unit_y: 5100,
            life_percent: 73,
            packet_size: 13,
            bitstream: Vec::new(),
        }));

        assert!(state.update(ServerMessage::AssignMerc {
            skill_id: 0x0A,
            summon_type: 0x0152,
            player_id: 7,
            merc_id: 0x5566_7788,
            seed2: 0x99AA_BBCC,
            init_seed: 0xDDEE_FF00,
        }));

        assert!(state.npc(0x5566_7788).is_none());
        let mercenary = state.mercenary(0x5566_7788).expect("merc exists");
        assert_eq!(mercenary.location(), Coordinate::new(5200, 5100));
        assert!(mercenary.world_location_known());
        assert_eq!(mercenary.life_percent(), Some(73));
    }

    #[test]
    fn remove_object_for_known_mercenary_clears_visibility_but_keeps_assignment() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: name16("Owner"),
            x: 10,
            y: 20,
        });
        assert!(state.update(ServerMessage::AssignMerc {
            skill_id: 0x0A,
            summon_type: 0x0152,
            player_id: 7,
            merc_id: 0x5566_7788,
            seed2: 0x99AA_BBCC,
            init_seed: 0xDDEE_FF00,
        }));
        assert!(state.update(ServerMessage::NpcStop {
            unit_id: 0x5566_7788,
            x: 5200,
            y: 5100,
            unit_life: 73,
        }));

        assert!(state.update(ServerMessage::RemoveObject {
            unit_type: 1,
            unit_id: 0x5566_7788,
        }));

        let player = state.player(7).expect("owner remains");
        assert_eq!(player.mercenary_id(), 0x5566_7788);
        let mercenary = state.mercenary(0x5566_7788).expect("merc remains");
        assert!(!mercenary.world_location_known());
        assert_eq!(mercenary.life_percent(), Some(73));
    }

    #[test]
    fn player_alias_reassigns_existing_mercenary_owner() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Owner"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AssignMerc {
            skill_id: 0x0A,
            summon_type: 0x0152,
            player_id: 0x2000,
            merc_id: 0x5566_7788,
            seed2: 0x99AA_BBCC,
            init_seed: 0xDDEE_FF00,
        }));
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 1,
            szname: name16("Owner"),
            x: 10,
            y: 20,
        }));

        let player = state.player(0x1000).expect("player exists");
        assert_eq!(player.mercenary_id(), 0x5566_7788);
        let mercenary = state.mercenary(0x5566_7788).expect("merc exists");
        assert_eq!(mercenary.owner_id(), 0x1000);
    }

    #[test]
    fn mercenary_owner_identity_relinks_after_player_reappears() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Owner"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AssignMerc {
            skill_id: 0x0A,
            summon_type: 0x0152,
            player_id: 0x1000,
            merc_id: 0x5566_7788,
            seed2: 0x99AA_BBCC,
            init_seed: 0xDDEE_FF00,
        }));

        assert!(state.update(ServerMessage::PlayerLeft { player_id: 0x1000 }));
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x3000,
            character_class: 1,
            character_name: name16("Owner"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));

        let player = state.player(0x3000).expect("player exists");
        assert_eq!(player.mercenary_id(), 0x5566_7788);
        let mercenary = state.mercenary(0x5566_7788).expect("merc exists");
        assert_eq!(mercenary.owner_id(), 0x3000);
    }

    #[test]
    fn game_exit_clears_player_roster_and_local_identity() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        assert!(state.update(ServerMessage::GameExitSuccessful));

        assert!(state.players().is_empty());
        assert_eq!(state.local_player_id(), None);
        assert!(state.map().revealed_tiles.is_empty());
    }

    #[test]
    fn game_connection_terminated_clears_player_roster_and_local_identity() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        assert!(state.update(ServerMessage::GameConnectionTerminated));

        assert!(state.players().is_empty());
        assert_eq!(state.local_player_id(), None);
    }

    #[test]
    fn duplicate_game_loading_does_not_clear_active_session() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: name16("Local"),
            x: 10,
            y: 20,
        }));
        mark_local(&mut state, 7);
        assert_eq!(state.players().len(), 1);

        assert!(!state.update(ServerMessage::GameLoading));

        assert_eq!(state.players().len(), 1);
        assert_eq!(state.local_player_id(), Some(7));
        assert!(state.player(7).is_some());
    }

    #[test]
    fn load_act_keeps_last_known_player_position_from_earlier_movement_packet() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Joan\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        state.update(ServerMessage::AssignPlayer {
            unit_id: 8,
            class: 2,
            szname: *b"Orin\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 30,
            y: 40,
        });
        mark_local(&mut state, 7);
        assert!(state.update(ServerMessage::LifeManaUpdate {
            bitfield: status_bitfield::<12>(&[
                (100, 15),
                (100, 15),
                (100, 15),
                (5118, 16),
                (5168, 16),
                (0, 8),
                (0, 8),
            ]),
        }));

        assert!(state.update(ServerMessage::LoadAct {
            act: 0,
            map_id: 0x1234,
            area_id: 2,
            automap: 0,
        }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.location().x(), 5118);
        assert_eq!(player.location().y(), 5168);
        assert!(player.world_location_known());
        assert!(
            !state
                .player(8)
                .expect("remote player exists")
                .world_location_known()
        );
    }

    #[test]
    fn player_left_removes_coalesced_roster_and_assignment_ids() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 1,
            character_name: name16("Alias"),
            character_level: 42,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 1,
            szname: name16("Alias"),
            x: 5200,
            y: 5100,
        }));

        assert_eq!(state.players().len(), 1);
        assert!(state.player(0x1000).is_some());
        assert!(state.player(0x2000).is_some());

        assert!(state.update(ServerMessage::PlayerLeft { player_id: 0x1000 }));
        assert!(state.players().is_empty());
        assert!(state.player(0x1000).is_none());
        assert!(state.player(0x2000).is_none());
    }

    #[test]
    fn remove_player_unit_clears_world_marker_but_keeps_roster() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 2,
            character_name: name16("UnitAlias"),
            character_level: 12,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 2,
            szname: name16("UnitAlias"),
            x: 5210,
            y: 5110,
        }));

        assert!(state.update(ServerMessage::RemoveObject {
            unit_type: 0,
            unit_id: 0x2000,
        }));
        assert_eq!(state.players().len(), 1);
        let player = state.player(0x1000).expect("player remains in roster");
        assert_eq!(player.location().x(), 5210);
        assert_eq!(player.location().y(), 5110);
        assert!(!player.world_location_known());

        assert!(state.update(ServerMessage::PlayerLeft { player_id: 0x1000 }));
        assert!(state.players().is_empty());
    }

    #[test]
    fn roster_update_after_assignment_keeps_world_position() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 2,
            szname: name16("LateRoster"),
            x: 5210,
            y: 5110,
        }));
        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 2,
            character_name: name16("LateRoster"),
            character_level: 12,
            party_id: 0,
            unknown: [0; 8],
        }));

        assert_eq!(state.players().len(), 1);
        let player = state.player(0x1000).expect("roster alias resolves");
        assert_eq!(player.location().x(), 5210);
        assert_eq!(player.location().y(), 5110);
        assert_eq!(player.level(), 12);
        assert!(player.world_location_known());
    }

    #[test]
    fn local_player_id_resolves_through_player_aliases() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerJoined {
            packet_length: 36,
            player_id: 0x1000,
            character_class: 3,
            character_name: name16("LocalAlias"),
            character_level: 1,
            party_id: 0,
            unknown: [0; 8],
        }));
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x2000,
            class: 3,
            szname: name16("LocalAlias"),
            x: 5000,
            y: 5001,
        }));
        assert!(state.update(ServerMessage::GameHandshake {
            unit_type: 0,
            unit_id: 0x2000,
        }));

        assert_eq!(state.local_player_id(), Some(0x1000));
        assert!(state.update(ServerMessage::LifeManaUpdate {
            bitfield: status_bitfield::<12>(&[
                (1, 15),
                (2, 15),
                (3, 15),
                (5002, 16),
                (5003, 16),
                (0, 8),
                (0, 8),
            ]),
        }));
        assert_eq!(
            state
                .player(0x1000)
                .expect("canonical player")
                .location()
                .x(),
            5002
        );
    }

    #[test]
    fn life_mana_update_packets_update_local_vitals_and_position() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Sorc\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        let bitfield = status_bitfield::<12>(&[
            (1234, 15),
            (567, 15),
            (2500, 15),
            (101, 16),
            (202, 16),
            (9, 8),
            (10, 8),
        ]);

        assert!(state.update(ServerMessage::LifeManaUpdate { bitfield }));

        let player = state.player(7).expect("player exists");
        let vitals = player.vitals().expect("vitals should update");
        let movement = player.movement().expect("movement should update");
        assert_eq!(vitals.life(), Some(1234));
        assert_eq!(vitals.mana(), Some(567));
        assert_eq!(vitals.stamina(), Some(2500));
        assert_eq!(vitals.life_regen(), None);
        assert_eq!(movement.location().x(), 101);
        assert_eq!(movement.location().y(), 202);
        assert_eq!(movement.dx(), 9);
        assert_eq!(movement.dy(), 10);
        assert_eq!(player.location().x(), 101);
        assert_eq!(player.location().y(), 202);
    }

    #[test]
    fn hpmp_update_packet_records_regen_counters() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 4,
            szname: *b"Barb\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        let packed_bits = status_bitfield::<14>(&[
            (3000, 15),
            (1200, 15),
            (1800, 15),
            (12, 7),
            (34, 7),
            (333, 16),
            (444, 16),
            (5, 8),
            (6, 8),
        ]);

        assert!(state.update(ServerMessage::HPMPUPDATE { packed_bits }));

        let player = state.player(7).expect("player exists");
        let vitals = player.vitals().expect("vitals should update");
        assert_eq!(vitals.life(), Some(3000));
        assert_eq!(vitals.mana(), Some(1200));
        assert_eq!(vitals.stamina(), Some(1800));
        assert_eq!(vitals.life_regen(), Some(12));
        assert_eq!(vitals.mana_regen(), Some(34));
        assert_eq!(player.location().x(), 333);
        assert_eq!(player.location().y(), 444);
    }

    #[test]
    fn walk_update_refreshes_stamina_and_local_position() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 0,
            szname: *b"Ama\0\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);
        state.update(ServerMessage::LifeManaUpdate {
            bitfield: status_bitfield::<12>(&[
                (900, 15),
                (300, 15),
                (700, 15),
                (10, 16),
                (20, 16),
                (0, 8),
                (0, 8),
            ]),
        });

        let bitfield = status_bitfield::<8>(&[(123, 15), (111, 16), (222, 16), (3, 8), (4, 8)]);

        assert!(state.update(ServerMessage::WalkUpdate { bitfield }));

        let player = state.player(7).expect("player exists");
        let vitals = player.vitals().expect("vitals should update");
        let movement = player.movement().expect("movement should update");
        assert_eq!(vitals.life(), Some(900));
        assert_eq!(vitals.mana(), Some(300));
        assert_eq!(vitals.stamina(), Some(123));
        assert_eq!(movement.location().x(), 111);
        assert_eq!(movement.location().y(), 222);
        assert_eq!(movement.dx(), 3);
        assert_eq!(movement.dy(), 4);
    }

    #[test]
    fn walk_update_without_prior_life_mana_keeps_unknown_values_unknown() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 0,
            szname: *b"Ama\0\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        });
        mark_local(&mut state, 7);

        let bitfield = status_bitfield::<8>(&[(123, 15), (111, 16), (222, 16), (3, 8), (4, 8)]);

        assert!(state.update(ServerMessage::WalkUpdate { bitfield }));

        let vitals = state
            .player(7)
            .expect("player exists")
            .vitals()
            .expect("stamina should seed vitals");
        assert_eq!(vitals.life(), None);
        assert_eq!(vitals.mana(), None);
        assert_eq!(vitals.stamina(), Some(123));
    }

    #[test]
    fn world_object_packet_is_tracked_and_removed_by_unit_type() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::WorldObject {
            object_type: 2,
            object_id: 99,
            object_class: 344,
            x: 1000,
            y: 1001,
            state: 1,
            interaction: 0,
        }));

        let object = state.object(99).expect("object exists");
        assert_eq!(object.class_id(), 344);
        assert_eq!(object.location().x(), 1000);

        assert!(state.update(ServerMessage::RemoveObject {
            unit_type: 2,
            unit_id: 99,
        }));
        assert!(state.object(99).is_none());
    }

    #[test]
    fn object_state_packet_updates_known_world_object() {
        let mut state = GameState::default();
        state.update(ServerMessage::WorldObject {
            object_type: 2,
            object_id: 99,
            object_class: 344,
            x: 1000,
            y: 1001,
            state: 1,
            interaction: 0,
        });

        assert!(state.update(ServerMessage::ObjectState {
            unit_type: 2,
            unit_id: 99,
            portal_flags: 0x03,
            is_targetable: 0x01,
            unit_state: 0x1122_3344,
        }));

        let object = state.object(99).expect("object exists");
        assert_eq!(object.state(), 0x1122_3344);
        assert_eq!(object.portal_flags(), Some(0x03));
        assert_eq!(object.is_targetable(), Some(0x01));
    }

    #[test]
    fn level_warp_packet_is_tracked_as_world_object() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::AssignLevelWarp {
            unit_type: 2,
            unit_id: 77,
            warp_class_id: 5,
            warp_x: 1200,
            warp_y: 1300,
        }));

        let object = state.object(77).expect("level warp exists");
        assert_eq!(object.object_type(), 2);
        assert_eq!(object.class_id(), 5);
        assert_eq!(object.location().x(), 1200);
        assert_eq!(object.location().y(), 1300);
    }

    #[test]
    fn npc_assign_and_movement_packets_update_npc_memory() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::MonsterAssign {
            unit_id: 55,
            unit_code: 156,
            unit_x: 300,
            unit_y: 301,
            life_percent: 100,
            packet_size: 13,
            bitstream: Vec::new(),
        }));

        assert!(state.update(ServerMessage::NpcMove {
            unit_id: 55,
            unit_type: 1,
            target_x: 320,
            target_y: 321,
            unknown1: 0,
            unknown2: 0,
            velocity: 0,
            unknown4: 0,
        }));

        let npc = state.npc(55).expect("npc exists");
        assert_eq!(npc.class_id(), Some(156));
        assert_eq!(npc.life_percent(), Some(100));
        assert_eq!(npc.location().x(), 320);
        assert_eq!(npc.location().y(), 321);
    }

    #[test]
    fn npc_assign_packet_bytes_parse_and_update_npc_memory() {
        let mut state = GameState::default();
        let mut packet = vec![0xAC];
        packet.extend_from_slice(&55u32.to_le_bytes());
        packet.extend_from_slice(&156u16.to_le_bytes());
        packet.extend_from_slice(&300u16.to_le_bytes());
        packet.extend_from_slice(&301u16.to_le_bytes());
        packet.push(100);
        packet.push(13);

        let applied = state
            .apply_packet(&D2GSPacket { data: packet })
            .expect("npc assign should parse");

        let npc = state.npc(55).expect("npc exists");
        assert!(applied);
        assert_eq!(npc.class_id(), Some(156));
        assert_eq!(npc.location().x(), 300);
        assert_eq!(npc.life_percent(), Some(100));
    }

    #[test]
    fn bulk_position_update_packet_moves_known_units_and_tracks_missiles() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Sorc\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 11,
        });
        state.update(ServerMessage::MonsterAssign {
            unit_id: 55,
            unit_code: 156,
            unit_x: 300,
            unit_y: 301,
            life_percent: 100,
            packet_size: 13,
            bitstream: Vec::new(),
        });

        let packet = D2GSPacket {
            data: vec![
                0x16, 0x00, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x6F, 0x00, 0xDE, 0x00, 0x01,
                0x37, 0x00, 0x00, 0x00, 0x4D, 0x01, 0xBC, 0x01, 0x03, 0x63, 0x00, 0x00, 0x00, 0x90,
                0x01, 0x91, 0x01,
            ],
        };

        let applied = state
            .apply_packet(&packet)
            .expect("bulk position update should parse");

        assert!(applied);
        assert_eq!(
            state.player(7).expect("player exists").location(),
            Coordinate::new(111, 222)
        );
        assert_eq!(
            state.npc(55).expect("npc exists").location(),
            Coordinate::new(333, 444)
        );
        assert_eq!(
            state.missile(99).expect("missile exists").location(),
            Coordinate::new(400, 401)
        );
    }

    #[test]
    fn world_object_packet_bytes_parse_and_update_memory() {
        let mut state = GameState::default();
        let mut packet = vec![0x51, 0x02];
        packet.extend_from_slice(&99u32.to_le_bytes());
        packet.extend_from_slice(&344u16.to_le_bytes());
        packet.extend_from_slice(&1000u16.to_le_bytes());
        packet.extend_from_slice(&1001u16.to_le_bytes());
        packet.push(1);
        packet.push(0);

        let applied = state
            .apply_packet(&D2GSPacket { data: packet })
            .expect("world object should parse");

        let object = state.object(99).expect("object exists");
        assert!(applied);
        assert_eq!(object.object_type(), 2);
        assert_eq!(object.class_id(), 344);
        assert_eq!(object.location().y(), 1001);
    }

    #[test]
    fn item_action_packets_update_item_owner_and_placement() {
        let mut state = GameState::default();
        let ground_bits = item_bitstream(
            0,
            0x60,
            ItemDestination::Ground,
            ItemPlacement::Ground { x: 123, y: 456 },
        );

        assert!(state.update(ServerMessage::ItemActionWorld {
            action: 0x01,
            packet_size: (8 + ground_bits.len()) as u8,
            category: 0x04,
            item_id: 0x1122_3344,
            bitstream: ground_bits,
        }));

        let item = state.item(0x1122_3344).expect("item exists");
        assert_eq!(item.owner(), ItemOwner::World);
        assert_eq!(
            item.packet_data()
                .expect("item prefix should parse")
                .placement,
            ItemPlacement::Ground { x: 123, y: 456 }
        );

        let container_bits = item_bitstream(
            0,
            0x60,
            ItemDestination::Cursor,
            ItemPlacement::Container {
                equipment_location: 0,
                x: 2,
                y: 3,
                container: 1,
            },
        );

        assert!(state.update(ServerMessage::ItemActionOwned {
            action: 0x02,
            packet_size: (13 + container_bits.len()) as u8,
            category: 0x05,
            item_id: 0x1122_3344,
            owner_type: 0,
            owner_id: 0x0102_0304,
            bitstream: container_bits,
        }));

        let item = state.item(0x1122_3344).expect("item still exists");
        assert_eq!(
            item.owner(),
            ItemOwner::Unit {
                unit_type: 0,
                unit_id: 0x0102_0304,
            }
        );
        assert_eq!(
            item.packet_data()
                .expect("item prefix should parse")
                .placement,
            ItemPlacement::Container {
                equipment_location: 0,
                x: 2,
                y: 3,
                container: 1,
            }
        );

        assert!(state.update(ServerMessage::RemoveObject {
            unit_type: 0x04,
            unit_id: 0x1122_3344,
        }));
        assert!(state.item(0x1122_3344).is_none());
    }

    #[test]
    fn item_stat_update_packets_are_preserved_for_later_stat_decoding() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::UpdateItemStats {
            packet_size: 5,
            bitstream: vec![0x10, 0x20, 0x30],
        }));

        let updates = state.item_stat_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].packet_size(), 5);
        assert_eq!(updates[0].bitstream(), &[0x10, 0x20, 0x30]);
    }

    #[test]
    fn set_item_state_packet_updates_matching_item_flags() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::SetItemState {
            unit_type: 0,
            unit_id: 0x0102_0304,
            item_id: 0x5566_7788,
            and_value: 0xAABB_CCDD,
            flags: 0x1122_3344,
        }));

        let item = state.item(0x5566_7788).expect("item state creates item");
        let flags = item.state_flags().expect("item state flags should update");
        assert_eq!(flags.unit_type(), 0);
        assert_eq!(flags.unit_id(), 0x0102_0304);
        assert_eq!(flags.and_value(), 0xAABB_CCDD);
        assert_eq!(flags.flags(), 0x1122_3344);
    }

    #[test]
    fn missile_packet_bytes_parse_and_update_memory() {
        let mut state = GameState::default();
        let packet = D2GSPacket {
            data: vec![
                0x73, 0x78, 0x56, 0x34, 0x12, 0x9A, 0x00, 0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00,
                0x00, 0x30, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x07, 0x00, 0x01, 0x04, 0x03,
                0x02, 0x01, 0x05, 0x06,
            ],
        };

        let applied = state
            .apply_packet(&packet)
            .expect("missile packet should parse");

        let missile = state.missile(0x1234_5678).expect("missile exists");
        assert!(applied);
        assert_eq!(missile.class_id(), Some(0x009A));
        assert_eq!(missile.location(), Coordinate::new(16, 32));
        assert_eq!(missile.target(), Some(Coordinate::new(48, 64)));
        assert_eq!(missile.owner_id(), Some(0x0102_0304));
        assert_eq!(missile.skill_level(), Some(5));
        assert_eq!(missile.pierce_level(), Some(6));
    }

    #[test]
    fn skill_cast_packets_create_map_visible_spell_markers() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: name16("Caster"),
            x: 5200,
            y: 5100,
        }));
        assert!(state.update(ServerMessage::MonsterAssign {
            unit_id: 55,
            unit_code: 156,
            unit_x: 5220,
            unit_y: 5120,
            life_percent: 100,
            packet_size: 13,
            bitstream: Vec::new(),
        }));

        assert!(state.update(ServerMessage::SkillTriggerOnLocation {
            attacker_type: 0,
            attacker_id: 7,
            skill_id: 330,
            unused: 0,
            skill_level: 4,
            target_x: 5230,
            target_y: 5130,
            unused2: 0,
        }));

        let marker_id = skill_cast_marker_id(0, 7, 330);
        let marker = state.missile(marker_id).expect("location spell marker");
        assert_eq!(marker.location(), Coordinate::new(5200, 5100));
        assert_eq!(marker.target(), Some(Coordinate::new(5230, 5130)));
        assert_eq!(marker.class_id(), Some(330));
        assert_eq!(marker.owner_id(), Some(7));
        assert_eq!(marker.skill_level(), Some(4));

        assert!(state.update(ServerMessage::SkillTriggerOnTarget {
            attacker_type: 0,
            attacker_id: 7,
            skill_id: 330,
            skill_level: 5,
            target_type: 1,
            target_id: 55,
            unused: 0,
        }));

        let marker = state.missile(marker_id).expect("target spell marker");
        assert_eq!(marker.location(), Coordinate::new(5200, 5100));
        assert_eq!(marker.target(), Some(Coordinate::new(5220, 5120)));
        assert_eq!(marker.skill_level(), Some(5));
    }

    #[test]
    fn merc_packets_parse_and_update_assignment_stats_and_revive_info() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Sorc\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 11,
        });
        mark_local(&mut state, 7);

        for packet in [
            D2GSPacket {
                data: vec![
                    0x81, 0x0A, 0x52, 0x01, 0x07, 0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55, 0xCC,
                    0xBB, 0xAA, 0x99, 0x00, 0xFF, 0xEE, 0xDD,
                ],
            },
            D2GSPacket {
                data: vec![0x9E, UnitStat::Level as u8, 0x88, 0x77, 0x66, 0x55, 90],
            },
            D2GSPacket {
                data: vec![0xA1, UnitStat::Experience as u8, 0x88, 0x77, 0x66, 0x55, 5],
            },
            D2GSPacket {
                data: vec![
                    0xA2,
                    UnitStat::Experience as u8,
                    0x88,
                    0x77,
                    0x66,
                    0x55,
                    0x34,
                    0x12,
                ],
            },
            D2GSPacket {
                data: vec![0x9B, 0x34, 0x12, 0x78, 0x56, 0x00, 0x00],
            },
        ] {
            assert!(
                state
                    .apply_packet(&packet)
                    .expect("merc packet should parse")
            );
        }

        assert_eq!(
            state.player(7).expect("player exists").mercenary_id(),
            0x5566_7788
        );

        let merc = state.mercenary(0x5566_7788).expect("mercenary exists");
        assert_eq!(merc.owner_id(), 7);
        assert_eq!(merc.class_id(), 0x0152);
        assert_eq!(merc.skill_id(), 0x0A);
        assert_eq!(merc.stat(UnitStat::Level as u16), Some(90));
        assert_eq!(merc.stat(UnitStat::Experience as u16), Some(0x1239));
        assert_eq!(merc.revive_name_id(), Some(0x1234));
        assert_eq!(merc.revive_cost(), Some(0x5678));
    }

    #[test]
    fn player_corpse_packet_tracks_and_untracks_player_corpse_ids() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerCorpseAssign {
            assign: 1,
            owner_id: 0x1122_3344,
            corpse_id: 0x5566_7788,
        }));

        let corpse = state
            .player_corpse(0x5566_7788)
            .expect("player corpse should be tracked");
        assert_eq!(corpse.owner_id(), 0x1122_3344);
        assert_eq!(corpse.corpse_id(), 0x5566_7788);

        assert!(state.update(ServerMessage::PlayerCorpseAssign {
            assign: 0,
            owner_id: 0x1122_3344,
            corpse_id: 0x5566_7788,
        }));
        assert!(state.player_corpse(0x5566_7788).is_none());
    }

    #[test]
    fn state_packets_track_single_and_multi_state_payloads() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::SetState {
            unit_type: 1,
            unit_id: 55,
            packet_size: 11,
            state: 0x69,
            state_effects: vec![0xAA, 0xBB, 0xCC],
        }));
        assert!(state.update(ServerMessage::MultiStates {
            unit_type: 1,
            unit_id: 55,
            packet_size: 10,
            state_effects: vec![0x01, 0x02, 0x03],
        }));

        let states = state.unit_state(1, 55).expect("unit states should exist");
        assert_eq!(states.state_effects(0x69), Some(&[0xAA, 0xBB, 0xCC][..]));
        assert_eq!(states.multi_state_effects(), Some(&[0x01, 0x02, 0x03][..]));

        assert!(state.update(ServerMessage::EndState {
            unit_type: 1,
            unit_id: 55,
            state: 0x69,
        }));
        let states = state
            .unit_state(1, 55)
            .expect("multi-state payload should remain");
        assert_eq!(states.state_effects(0x69), None);
        assert_eq!(states.multi_state_effects(), Some(&[0x01, 0x02, 0x03][..]));
    }

    #[test]
    fn npc_zero_life_removes_npc_memory() {
        let mut state = GameState::default();
        state.update(ServerMessage::MonsterAssign {
            unit_id: 55,
            unit_code: 156,
            unit_x: 300,
            unit_y: 301,
            life_percent: 100,
            packet_size: 13,
            bitstream: Vec::new(),
        });

        assert!(state.update(ServerMessage::NpcStop {
            unit_id: 55,
            x: 320,
            y: 321,
            unit_life: 0,
        }));

        assert!(state.npc(55).is_none());
    }

    #[test]
    fn local_stat_and_experience_packets_update_local_player() {
        let mut state = GameState::default();
        state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Sorc\0\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 11,
        });
        mark_local(&mut state, 7);

        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::Strength as u8,
            amount: 50,
        }));
        assert!(state.update(ServerMessage::SetAttributeU32 {
            attribute: UnitStat::Experience as u8,
            amount: 10_000,
        }));
        assert!(state.update(ServerMessage::AddExpU16 { amount: 25 }));
        assert!(state.update(ServerMessage::AddExpU32 { amount: 1000 }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.stat(UnitStat::Strength as u16), Some(50));
        assert_eq!(player.stat(UnitStat::Experience as u16), Some(11_025));
    }

    #[test]
    fn player_quest_log_tracks_mandatory_and_optional_act_one_progress() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerQuestLogInfo {
            quest_bits: quest_log(&[
                (QuestLogEntry::ActIIntroduction, 1),
                (QuestLogEntry::DenOfEvil, completed_quest()),
                (QuestLogEntry::TheSearchForCain, completed_quest()),
                (QuestLogEntry::SistersToTheSlaughter, completed_quest()),
                (QuestLogEntry::TravelToActII, 1),
                (QuestLogEntry::ActIIIntroduction, 1),
            ]),
        }));

        let quest_log = state.player_quest_log().expect("quest log should exist");
        assert!(quest_log.entry(QuestLogEntry::DenOfEvil).is_completed());
        assert!(
            quest_log
                .entry(QuestLogEntry::SistersToTheSlaughter)
                .is_completed()
        );
        assert!(quest_log.entry(QuestLogEntry::TravelToActII).is_set());
        assert!(
            !quest_log
                .entry(QuestLogEntry::TheForgottenTower)
                .is_completed()
        );
    }

    #[test]
    fn player_quest_log_tracks_late_game_dependencies_separately_from_optional_quests() {
        let mut state = GameState::default();

        assert!(state.update(ServerMessage::PlayerQuestLogInfo {
            quest_bits: quest_log(&[
                (QuestLogEntry::TravelToActV, 1),
                (QuestLogEntry::PostTerrorsEndCain, 1),
                (QuestLogEntry::SiegeOnHarrogath, completed_quest()),
                (QuestLogEntry::RescueOnMountArreat, completed_quest()),
                (QuestLogEntry::RiteOfPassage, completed_quest()),
                (QuestLogEntry::EveOfDestruction, completed_quest()),
            ]),
        }));

        let quest_log = state.player_quest_log().expect("quest log should exist");
        assert!(quest_log.entry(QuestLogEntry::TravelToActV).is_set());
        assert!(quest_log.entry(QuestLogEntry::RiteOfPassage).is_completed());
        assert!(
            quest_log
                .entry(QuestLogEntry::EveOfDestruction)
                .is_completed()
        );
        assert!(!quest_log.entry(QuestLogEntry::PrisonOfIce).is_completed());
    }

    #[test]
    fn game_loading_clears_player_quest_log() {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::PlayerQuestLogInfo {
            quest_bits: quest_log(&[(QuestLogEntry::TheSummoner, completed_quest())]),
        }));
        assert!(state.player_quest_log().is_some());

        assert!(state.update(ServerMessage::GameLoading));
        assert!(state.player_quest_log().is_none());
    }

    fn mark_local(state: &mut GameState, unit_id: u32) {
        assert!(state.update(ServerMessage::GameHandshake {
            unit_type: 0,
            unit_id,
        }));
    }

    fn name16(name: &str) -> [u8; 16] {
        let mut bytes = [0; 16];
        let name = name.as_bytes();
        let len = name.len().min(bytes.len());
        bytes[..len].copy_from_slice(&name[..len]);
        bytes
    }

    fn completed_quest() -> u8 {
        QuestLogEntryState::COMPLETED_BIT | QuestLogEntryState::REQUIREMENT_COMPLETED_BIT
    }

    fn quest_log(entries: &[(QuestLogEntry, u8)]) -> [u8; 41] {
        let mut quest_bits = [0u8; 41];
        for (entry, value) in entries {
            quest_bits[entry.index()] = *value;
        }
        quest_bits
    }

    fn item_bitstream(
        flags: u32,
        version: u8,
        destination: ItemDestination,
        placement: ItemPlacement,
    ) -> Vec<u8> {
        let mut writer = TestBitWriter::default();
        writer.write_bits(flags, 32);
        writer.write_bits(version as u32, 8);
        writer.write_bits(0, 2);
        writer.write_bits(destination.packet_value() as u32, 3);
        match placement {
            ItemPlacement::Ground { x, y } => {
                writer.write_bits(x as u32, 16);
                writer.write_bits(y as u32, 16);
            }
            ItemPlacement::Container {
                equipment_location,
                x,
                y,
                container,
            } => {
                writer.write_bits(equipment_location as u32, 4);
                writer.write_bits(x as u32, 4);
                writer.write_bits(y as u32, 3);
                writer.write_bits(container as u32, 4);
            }
        }
        writer.finish()
    }

    fn status_bitfield<const N: usize>(fields: &[(u32, usize)]) -> [u8; N] {
        let mut writer = TestBitWriter::default();
        for &(value, count) in fields {
            writer.write_bits(value, count);
        }
        writer.finish().try_into().expect("bitfield size mismatch")
    }

    #[derive(Default)]
    struct TestBitWriter {
        bytes: Vec<u8>,
        bit_offset: usize,
    }

    impl TestBitWriter {
        fn write_bits(&mut self, value: u32, count: usize) {
            for index in 0..count {
                if self.bit_offset / 8 == self.bytes.len() {
                    self.bytes.push(0);
                }
                let bit = ((value >> index) & 1) as u8;
                self.bytes[self.bit_offset / 8] |= bit << (self.bit_offset % 8);
                self.bit_offset += 1;
            }
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }
}
