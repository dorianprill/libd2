#[allow(dead_code)]
pub type ItemId = u32;

/// Action id from a D2GS item action packet.
///
/// The same packet family (`0x9C`/`0x9D`) is used for item creation, movement,
/// equipment changes, socketing, quantity changes, and stat refreshes. Keeping
/// the raw value matters because community packet tables disagree on some of
/// the higher action ids and mods may add more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    AddToGround,
    GroundToCursor,
    DropToGround,
    OnGround,
    PutInContainer,
    RemoveFromContainer,
    Equip,
    IndirectlySwapBodyItem,
    UnEquip,
    SwapBodyItem,
    AddQuantity,
    AddToShop,
    RemoveFromShop,
    SwapInContainer,
    PutInBelt,
    RemoveFromBelt,
    SwapInBelt,
    AutoUnEquip,
    RemoveFromHireling,
    ItemInSocket,
    UpdateStats,
    WeaponSwitch,
    Unknown(u8),
}

impl ItemAction {
    pub fn from_packet_value(value: u8) -> Self {
        match value {
            0x00 => Self::AddToGround,
            0x01 => Self::GroundToCursor,
            0x02 => Self::DropToGround,
            0x03 => Self::OnGround,
            0x04 => Self::PutInContainer,
            0x05 => Self::RemoveFromContainer,
            0x06 => Self::Equip,
            0x07 => Self::IndirectlySwapBodyItem,
            0x08 => Self::UnEquip,
            0x09 => Self::SwapBodyItem,
            0x0A => Self::AddQuantity,
            0x0B => Self::AddToShop,
            0x0C => Self::RemoveFromShop,
            0x0D => Self::SwapInContainer,
            0x0E => Self::PutInBelt,
            0x0F => Self::RemoveFromBelt,
            0x10 => Self::SwapInBelt,
            0x11 => Self::AutoUnEquip,
            0x12 => Self::RemoveFromHireling,
            0x13 => Self::ItemInSocket,
            0x15 => Self::UpdateStats,
            0x17 => Self::WeaponSwitch,
            other => Self::Unknown(other),
        }
    }

    pub fn packet_value(self) -> u8 {
        match self {
            Self::AddToGround => 0x00,
            Self::GroundToCursor => 0x01,
            Self::DropToGround => 0x02,
            Self::OnGround => 0x03,
            Self::PutInContainer => 0x04,
            Self::RemoveFromContainer => 0x05,
            Self::Equip => 0x06,
            Self::IndirectlySwapBodyItem => 0x07,
            Self::UnEquip => 0x08,
            Self::SwapBodyItem => 0x09,
            Self::AddQuantity => 0x0A,
            Self::AddToShop => 0x0B,
            Self::RemoveFromShop => 0x0C,
            Self::SwapInContainer => 0x0D,
            Self::PutInBelt => 0x0E,
            Self::RemoveFromBelt => 0x0F,
            Self::SwapInBelt => 0x10,
            Self::AutoUnEquip => 0x11,
            Self::RemoveFromHireling => 0x12,
            Self::ItemInSocket => 0x13,
            Self::UpdateStats => 0x15,
            Self::WeaponSwitch => 0x17,
            Self::Unknown(value) => value,
        }
    }
}

/// High-level item category from the D2GS item action envelope.
///
/// This category is not the three-letter item code. It is a coarse packet
/// category used by the client to decide which optional item fields are present,
/// such as armor defense or weapon durability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemCategory {
    Helm,
    Armor,
    Weapon,
    Weapon2,
    Shield,
    Special,
    Misc,
    Unknown(u8),
}

impl ItemCategory {
    pub fn from_packet_value(value: u8) -> Self {
        match value {
            0x00 => Self::Helm,
            0x01 => Self::Armor,
            0x05 => Self::Weapon,
            0x06 => Self::Weapon2,
            0x07 => Self::Shield,
            0x0A => Self::Special,
            0x10 => Self::Misc,
            other => Self::Unknown(other),
        }
    }

    pub fn packet_value(self) -> u8 {
        match self {
            Self::Helm => 0x00,
            Self::Armor => 0x01,
            Self::Weapon => 0x05,
            Self::Weapon2 => 0x06,
            Self::Shield => 0x07,
            Self::Special => 0x0A,
            Self::Misc => 0x10,
            Self::Unknown(value) => value,
        }
    }

    fn has_armor_defense(self) -> bool {
        self == Self::Armor
    }

    fn has_durability(self) -> bool {
        matches!(
            self,
            Self::Armor | Self::Weapon | Self::Weapon2 | Self::Shield
        )
    }
}

/// Current high-level owner of an item known from D2GS item action packets.
///
/// Diablo II uses the same item unit id while an item moves between the world,
/// the cursor, equipment, inventory-like grids, sockets, vendors, and other
/// unit-owned containers. Server packet `0x9C` describes a world item and packet
/// `0x9D` describes an item owned by another unit, most commonly the local
/// player. The owner alone is not enough to know the final UI slot; combine it
/// with [`ItemPacketData::destination`] and [`ItemPacketData::placement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemOwner {
    World,
    Unit { unit_type: u8, unit_id: u32 },
}

/// Latest raw item-state flags from D2GS packet `0x7D`.
///
/// The server sends this fixed packet with an owning unit id, an item GUID,
/// an `AndValue`, and the flags after applying that mask. Public packet tables
/// do not give enough semantic names for every bit, so the library stores the
/// raw values until item-state flag decoding is backed by static data and
/// fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStateFlags {
    unit_type: u8,
    unit_id: u32,
    and_value: u32,
    flags: u32,
}

impl ItemStateFlags {
    pub fn new(unit_type: u8, unit_id: u32, and_value: u32, flags: u32) -> Self {
        Self {
            unit_type,
            unit_id,
            and_value,
            flags,
        }
    }

    pub fn unit_type(&self) -> u8 {
        self.unit_type
    }

    pub fn unit_id(&self) -> u32 {
        self.unit_id
    }

    pub fn and_value(&self) -> u32 {
        self.and_value
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }
}

/// Destination field embedded in the D2GS item bitstream.
///
/// Reverse-engineered packet notes call this field "destination" rather than
/// "location" because an item action packet can describe both the source and
/// destination of a move. The destination is the stable part: it tells where the
/// item will end up after this action, while the container/x/y fields may refer
/// to the relevant grid or slot for that action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemDestination {
    Unspecified,
    Equipment,
    Belt,
    Ground,
    Cursor,
    Item,
    Unknown(u8),
}

impl ItemDestination {
    pub fn from_packet_value(value: u8) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Equipment,
            2 => Self::Belt,
            3 => Self::Ground,
            4 => Self::Cursor,
            6 => Self::Item,
            other => Self::Unknown(other),
        }
    }

    pub fn packet_value(self) -> u8 {
        match self {
            Self::Unspecified => 0,
            Self::Equipment => 1,
            Self::Belt => 2,
            Self::Ground => 3,
            Self::Cursor => 4,
            Self::Item => 6,
            Self::Unknown(value) => value,
        }
    }
}

/// Container id from non-ground item placement fields.
///
/// Container values identify UI/storage surfaces only after combining the item
/// action, destination, owner unit, and sometimes NPC transaction state. For
/// example, the same item packet shape is used for player inventory, cube,
/// stash, equipment, shop tabs, and socketed items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemContainer {
    Equipment,
    Ground,
    Inventory,
    TraderOffer,
    ForTrade,
    Cube,
    Stash,
    Belt,
    Item,
    ArmorTab,
    WeaponTab1,
    WeaponTab2,
    MiscTab,
    Unknown(u8),
}

impl ItemContainer {
    pub fn from_packet_value(value: u8) -> Self {
        match value {
            0x00 => Self::Equipment,
            0x01 => Self::Ground,
            0x02 => Self::Inventory,
            0x04 => Self::TraderOffer,
            0x06 => Self::ForTrade,
            0x08 => Self::Cube,
            0x0A => Self::Stash,
            0x0C => Self::Belt,
            0x0E => Self::Item,
            0x82 => Self::ArmorTab,
            0x84 => Self::WeaponTab1,
            0x86 => Self::WeaponTab2,
            0x88 => Self::MiscTab,
            other => Self::Unknown(other),
        }
    }

    pub fn packet_value(self) -> u8 {
        match self {
            Self::Equipment => 0x00,
            Self::Ground => 0x01,
            Self::Inventory => 0x02,
            Self::TraderOffer => 0x04,
            Self::ForTrade => 0x06,
            Self::Cube => 0x08,
            Self::Stash => 0x0A,
            Self::Belt => 0x0C,
            Self::Item => 0x0E,
            Self::ArmorTab => 0x82,
            Self::WeaponTab1 => 0x84,
            Self::WeaponTab2 => 0x86,
            Self::MiscTab => 0x88,
            Self::Unknown(value) => value,
        }
    }
}

/// Stable placement fields decoded from a D2GS item action bitstream.
///
/// Ground destinations carry full map coordinates. Other destinations carry the
/// compact equipment-location/x/y/container fields used by the client UI and by
/// inventory-like grids. Interpreting those container values into inventory,
/// stash, cube, belt, sockets, or vendor pages still depends on action type and
/// later item parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPlacement {
    Ground {
        x: u16,
        y: u16,
    },
    Container {
        equipment_location: u8,
        x: u8,
        y: u8,
        container: u8,
    },
}

impl ItemPlacement {
    pub fn container_kind(&self) -> Option<ItemContainer> {
        match self {
            Self::Ground { .. } => None,
            Self::Container { container, .. } => Some(ItemContainer::from_packet_value(*container)),
        }
    }
}

/// Bit flags carried before the item bitstream body.
///
/// These are packet-time item flags rather than save-file section markers. They
/// control which optional subfields are present: ears stop after placement,
/// simple/gamble items stop after code and used-socket count, identified items
/// expose quality-specific ids, and socketed/runeword/personalized items add
/// their own trailing fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFlags {
    bits: u32,
}

impl ItemFlags {
    pub const EQUIPPED: u32 = 0x0000_0001;
    pub const IN_SOCKET: u32 = 0x0000_0008;
    pub const IDENTIFIED: u32 = 0x0000_0010;
    pub const SWITCHED_IN: u32 = 0x0000_0040;
    pub const SWITCHED_OUT: u32 = 0x0000_0080;
    pub const BROKEN: u32 = 0x0000_0100;
    pub const POTION: u32 = 0x0000_0400;
    pub const SOCKETED: u32 = 0x0000_0800;
    pub const IN_STORE: u32 = 0x0000_2000;
    pub const NOT_IN_SOCKET: u32 = 0x0000_4000;
    pub const EAR: u32 = 0x0001_0000;
    pub const START_ITEM: u32 = 0x0002_0000;
    pub const SIMPLE_ITEM: u32 = 0x0020_0000;
    pub const ETHEREAL: u32 = 0x0040_0000;
    pub const ANY: u32 = 0x0080_0000;
    pub const PERSONALIZED: u32 = 0x0100_0000;
    pub const GAMBLE: u32 = 0x0200_0000;
    pub const RUNEWORD: u32 = 0x0400_0000;

    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub fn bits(self) -> u32 {
        self.bits
    }

    pub fn contains(self, mask: u32) -> bool {
        self.bits & mask != 0
    }

    pub fn is_identified(self) -> bool {
        self.contains(Self::IDENTIFIED)
    }

    pub fn is_socketed(self) -> bool {
        self.contains(Self::SOCKETED)
    }

    pub fn is_ear(self) -> bool {
        self.contains(Self::EAR)
    }

    pub fn is_simple_item(self) -> bool {
        self.contains(Self::SIMPLE_ITEM)
    }

    pub fn is_gamble(self) -> bool {
        self.contains(Self::GAMBLE)
    }

    pub fn is_personalized(self) -> bool {
        self.contains(Self::PERSONALIZED)
    }

    pub fn is_runeword(self) -> bool {
        self.contains(Self::RUNEWORD)
    }
}

/// Three-letter item code from `weapons.txt`, `armor.txt`, or `misc.txt`.
///
/// The packet field is four bytes because many classic item codes are stored as
/// three ASCII characters plus a trailing space. The trimmed code is exposed for
/// lookup while the raw bytes are kept for fixtures and unusual mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemCode {
    raw: [u8; 4],
    code: String,
}

impl ItemCode {
    pub fn from_raw(raw: [u8; 4]) -> Option<Self> {
        let end = raw
            .iter()
            .rposition(|byte| *byte != b' ' && *byte != 0)
            .map(|index| index + 1)
            .unwrap_or(0);
        let code = std::str::from_utf8(&raw[..end]).ok()?.to_owned();
        Some(Self { raw, code })
    }

    pub fn as_str(&self) -> &str {
        &self.code
    }

    pub fn raw(&self) -> [u8; 4] {
        self.raw
    }
}

/// Item quality id from the full item bitstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemQuality {
    NotApplicable,
    Inferior,
    Normal,
    Superior,
    Magic,
    Set,
    Rare,
    Unique,
    Crafted,
    Unknown(u8),
}

impl ItemQuality {
    pub fn from_packet_value(value: u8) -> Self {
        match value {
            0 => Self::NotApplicable,
            1 => Self::Inferior,
            2 => Self::Normal,
            3 => Self::Superior,
            4 => Self::Magic,
            5 => Self::Set,
            6 => Self::Rare,
            7 => Self::Unique,
            8 => Self::Crafted,
            other => Self::Unknown(other),
        }
    }

    pub fn packet_value(self) -> u8 {
        match self {
            Self::NotApplicable => 0,
            Self::Inferior => 1,
            Self::Normal => 2,
            Self::Superior => 3,
            Self::Magic => 4,
            Self::Set => 5,
            Self::Rare => 6,
            Self::Unique => 7,
            Self::Crafted => 8,
            Self::Unknown(value) => value,
        }
    }
}

/// Maximum and current durability decoded for armor/weapon-like categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDurability {
    pub max: u8,
    pub current: u8,
}

/// Runeword id pair carried by identified runeword item packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRuneword {
    pub id: u16,
    pub parameter: u8,
}

/// Optional prefix/suffix pair on rare and crafted item packets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ItemAffixPair {
    pub prefix: Option<u16>,
    pub suffix: Option<u16>,
}

/// First stable fields of a `0x9C`/`0x9D` item action bitstream.
///
/// Diablo II item packets are bit-packed after the byte-aligned D2GS envelope.
/// This parser decodes the fields whose presence can be determined from packet
/// flags and coarse item category alone: placement, code, gold amount,
/// used/open sockets, item level, quality, graphic/color ids, quality-specific
/// ids, defense, durability, and runeword metadata. It intentionally does not
/// decode the final stat list yet because that requires MPQ/TXT data such as
/// `ItemStatCost.txt` to know each stat's save-bit width and parameter rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPacketData {
    pub flags: ItemFlags,
    pub version: u8,
    pub unknown_prefix_bits: u8,
    pub destination: ItemDestination,
    pub placement: ItemPlacement,
    pub code: Option<ItemCode>,
    pub gold_amount: Option<u32>,
    pub sockets_used: Option<u8>,
    pub level: Option<u8>,
    pub quality: Option<ItemQuality>,
    pub graphic: Option<u8>,
    pub color: Option<u16>,
    pub quality_modifier: Option<u8>,
    pub magic_prefix: Option<u16>,
    pub magic_suffix: Option<u16>,
    pub rare_name: Option<[u8; 2]>,
    pub rare_affixes: [ItemAffixPair; 3],
    pub code_extra: Option<u16>,
    pub runeword: Option<ItemRuneword>,
    pub personalized_name: Option<String>,
    pub defense: Option<u16>,
    pub durability: Option<ItemDurability>,
    pub sockets: Option<u8>,
}

impl ItemPacketData {
    const FLAGS_BITS: usize = 32;
    const VERSION_BITS: usize = 8;
    const UNKNOWN_BITS: usize = 2;
    const DESTINATION_BITS: usize = 3;
    const GROUND_COORD_BITS: usize = 16;
    const EQUIPMENT_LOCATION_BITS: usize = 4;
    const CONTAINER_X_BITS: usize = 4;
    const CONTAINER_Y_BITS: usize = 3;
    const CONTAINER_BITS: usize = 4;
    const ITEM_CODE_BYTES: usize = 4;
    const USED_SOCKET_BITS: usize = 3;
    const ITEM_LEVEL_BITS: usize = 7;
    const QUALITY_BITS: usize = 4;
    const GRAPHIC_BITS: usize = 3;
    const COLOR_BITS: usize = 11;
    const QUALITY_MODIFIER_BITS: usize = 3;
    const MAGIC_AFFIX_BITS: usize = 11;
    const UNIQUE_SET_CODE_EXTRA_BITS: usize = 12;
    const RARE_NAME_BITS: usize = 8;
    const DEFENSE_BITS: usize = 11;
    const DURABILITY_BITS: usize = 8;
    const SOCKETS_BITS: usize = 4;
    const GOLD_SMALL_AMOUNT_BITS: usize = 12;
    const GOLD_LARGE_AMOUNT_BITS: usize = 32;
    const RUNEWORD_ID_BITS: usize = 12;
    const RUNEWORD_PARAMETER_BITS: usize = 4;

    /// Parses the generic prefix of a D2GS item bitstream.
    ///
    /// Bits are read least-significant-bit first, matching the bit ordering used
    /// by the Diablo II save and item formats. Returns `None` when the packet
    /// ends before the placement fields for its destination.
    pub fn parse(bitstream: &[u8]) -> Option<Self> {
        Self::parse_with_category(bitstream, ItemCategory::Unknown(0))
    }

    /// Parses a D2GS item bitstream with the packet-envelope category.
    ///
    /// The category is needed for the few fields that are not self-describing in
    /// the bitstream. Armor packets carry defense, and armor/weapon-like
    /// packets carry durability. Unknown categories still parse all fields up to
    /// that category-dependent tail.
    pub fn parse_with_category(bitstream: &[u8], category: ItemCategory) -> Option<Self> {
        let mut reader = ItemBitReader::new(bitstream);
        let flags = ItemFlags::from_bits(reader.bits(Self::FLAGS_BITS)?);
        let version = reader.bits(Self::VERSION_BITS)? as u8;
        let unknown_prefix_bits = reader.bits(Self::UNKNOWN_BITS)? as u8;
        let destination =
            ItemDestination::from_packet_value(reader.bits(Self::DESTINATION_BITS)? as u8);

        let placement = if destination == ItemDestination::Ground {
            let x = reader.bits(Self::GROUND_COORD_BITS)? as u16;
            let y = reader.bits(Self::GROUND_COORD_BITS)? as u16;
            ItemPlacement::Ground { x, y }
        } else {
            let equipment_location = reader.bits(Self::EQUIPMENT_LOCATION_BITS)? as u8;
            let x = reader.bits(Self::CONTAINER_X_BITS)? as u8;
            let y = reader.bits(Self::CONTAINER_Y_BITS)? as u8;
            let container = reader.bits(Self::CONTAINER_BITS)? as u8;
            ItemPlacement::Container {
                equipment_location,
                x,
                y,
                container,
            }
        };

        let mut parsed = Self {
            flags,
            version,
            unknown_prefix_bits,
            destination,
            placement,
            code: None,
            gold_amount: None,
            sockets_used: None,
            level: None,
            quality: None,
            graphic: None,
            color: None,
            quality_modifier: None,
            magic_prefix: None,
            magic_suffix: None,
            rare_name: None,
            rare_affixes: [ItemAffixPair::default(); 3],
            code_extra: None,
            runeword: None,
            personalized_name: None,
            defense: None,
            durability: None,
            sockets: None,
        };

        if flags.is_ear() {
            return Some(parsed);
        }

        let Some(raw_code) = reader.fixed_string::<{ Self::ITEM_CODE_BYTES }>() else {
            return Some(parsed);
        };
        let Some(code) = ItemCode::from_raw(raw_code) else {
            return Some(parsed);
        };
        let is_gold = code.as_str() == "gld";
        parsed.code = Some(code);

        if is_gold {
            let Some(uses_large_amount) = reader.bool() else {
                return Some(parsed);
            };
            let amount_bits = if uses_large_amount {
                Self::GOLD_LARGE_AMOUNT_BITS
            } else {
                Self::GOLD_SMALL_AMOUNT_BITS
            };
            parsed.gold_amount = reader.bits(amount_bits);
            return Some(parsed);
        }

        let Some(sockets_used) = reader.bits(Self::USED_SOCKET_BITS).map(|value| value as u8)
        else {
            return Some(parsed);
        };
        if sockets_used > 0 {
            parsed.sockets_used = Some(sockets_used);
        }

        if flags.is_simple_item() || flags.is_gamble() {
            return Some(parsed);
        }

        let Some(level) = reader.bits(Self::ITEM_LEVEL_BITS).map(|value| value as u8) else {
            return Some(parsed);
        };
        parsed.level = Some(level);

        let Some(quality) = reader
            .bits(Self::QUALITY_BITS)
            .map(|value| ItemQuality::from_packet_value(value as u8))
        else {
            return Some(parsed);
        };
        parsed.quality = Some(quality);

        let Some(has_graphic) = reader.bool() else {
            return Some(parsed);
        };
        if has_graphic {
            parsed.graphic = reader.bits(Self::GRAPHIC_BITS).map(|value| value as u8);
            if parsed.graphic.is_none() {
                return Some(parsed);
            }
        }

        let Some(has_color) = reader.bool() else {
            return Some(parsed);
        };
        if has_color {
            parsed.color = reader.bits(Self::COLOR_BITS).map(|value| value as u16);
            if parsed.color.is_none() {
                return Some(parsed);
            }
        }

        if flags.is_identified() {
            parsed.parse_identified_tail(&mut reader, category);
        }

        Some(parsed)
    }

    fn parse_identified_tail(&mut self, reader: &mut ItemBitReader<'_>, category: ItemCategory) {
        let Some(quality) = self.quality else {
            return;
        };

        match quality {
            ItemQuality::Unique => {
                if self.code.as_ref().is_none_or(|code| code.as_str() != "std") {
                    self.code_extra = reader
                        .bits(Self::UNIQUE_SET_CODE_EXTRA_BITS)
                        .map(|value| value as u16);
                }
                return;
            }
            ItemQuality::Inferior | ItemQuality::Superior => {
                self.quality_modifier = reader
                    .bits(Self::QUALITY_MODIFIER_BITS)
                    .map(|value| value as u8);
                if self.quality_modifier.is_none() {
                    return;
                }
            }
            ItemQuality::Magic => {
                self.magic_prefix = reader
                    .bits(Self::MAGIC_AFFIX_BITS)
                    .map(|value| value as u16);
                self.magic_suffix = reader
                    .bits(Self::MAGIC_AFFIX_BITS)
                    .map(|value| value as u16);
                if self.magic_prefix.is_none() || self.magic_suffix.is_none() {
                    return;
                }
            }
            ItemQuality::Crafted | ItemQuality::Rare => {
                let Some(first_name) = reader.bits(Self::RARE_NAME_BITS).map(|value| value as u8)
                else {
                    return;
                };
                let Some(second_name) = reader.bits(Self::RARE_NAME_BITS).map(|value| value as u8)
                else {
                    return;
                };
                self.rare_name = Some([first_name, second_name]);

                for affix in &mut self.rare_affixes {
                    let Some(has_prefix) = reader.bool() else {
                        return;
                    };
                    if has_prefix {
                        affix.prefix = reader
                            .bits(Self::MAGIC_AFFIX_BITS)
                            .map(|value| value as u16);
                        if affix.prefix.is_none() {
                            return;
                        }
                    }

                    let Some(has_suffix) = reader.bool() else {
                        return;
                    };
                    if has_suffix {
                        affix.suffix = reader
                            .bits(Self::MAGIC_AFFIX_BITS)
                            .map(|value| value as u16);
                        if affix.suffix.is_none() {
                            return;
                        }
                    }
                }
            }
            ItemQuality::Set => {
                self.code_extra = reader
                    .bits(Self::UNIQUE_SET_CODE_EXTRA_BITS)
                    .map(|value| value as u16);
                if self.code_extra.is_none() {
                    return;
                }
            }
            ItemQuality::Normal | ItemQuality::NotApplicable | ItemQuality::Unknown(_) => {}
        }

        if self.flags.is_runeword() {
            let Some(id) = reader
                .bits(Self::RUNEWORD_ID_BITS)
                .map(|value| value as u16)
            else {
                return;
            };
            let Some(parameter) = reader
                .bits(Self::RUNEWORD_PARAMETER_BITS)
                .map(|value| value as u8)
            else {
                return;
            };
            self.runeword = Some(ItemRuneword { id, parameter });
        }

        if self.flags.is_personalized() {
            let Some(name) = reader.zero_terminated_string() else {
                return;
            };
            self.personalized_name = Some(name);
        }

        if category.has_armor_defense() {
            let Some(raw_defense) = reader.bits(Self::DEFENSE_BITS) else {
                return;
            };
            self.defense = Some(raw_defense.saturating_sub(10) as u16);
        }

        if category.has_durability() {
            let Some(max) = reader.bits(Self::DURABILITY_BITS).map(|value| value as u8) else {
                return;
            };
            let Some(current) = reader.bits(Self::DURABILITY_BITS).map(|value| value as u8) else {
                return;
            };
            let Some(_) = reader.bool() else {
                return;
            };
            self.durability = Some(ItemDurability { max, current });
        }

        if self.flags.is_socketed() {
            self.sockets = reader.bits(Self::SOCKETS_BITS).map(|value| value as u8);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    id: ItemId,
    action: u8,
    category: u8,
    owner: ItemOwner,
    packet_data: Option<ItemPacketData>,
    raw_bitstream: Vec<u8>,
    state_flags: Option<ItemStateFlags>,
}

impl Item {
    pub fn new(id: ItemId) -> Self {
        Self {
            id,
            action: 0,
            category: 0,
            owner: ItemOwner::World,
            packet_data: None,
            raw_bitstream: Vec::new(),
            state_flags: None,
        }
    }

    pub fn from_world_packet(id: ItemId, action: u8, category: u8, bitstream: Vec<u8>) -> Self {
        Self::from_packet(id, action, category, ItemOwner::World, bitstream)
    }

    pub fn from_owned_packet(
        id: ItemId,
        action: u8,
        category: u8,
        owner_type: u8,
        owner_id: u32,
        bitstream: Vec<u8>,
    ) -> Self {
        Self::from_packet(
            id,
            action,
            category,
            ItemOwner::Unit {
                unit_type: owner_type,
                unit_id: owner_id,
            },
            bitstream,
        )
    }

    fn from_packet(
        id: ItemId,
        action: u8,
        category: u8,
        owner: ItemOwner,
        bitstream: Vec<u8>,
    ) -> Self {
        let packet_data = ItemPacketData::parse_with_category(
            &bitstream,
            ItemCategory::from_packet_value(category),
        );
        Self {
            id,
            action,
            category,
            owner,
            packet_data,
            raw_bitstream: bitstream,
            state_flags: None,
        }
    }

    pub fn id(&self) -> ItemId {
        self.id
    }

    pub fn action(&self) -> u8 {
        self.action
    }

    pub fn action_kind(&self) -> ItemAction {
        ItemAction::from_packet_value(self.action)
    }

    pub fn category(&self) -> u8 {
        self.category
    }

    pub fn category_kind(&self) -> ItemCategory {
        ItemCategory::from_packet_value(self.category)
    }

    pub fn owner(&self) -> ItemOwner {
        self.owner
    }

    pub fn packet_data(&self) -> Option<&ItemPacketData> {
        self.packet_data.as_ref()
    }

    pub fn raw_bitstream(&self) -> &[u8] {
        &self.raw_bitstream
    }

    pub fn state_flags(&self) -> Option<ItemStateFlags> {
        self.state_flags
    }

    pub fn set_state_flags(&mut self, state_flags: ItemStateFlags) {
        self.state_flags = Some(state_flags);
    }
}

pub enum MercenaryItemSlot {
    // TODO unknown at this time - from bnetdocs:
    // These values have been recorded for mercenary body locations, but aren't confirmed:
    // (Note, each location ID is prefixed with 0x61)
    // Example: 1A 64 00 00 00 61 02 00 00 (Move item 0x64 to Mercenary Right-hand weapon)
}

#[repr(u8)]
pub enum ItemBufferId {
    CharacterInventory = 0x00,
    NpcVendor = 0x01,
    TradeWindow = 0x02,
    HoradricCube = 0x03,
    Stash = 0x04,
}

pub type ItemBufferCoord = u32;

fn read_bits(raw: &[u8], bit_offset: usize, count: usize) -> Option<u32> {
    if count > 32 || bit_offset.checked_add(count)? > raw.len() * 8 {
        return None;
    }

    let mut value = 0;
    for index in 0..count {
        let position = bit_offset + index;
        let bit = (raw[position / 8] >> (position % 8)) & 1;
        value |= (bit as u32) << index;
    }

    Some(value)
}

struct ItemBitReader<'a> {
    raw: &'a [u8],
    bit_offset: usize,
}

impl<'a> ItemBitReader<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { raw, bit_offset: 0 }
    }

    fn bits(&mut self, count: usize) -> Option<u32> {
        let value = read_bits(self.raw, self.bit_offset, count)?;
        self.bit_offset += count;
        Some(value)
    }

    fn bool(&mut self) -> Option<bool> {
        self.bits(1).map(|value| value != 0)
    }

    fn fixed_string<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut bytes = [0; N];
        for byte in &mut bytes {
            *byte = self.bits(8)? as u8;
        }
        Some(bytes)
    }

    fn zero_terminated_string(&mut self) -> Option<String> {
        let mut bytes = Vec::new();
        while self.bit_offset + 8 <= self.raw.len() * 8 {
            let byte = self.bits(8)? as u8;
            if byte == 0 {
                return String::from_utf8(bytes).ok();
            }
            bytes.push(byte);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Item, ItemAction, ItemCategory, ItemCode, ItemDestination, ItemDurability, ItemFlags,
        ItemOwner, ItemPacketData, ItemPlacement, ItemPlacement::Ground, ItemQuality,
    };

    #[test]
    fn parses_ground_item_packet_prefix() {
        let bitstream = item_bitstream(
            0x1122_3344,
            0x65,
            ItemDestination::Ground,
            Ground { x: 321, y: 654 },
        );

        let parsed = ItemPacketData::parse(&bitstream).expect("bitstream should parse");

        assert_eq!(parsed.flags.bits(), 0x1122_3344);
        assert_eq!(parsed.version, 0x65);
        assert_eq!(parsed.unknown_prefix_bits, 0);
        assert_eq!(parsed.destination, ItemDestination::Ground);
        assert_eq!(parsed.placement, Ground { x: 321, y: 654 });
    }

    #[test]
    fn parses_container_item_packet_prefix() {
        let placement = ItemPlacement::Container {
            equipment_location: 2,
            x: 9,
            y: 3,
            container: 5,
        };
        let bitstream = item_bitstream(0, 0x60, ItemDestination::Cursor, placement);

        let parsed = ItemPacketData::parse(&bitstream).expect("bitstream should parse");

        assert_eq!(parsed.destination, ItemDestination::Cursor);
        assert_eq!(parsed.placement, placement);
        assert_eq!(
            parsed.placement.container_kind(),
            Some(super::ItemContainer::Unknown(5))
        );
    }

    #[test]
    fn item_packets_preserve_owner_and_raw_bits() {
        let bitstream = item_bitstream(0, 0x60, ItemDestination::Ground, Ground { x: 10, y: 11 });
        let item = Item::from_owned_packet(7, 0x01, 0x04, 0, 99, bitstream.clone());

        assert_eq!(item.id(), 7);
        assert_eq!(item.action(), 0x01);
        assert_eq!(item.action_kind(), ItemAction::GroundToCursor);
        assert_eq!(item.category(), 0x04);
        assert_eq!(item.category_kind(), ItemCategory::Unknown(0x04));
        assert_eq!(
            item.owner(),
            ItemOwner::Unit {
                unit_type: 0,
                unit_id: 99,
            }
        );
        assert_eq!(item.raw_bitstream(), bitstream.as_slice());
        assert_eq!(
            item.packet_data().expect("prefix should parse").placement,
            Ground { x: 10, y: 11 }
        );
    }

    #[test]
    fn parses_fixture_item_code_quality_and_ground_location() {
        let packet = decode_hex("9c021e056800000010008000658c24c2bfe2664c0e04d02100111e3cf80f");
        let item = Item::from_world_packet(
            104,
            packet[1],
            packet[3],
            packet[8..packet[2] as usize].to_vec(),
        );
        let data = item.packet_data().expect("fixture item should parse");

        assert_eq!(item.action_kind(), ItemAction::DropToGround);
        assert_eq!(item.category_kind(), ItemCategory::Weapon);
        assert_eq!(data.version, 101);
        assert_eq!(data.destination, ItemDestination::Ground);
        assert_eq!(data.placement, Ground { x: 4388, y: 5630 });
        assert_eq!(data.code.as_ref().map(ItemCode::as_str), Some("7cr"));
        assert_eq!(data.level, Some(80));
        assert_eq!(data.quality, Some(ItemQuality::Superior));
    }

    #[test]
    fn parses_fixture_armor_defense_durability_and_sockets() {
        let packet = decode_hex("9d052001290000000001000000100880006510a05a37c60602a9001c120ef21f");
        let item = Item::from_owned_packet(
            0x29,
            packet[1],
            packet[3],
            packet[8],
            u32::from_le_bytes(packet[9..13].try_into().unwrap()),
            packet[13..packet[2] as usize].to_vec(),
        );
        let data = item.packet_data().expect("fixture item should parse");

        assert_eq!(item.action_kind(), ItemAction::RemoveFromContainer);
        assert_eq!(item.category_kind(), ItemCategory::Armor);
        assert_eq!(data.code.as_ref().map(ItemCode::as_str), Some("ucl"));
        assert_eq!(data.quality, Some(ItemQuality::Normal));
        assert_eq!(data.defense, Some(438));
        assert_eq!(
            data.durability,
            Some(ItemDurability {
                max: 36,
                current: 28,
            })
        );
        assert_eq!(data.sockets, Some(2));
        assert_eq!(data.sockets_used, None);
    }

    #[test]
    fn parses_gold_amount_without_static_data() {
        let mut writer = TestBitWriter::default();
        writer.write_bits(ItemFlags::SIMPLE_ITEM, 32);
        writer.write_bits(0x60, 8);
        writer.write_bits(0, 2);
        writer.write_bits(ItemDestination::Ground.packet_value() as u32, 3);
        writer.write_bits(20, 16);
        writer.write_bits(21, 16);
        writer.write_ascii4(*b"gld ");
        writer.write_bits(0, 1);
        writer.write_bits(4095, 12);

        let parsed = ItemPacketData::parse(&writer.finish()).expect("gold should parse");

        assert_eq!(parsed.code.as_ref().map(ItemCode::as_str), Some("gld"));
        assert_eq!(parsed.gold_amount, Some(4095));
        assert_eq!(parsed.quality, None);
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

    fn decode_hex(input: &str) -> Vec<u8> {
        assert_eq!(input.len() % 2, 0);
        input
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                let high = hex_value(pair[0]);
                let low = hex_value(pair[1]);
                (high << 4) | low
            })
            .collect()
    }

    fn hex_value(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("invalid hex byte"),
        }
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

        fn write_ascii4(&mut self, value: [u8; 4]) {
            for byte in value {
                self.write_bits(byte as u32, 8);
            }
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }
}
