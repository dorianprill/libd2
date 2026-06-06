use std::collections::{HashMap, HashSet};

use crate::core::character_class::CharacterClass;
use crate::core::coordinate::Coordinate;
use crate::core::entity::npc::Npc;
use crate::core::entity::player::{Player, PlayerMovement, PlayerVitals};
use crate::core::network::d2gs::D2GSPacket;
use crate::core::object::item::{Item, ItemStateFlags};
use crate::core::object::WorldObject;
use crate::core::protocol::server_message::ServerMessageParseError;
use crate::core::unit_stat::UnitStat;
use crate::core::update::Update;
use crate::ServerMessage;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMapState {
    pub act: Option<u8>,
    pub map_id: Option<u32>,
    pub area_id: Option<u16>,
    pub automap: Option<u32>,
    pub revealed_tiles: HashSet<MapTile>,
}

impl Default for GameMapState {
    fn default() -> Self {
        Self {
            act: None,
            map_id: None,
            area_id: None,
            automap: None,
            revealed_tiles: HashSet::new(),
        }
    }
}

/// Raw server item-stat update captured from D2GS packet `0x3E`.
///
/// Public packet tables for legacy Diablo II expose `0x3E` as a declared-size
/// stat bitstream, with 1.14d adding fixed padding out to 34 bytes. They do not
/// expose a stable item GUID in the packet envelope. Until the item-stat
/// bitstream itself is decoded with `ItemStatCost` metadata, the library keeps
/// these events in arrival order instead of guessing which [`Item`] owns them.
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
    pub(crate) npcs: HashMap<u32, Npc>,
    pub(crate) objects: HashMap<u32, WorldObject>,
    pub(crate) items: HashMap<u32, Item>,
    pub(crate) item_stat_updates: Vec<ItemStatUpdate>,
    pub(crate) game_type: GameServerType,
    pub(crate) difficulty: Difficulty,
    pub(crate) locale: Locale,
    pub(crate) map: GameMapState,
    pub(crate) local_player_id: Option<u32>,
    pub(crate) is_expansion: bool,
    pub(crate) is_ladder: bool,
    pub(crate) is_hardcore: bool,
}

impl GameState {
    pub fn new(game_type: GameServerType, difficulty: Difficulty, locale: Locale) -> Self {
        Self {
            players: HashMap::with_capacity(8),
            player_aliases: HashMap::with_capacity(8),
            npcs: HashMap::with_capacity(1024),
            objects: HashMap::with_capacity(256),
            items: HashMap::with_capacity(256),
            item_stat_updates: Vec::with_capacity(64),
            game_type,
            difficulty,
            locale,
            map: GameMapState::default(),
            local_player_id: None,
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

    pub fn map(&self) -> &GameMapState {
        &self.map
    }

    pub fn local_player_id(&self) -> Option<u32> {
        self.local_player_id.map(|id| self.resolve_player_id(id))
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
        player.set_stat(stat, amount);
        true
    }

    fn set_player_stat(&mut self, unit_id: u32, stat: u16, amount: u32) -> bool {
        let unit_id = self.resolve_player_id(unit_id);
        let Some(player) = self.players.get_mut(&unit_id) else {
            return false;
        };
        player.set_stat(stat, amount);
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
            player.set_name(name);
            player.set_location(location);
            return;
        }

        self.player_aliases.remove(&unit_id);

        if let Some(existing_id) = self.find_player_id_by_identity(class, &name) {
            self.link_player_alias(unit_id, existing_id);
            if let Some(player) = self.players.get_mut(&existing_id) {
                player.set_class(class);
                player.set_name(name);
                player.set_location(location);
            }
            return;
        }

        self.players
            .insert(unit_id, Player::new(unit_id, class, name, location));
    }

    fn upsert_roster_player(
        &mut self,
        player_id: u32,
        class: CharacterClass,
        name: impl Into<String>,
        level: u32,
    ) {
        let name = name.into();
        let canonical_id = self.resolve_player_id(player_id);
        if let Some(player) = self.players.get_mut(&canonical_id) {
            player.set_class(class);
            player.set_name(name);
            player.set_level(level);
            return;
        }

        self.player_aliases.remove(&player_id);

        if let Some(existing_id) = self.find_player_id_by_identity(class, &name) {
            self.link_player_alias(player_id, existing_id);
            if let Some(player) = self.players.get_mut(&existing_id) {
                player.set_class(class);
                player.set_name(name);
                player.set_level(level);
            }
            return;
        }

        let mut player = Player::new_roster(player_id, class, name);
        player.set_level(level);
        self.players.insert(player_id, player);
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

    fn remove_unit(&mut self, unit_type: u8, unit_id: u32) -> bool {
        match unit_type {
            0x00 => self.clear_player_world_location(unit_id),
            0x01 => self.npcs.remove(&unit_id).is_some(),
            0x02 | 0x05 => self.objects.remove(&unit_id).is_some(),
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

        if local_removed {
            self.local_player_id = None;
        }

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
        self.objects.clear();
        self.items.clear();
        self.item_stat_updates.clear();
        self.map.revealed_tiles.clear();
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

    fn reset_session(&mut self) {
        self.players.clear();
        self.player_aliases.clear();
        self.npcs.clear();
        self.objects.clear();
        self.items.clear();
        self.item_stat_updates.clear();
        self.map = GameMapState::default();
        self.local_player_id = None;
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
                self.reset_session();
                true
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
            ServerMessage::PlayerJoined {
                player_id,
                character_class,
                character_name,
                character_level,
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
                );
                true
            }
            ServerMessage::PlayerLeft { player_id } => self.remove_player(player_id),
            ServerMessage::PlayerPartyInfo {
                unit_id,
                character_level,
                ..
            } => {
                let unit_id = self.resolve_player_id(unit_id);
                if let Some(player) = self.players.get_mut(&unit_id) {
                    player.set_level(character_level as u32);
                    true
                } else {
                    false
                }
            }
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
                if let Some(npc) = self.npcs.get_mut(&unit_id) {
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
                self.npcs.insert(
                    unit_id,
                    Npc::with_class(
                        unit_id,
                        unit_code,
                        Coordinate::new(unit_x, unit_y),
                        life_percent,
                    ),
                );
                true
            }
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
            ServerMessage::AddExpU32 { amount } => {
                self.set_local_stat(UnitStat::Experience as u16, amount)
            }
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
    use crate::core::entity::Entity;
    use crate::core::network::d2gs::D2GSPacket;
    use crate::core::unit_stat::UnitStat;
    use crate::core::update::Update;
    use crate::{
        CharacterClass, Difficulty, GameState, ItemDestination, ItemOwner, ItemPlacement,
        ServerMessage,
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
        assert!(!state
            .player(8)
            .expect("remote player exists")
            .world_location_known());
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
        assert!(state.update(ServerMessage::AddExpU16 { amount: 25 }));
        assert!(state.update(ServerMessage::AddExpU32 { amount: 1000 }));

        let player = state.player(7).expect("player exists");
        assert_eq!(player.stat(UnitStat::Strength as u16), Some(50));
        assert_eq!(player.stat(UnitStat::Experience as u16), Some(1000));
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
