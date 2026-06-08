/// D2 Game Server Message Identifiers
/// I've merged infos from several sources on this:
/// blizzhackers: https://github.com/blizzhackers/Diablo2PacketsData/blob/main/src/data/1.14d/gs2client.json
/// MephisTools: https://github.com/MephisTools/diablo2-protocol/blob/master/data/1.14/d2gs.json
/// ServerMessage (Server->Client) is determined by the first byte of a D2GSPacket's data (enum value here)
use crate::core::network::d2gs::D2GSPacket;

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ServerMessage {
    GameLoading = 0x00,

    /// eArenaFlags contains information about game settings.
    /// eArenaFlags & 0x00000004 = Unknown (Always set)
    /// eArenaFlags & 0x00000800 = Hardcore
    /// eArenaFlags & 0x00100000 = Expansion
    GameFlags {
        difficulty: u8,
        arena_flags: u32,
        is_expansion: u8,
        is_ladder: u8,
    } = 0x01,

    LoadSuccessful = 0x02,

    LoadAct {
        act: u8,
        map_id: u32,
        area_id: u16,
        automap: u32,
    } = 0x03,

    LoadComplete = 0x4,

    UnloadComplete = 0x5,

    GameExitSuccessful = 0x06,

    MapReveal {
        tile_x: u16,
        tile_y: u16,
        area_id: u8,
    } = 0x07,

    MapHide {
        tile_x: u16,
        tile_y: u16,
        area_id: u8,
    } = 0x08,

    AssignLevelWarp {
        unit_type: u8,
        unit_id: u32,
        warp_class_id: u8,
        warp_x: u16,
        warp_y: u16,
    } = 0x09,

    RemoveObject {
        unit_type: u8,
        unit_id: u32,
    } = 0x0A,

    GameHandshake {
        unit_type: u8,
        unit_id: u32,
    } = 0x0B,

    NpcHit {
        unit_type: u8,
        unit_id: u32,
        animation_id: u16, // FIXME blizzhackers reports this as two bytes nUnitHitType and nUnitHitClass
        alive: u8,
    } = 0x0C,

    PlayerStop {
        unit_type: u8,
        unit_id: u32,
        hit_class: u8,
        x: u16,
        y: u16,
        unit_hit_class: u8,
        alive: u8,
    } = 0x0D,

    ObjectState {
        unit_type: u8,
        unit_id: u32,
        portal_flags: u8,
        is_targetable: u8, // TODO is this flags or bool?
        unit_state: u32,
    } = 0x0E,

    PlayerMove {
        unit_type: u8,
        unit_id: u32,
        move_type: u8,
        target_x: u16,
        target_y: u16,
        unit_hit_class: u8,
        current_x: u16,
        current_y: u16,
    } = 0x0F,

    CharToObject {
        player_type: u8,
        player_id: u32,
        movement_type: u8,
        target_type: u8,
        target_id: u32,
        target_x: u16,
        target_y: u16,
    } = 0x10,

    ReportKill {
        unit_type: u8,
        unit_id: u32,
        unknown: u16,
    } = 0x11,

    Unknown1 {
        unknown: [u8; 25],
    } = 0x12,

    Unknown2 {
        unknown: [u8; 13],
    } = 0x13,

    Unknown3 {
        unknown: [u8; 17],
    } = 0x14,

    ReassignPlayer {
        unit_type: u8,
        unit_id: u32,
        x: u16,
        y: u16,
        value: u8, // TODO is this a boolean?
    } = 0x15,

    MultipleUnitsCoordsUpdate {
        unused_1: u8,
        unused_2: u8,
        count: u8,
        // then follows an array of units
        // { "sUnitInfo[count]" : [
        //			{ "BYTE" : "nUnitType" },
        //			{ "int" : "nUnitGUID" },
        //			{ "short" : "nUnitX" },
        //			{ "short" : "nUnitY" }
        //		]
        //	}
        // TODO how to represent?
    } = 0x16,

    Unused1 = 0x17,

    /// Local-player life/mana/stamina, regeneration counters, and movement.
    ///
    /// This is the larger HP/MP bitstream variant: 15-bit HP, 15-bit MP,
    /// 15-bit stamina, 7-bit HP regeneration, 7-bit MP regeneration, 16-bit X,
    /// 16-bit Y, and two trailing 8-bit movement verification fields.
    HPMPUPDATE {
        packed_bits: [u8; 14],
    } = 0x18,

    SmallGoldPickup {
        amount: u8,
    } = 0x19,

    AddExpU8 {
        amount: u8,
    } = 0x1A,

    AddExpU16 {
        amount: u16,
    } = 0x1B,

    AddExpU32 {
        amount: u32,
    } = 0x1C,

    SetAttributeU8 {
        attribute: u8,
        amount: u8,
    } = 0x1D,

    SetAttributeU16 {
        attribute: u8,
        amount: u16,
    } = 0x1E,

    SetAttributeU32 {
        attribute: u8,
        amount: u32,
    } = 0x1F,

    AttributeUpdate {
        unit_id: u32,
        attribute: u8,
        amount: u32,
    } = 0x20,

    UpdateItemOSkill {
        unknown1: u16,
        unit_id: u32,
        skill: u16,
        base_level: u8,
        bonus_level: u8,
        unknown2: u8, // FIXME blizzhackers reports this as 'padding'
    } = 0x21,

    UpdateItemSkill {
        unknown1: u16,
        unit_id: u32,
        skill: u16,
        amount: u8,
        unknown2: u16, // FIXME blizzhackers reports his as two u8's: Padding, bBody
    } = 0x22,

    SetSkill {
        unit_type: u8, // FIXME apparently unused according to blizzhackers
        unit_id: u32,
        hand: u8,
        skill_id: u16,
        item_id: u32, // FIXME naming according to blizzhackers. maybe signals if skill is from item?
    } = 0x23,

    Unknown6 {
        unknown: [u8; 89],
    } = 0x24,

    Unknown7 {
        unknown: [u8; 89],
    } = 0x25,

    GameChat {
        chat_kind: u16,
        unknown: u16,
        unknown2: u32,
        chat_type: u8,
        char_name: [u8; 255], // these are C-Strings but..
        message: [u8; 255],   // FIXME handle as &String instead?
    } = 0x26,

    NpcInfo {
        unit_type: u8,
        unit_id: u32,
        count: u8, // TODO is this fixed to value 8?
        unknown: u8,
        unknown2: [u8; 32], // array structure according to blizzhackers:
                            //{ "sUnitMessages[8]" : [
                            //			{ "BYTE" : "bShow" },
                            //			{ "BYTE" : "NotUsed" },
                            //			{ "short" : "nMsgID" }
                            //		]
                            //	}
    } = 0x27,

    PlayerQuestInfo {
        update_type: u8,
        unit_id: u32,
        action_type: u8,
        quest_bits: [u8; 96],
    } = 0x28,

    GameQuestInfo {
        unknown: [u8; 96],
    } = 0x29,

    NpcTransaction {
        trade_type: u8,
        result: u8,
        unused: u32,
        merchandise_id: u32,
        inventory_gold: u32,
    } = 0x2A,

    Unused2 = 0x2B,

    PlaySound {
        unit_type: u8,
        unit_id: u32,
        sound_id: u16,
    } = 0x2C,

    Unused3 = 0x2D,
    Unused4 = 0x2E,
    Unused5 = 0x2F,
    Unused6 = 0x30,
    Unused7 = 0x31,
    Unused8 = 0x32,
    Unused9 = 0x33,
    Unused10 = 0x34,
    Unused11 = 0x35,
    Unused12 = 0x36,
    Unused13 = 0x37,
    Unused14 = 0x38,
    Unused15 = 0x39,
    Unused16 = 0x3A,
    Unused17 = 0x3B,
    Unused18 = 0x3C,
    Unused19 = 0x3D,

    /// Item-stat bitstream update.
    ///
    /// Legacy packet tables disagree on framing details: 1.13c/1.15 list this
    /// as variable-length, while 1.14d lists a 34-byte packet whose declared
    /// `nFullPacketSize` bytes are followed by padding. In both forms the
    /// stable envelope is only packet id plus declared size; item identity and
    /// stat-list semantics must be decoded from the bitstream later.
    UpdateItemStats {
        packet_size: u8,
        bitstream: Vec<u8>,
    } = 0x3E,

    UseStackableItem {
        spell_icon: u8,
        item_id: u32,
        skill_id: u16,
    } = 0x3F,

    SetItemFlags {
        unit_id: u32,
        item_flag: u32,
        remove: u32,
    } = 0x40,

    Unused20 = 0x41,

    ClearCursor {
        unit_type: u8,
        player_id: u32,
    } = 0x42,

    Unused21 = 0x43,

    Unused22 = 0x44,

    Unknown9 {
        unknown: [u8; 12],
    } = 0x45,

    Unused23 = 0x46,

    /// Unit relator notification.
    ///
    /// Packet tables name `0x47` and `0x48` as relators and describe the
    /// payload as a 16-bit unit-type-ish parameter, a unit GUID, and a trailing
    /// 32-bit parameter. Live LoD 1.14d captures commonly emit paired relators
    /// for player ids with the trailing parameter set to zero. Resource
    /// projects such as blacha/diablo2 currently parse but ignore them for
    /// state reconstruction, so libd2 exposes the envelope without assigning
    /// gameplay semantics yet.
    Relator1 {
        unit_type: u8,
        gap: u8,
        unit_id: u32,
        param2: u32,
    } = 0x47,

    /// Second unit relator notification; see [`ServerMessage::Relator1`].
    Relator2 {
        unit_type: u8,
        gap: u8,
        unit_id: u32,
        param2: u32,
    } = 0x48,

    Unused24 = 0x49,

    Unused25 = 0x4A,

    Unused26 = 0x4B,

    UnitSkillOnTarget {
        unit_type: u8,
        unit_id: u32,
        skill_id: u16,
        skill_level: u8,
        target_type: u8,
        target_id: u32,
        unused: u16, // all zeros acc. to bh
    } = 0x4C,

    UnitSkillOnLocation {
        unit_type: u8,
        unit_id: u32,
        skill: u16,
        unknown1: u16, // FIXME conflicting info: u8 or u16?
        skill_level: u8,
        x: u16,
        y: u16,
        unknown2: u16, // acc. to bh all zeros
    } = 0x4D,

    MercForHire {
        merc_name_id: u16,
        seed: u32,
    } = 0x4E,

    StartMercList = 0x4F, // FIXME conflicting info: startmerclist vs clearmerclist

    /// FIXME conflicting info
    /// bh reports:
    /// "D2GS_QUEST_SPECIAL" : {
    ///	"PacketId" : "0x50",
    ///	"Description" : "",
    ///	"Size" : 15,
    ///	"Structure" : [
    ///		{ "BYTE" : "PacketId" },
    ///		{ "short" : "nMessageType" },
    ///		{ "short" : "nArg1" },
    ///		{ "short" : "nArg2" },
    ///		{ "short" : "nArg3" },
    ///		{ "short" : "nArg4" },
    ///		{ "short" : "nArg5" },
    ///		{ "short" : "nArg6" }
    ///	]
    ///},
    StartGame = 0x50,

    WorldObject {
        object_type: u8,
        object_id: u32,
        object_class: u16,
        x: u16,
        y: u16,
        state: u8,
        interaction: u8,
    } = 0x51,

    PlayerQuestLogInfo {
        unknown: [u8; 41],
    } = 0x52,

    /// FIXME conflicting info mephi vs bh
    /// PlayerSlotRefresh {
    ///     slot:       u32,
    ///     unknown:    u8,
    ///     tick_count: u32
    /// } = 0x53,
    DarknessUpdate {
        act: u32,
        angle: u32,
        on_off: u8,
    } = 0x53,

    Unknown10 {
        unknown: [u8; 9],
    } = 0x54,

    Unknown11 {
        unknown: [u8; 2],
    } = 0x55,

    Unused27 = 0x56,

    NpcEnchants {
        monster_id: u32,
        monster_type: u8,
        monster_name_id: u16,
        enchant: [u8; 3],
        unused: u8,
        champion: u16, // TODO is this a class? is this a strength multiplier?
    } = 0x57,

    /// Conflicting info:
    /// Uknown28 {
    ///     unknown: `[u8; 13]`
    /// } = 0x58,
    /// BH variant chosen for now
    OpenUI {
        unit_id: u32,
        ui_type: u8,
        some_bool: u8,
    } = 0x58,

    AssignPlayer {
        unit_id: u32,
        class: u8,
        szname: [u8; 16],
        x: u16,
        y: u16,
    } = 0x59,

    EventMessages {
        message_type: u8,
        color: u8,
        arg: u32,
        arg_type: u8,
        name1: [u8; 16],
        name2: [u8; 16],
    } = 0x5A,

    PlayerJoined {
        packet_length: u16,
        player_id: u32,
        character_class: u8,
        character_name: [u8; 16],
        character_level: u16,
        party_id: u16,
        unknown: [u8; 8],
    } = 0x5B,

    PlayerLeft {
        player_id: u32,
    } = 0x5C,

    QuestStateUpdate {
        quest_id: u8,
        alert_flags: u8,
        status: u8,
        extra: u16,
    } = 0x5D,

    GameQuestAvailability {
        unknown: [u8; 37],
    } = 0x5E,

    Unknown14 {
        unknown: [u8; 4],
    } = 0x5F,

    TownPortalState {
        state: u8,
        area_id: u8,
        unit_id: u32,
    } = 0x60,

    ActUnlocked {
        act: u8,
    } = 0x61,

    MakeUnitTargetable {
        unit_type: u8,
        unit_id: u32,
        unused: u8,
    } = 0x62,

    WaypointMenu {
        unit_id: u32,
        unknown: u16,
        waypoint_bits: [u8; 8],
        unused: [u8; 6],
    } = 0x63,

    Unused29 = 0x64,

    PlayerKillCount {
        player_id: u32,
        count: u16,
    } = 0x65,

    Unknown17 {
        unknown: [u8; 6],
    } = 0x66,

    NpcMove {
        unit_id: u32,
        unit_type: u8,
        target_x: u16,
        target_y: u16,
        unknown1: u16,
        unknown2: u8,
        velocity: u16,
        unknown4: u8,
    } = 0x67,

    NpcMoveToEntity {
        unit_id: u32,
        move_type: u8,
        target_x: u16,
        target_y: u16,
        target_unit_type: u8,
        target_id: u32,
        unknown1: u16,
        unknown2: u8,
        unused: u16,
        unknown4: u8,
    } = 0x68,

    NpcStateUpdate {
        unit_id: u32,
        state: u8,
        x: u16,
        y: u16,
        unit_life: u8,
        hit_class: u8,
    } = 0x69,

    /// Some unknown NPC info/interaction/state change
    /// TODO find out what it does
    Unknown18 {
        unit_id: u32,
        state: u8,
        unknown1: u8,
        unknown2: u32,
        unknown3: u8,
    } = 0x6A,

    NpcAction {
        unit_id: u32,
        action: u8,
        unknown: [u8; 6],
        x: u16,
        y: u16,
    } = 0x6B,

    NpcAttack {
        unit_id: u32,
        attack_type: u16,
        target_id: u32,
        target_type: u8,
        target_x: u16,
        target_y: u16,
    } = 0x6C,

    NpcStop {
        unit_id: u32,
        x: u16,
        y: u16,
        unit_life: u8,
    } = 0x6D,

    Unknown19 = 0x6E,
    Unknown20 = 0x6F,
    Unknown21 = 0x70,
    Unknown22 = 0x71,
    Unknown23 = 0x72,

    /// Update for missile objects
    /// current_frame might be wrong acc. to bh
    MissileData {
        unused: u32,
        missile_class: u16,
        missile_x: u32,
        missile_y: u32,
        target_x: u32,
        target_y: u32,
        current_frame: u16,
        owner_type: u8,
        owner_id: u32,
        skill_level: u8,
        pierce_level: u8,
    } = 0x73,

    PlayerCorpseAssign {
        assign: u8,
        owner_id: u32,
        corpse_id: u32,
    } = 0x74,

    PlayerPartyInfo {
        unit_id: u32,
        party_id: u16,
        character_level: u16,
        relationship: u16,
        in_party: u16,
    } = 0x75,

    PlayerInProximity {
        unit_type: u8,
        unit_id: u32,
    } = 0x76,

    TradeAction {
        request_type: u8,
    } = 0x77,

    TradeAccepted {
        character_name: [u8; 16],
        unit_id: u32,
    } = 0x78,

    GoldInTrade {
        owner_id: u8,
        amount: u32,
    } = 0x79,

    SummonAssign {
        action: u8,
        skill_id: u8,
        summon_type: u16,
        player_id: u32,
        summon_id: u32,
    } = 0x7A,

    AssignSkillHotkey {
        slot: u8,
        skill: u8,
        hand: u8,
        item_id: u32, // for item skills?
    } = 0x7B,

    UseScroll {
        scroll_type: u8,
        scroll_id: u32,
    } = 0x7C,

    SetItemState {
        unit_type: u8,
        unit_id: u32,
        item_id: u32,
        and_value: u32,
        flags: u32,
    } = 0x7D,

    /// Unknown usage
    CmnCof {
        unknown: [u8; 4],
    } = 0x7E,

    AllyPartyInfo {
        unit_type: u8,
        unit_life: u16,
        unit_id: u32,
        unit_area: u32,
    } = 0x7F,

    Unused30 = 0x80,

    AssignMerc {
        skill_id: u8,
        summon_type: u16,
        player_id: u32,
        merc_id: u32,
        seed2: u32,
        init_seed: u32,
    } = 0x81,

    /// Seems like local_id and remote_id contain
    /// portals GUID for both ends of each portal.
    /// TODO which one is for town and which one for wilderness?
    PortalOwnership {
        player_id: u32,
        player_name: u32,
        local_id: u32,
        remote_id: u32,
    } = 0x82,

    Unused31 = 0x83,
    Unused32 = 0x84,
    Unused33 = 0x85,
    Unused34 = 0x86,
    Unused35 = 0x87,
    Unused36 = 0x88,

    UniqueEvents {
        event_id: u8,
    } = 0x89,

    NpcWantsToInteract {
        unit_type: u8,
        unit_id: u32,
    } = 0x8A,

    PlayerPartyUpdate {
        unit_id: u32,
        party_state: u8,
    } = 0x8B,

    PlayerRelationUpdate {
        player1_id: u32,
        player2_id: u32,
        relationship: u16,
    } = 0x8C,

    AssignPlayerToParty {
        player_id: u32,
        party_id: u16,
    } = 0x8D,

    CorpseAssign {
        assign: u8,
        player_id: u32,
        corpse_id: u32,
    } = 0x8E,

    Pong {
        pong1: u32,
        pong2: u32,
        pong3: u32,
        count: u32,
        pong5: u32,
        pong6_warden: u32,
        pong7_warden: u32,
        pong8_warden: u32,
    } = 0x8F,

    PlayerMapUpdate {
        player_id: u32,
        player_x: u32,
        player_y: u32,
    } = 0x90,

    NpcGossip {
        act: u8,
        unknown: [u16; 12],
    } = 0x91,

    ObjectDisplayDisable {
        unit_type: u8,
        unit_id: u32,
    } = 0x92,

    UnknownUnitSkill {
        player_id: u32,
        unknown1: u8,
        skill_type: u8,
        skill_page: u8,
    } = 0x93,

    /// Base skill levels for a player (`D2GS_SKILLSLIST`).
    ///
    /// Legacy packet tables for 1.13c, 1.14d, and D2R 1.15 all describe this
    /// as a variable-length list keyed by global `Skills.txt` ids: one count
    /// byte, a player GUID, then `count` triples of little-endian skill id and
    /// base level. These are the skill points stored in the save-file `if`
    /// class-skill table, before item/aura modifiers are applied.
    PlayerSkillsInfo {
        skills_count: u8,
        player_id: u32,
        skills: Vec<SkillDescription>,
    } = 0x94,

    /// Local-player life/mana/stamina and movement verification.
    ///
    /// The 12-byte bitstream contains 15-bit HP, 15-bit MP, 15-bit stamina,
    /// 16-bit X, 16-bit Y, and two trailing 8-bit movement verification fields
    /// commonly named `dX` and `dY` in packet tables.
    LifeManaUpdate {
        bitfield: [u8; 12],
    } = 0x95,

    /// Local-player stamina and movement verification.
    ///
    /// Public sources disagree on the exact meaning of the trailing movement
    /// bits. The library decodes stamina, X/Y, and preserves the two 8-bit
    /// fields as raw values in [`PlayerMovement`](crate::PlayerMovement).
    WalkUpdate {
        bitfield: [u8; 8],
    } = 0x96,

    WeaponSwitch = 0x97,

    EvilHut {
        unit_id: u32,
        value: u16,
    } = 0x98,

    /// TODO is this for Chance-to-cast skills?
    SkillTriggerOnTarget {
        attacker_type: u8,
        attacker_id: u32,
        skill_id: u16,
        skill_level: u8,
        target_type: u8,
        target_id: u32,
        unused: u16,
    } = 0x99,

    /// TODO is this for chance-to-cast skills?
    SkillTriggerOnLocation {
        attacker_type: u8,
        attacker_id: u32,
        skill_id: u16,
        unused: u16,
        skill_level: u8,
        target_x: u16,
        target_y: u16,
        unused2: u16,
    } = 0x9A,

    MercReviveCost {
        merc_name_id: u16,
        revive_cost: u16,
        unused: u16,
    } = 0x9B,

    /// World item action (`D2GS_ITEM_WORLD`).
    ///
    /// The bitstream begins with item flags, item-data version, destination,
    /// and placement fields before continuing into quality/stat data.
    ItemActionWorld {
        action: u8,
        packet_size: u8,
        category: u8,
        item_id: u32,
        bitstream: Vec<u8>,
    } = 0x9C,

    /// Owned item action (`D2GS_ITEM_OWNED`).
    ///
    /// This shares the world-item payload shape and adds the owning unit type
    /// and id before the item bitstream.
    ItemActionOwned {
        action: u8,
        packet_size: u8,
        category: u8,
        item_id: u32,
        owner_type: u8,
        owner_id: u32,
        bitstream: Vec<u8>,
    } = 0x9D,

    MercAttributeU8 {
        attribute: u8,
        merc_id: u32,
        amount: u8,
    } = 0x9E,

    MercAttributeU16 {
        attribute: u8,
        merc_id: u32,
        amount: u16,
    } = 0x9F,

    MercAttributeU32 {
        attribute: u8,
        merc_id: u32,
        amount: u32,
    } = 0xA0,

    /// TODO
    /// This is probably merc_id: u32 and an amount: u8 and an unknown
    MercAddExpU8 {
        stat_id: u8,
        merc_id: u32,
        value: u8,
    } = 0xA1,

    /// TODO
    /// Same as with MercAddExpByte
    MercAddExpU16 {
        stat_id: u8,
        merc_id: u32,
        value: u16,
    } = 0xA2,

    /// TODO what does this signal exactly?
    SkillAuraStat {
        aura_stat_id: u8,
        skill_id: u16,
        skill_level: u16,
        unit_type: u8,
        unit_id: u32,
        target_type: u8,
        target_id: u32,
        target_x: u32,
        target_y: u32,
    } = 0xA3,

    BaalWaves {
        class_id: u16,
    } = 0xA4,

    StateSkillMove {
        unit_type: u8,
        unit_id: u32,
        skill_id: u16,
    } = 0xA5,

    /// This packet has it's handler in game code
    /// but it seems like it is never used.
    RunesTxt {
        must_be_zero: u8,
        packet_size: u16,
        txt_runes_size: u16,
        bitstream: [u8; 250], // { "BYTE" : "BitStream[nFullPacketSize - 6]" } FIXME maximum packet size
    } = 0xA6,

    /// The delayed state prevents the player from entering another town portal too quickly again
    DelayState {
        unit_type: u8,
        unit_id: u32,
        state: u8,
    } = 0xA7,

    SetState {
        unit_type: u8,
        unit_id: u32,
        packet_size: u8,
        state: u8,
        state_effects: [u8; 248], // { "BYTE" : "BitStream[nFullPacketSize - 8]" } FIXME maximum packet size
    } = 0xA8,

    EndState {
        unit_type: u8,
        unit_id: u32,
        state: u8,
    } = 0xA9,

    StateAdd {
        unit_type: u8,
        unit_id: u32,
        packet_size: u8,
        bitstream: [u8; 249], // { "BYTE" : "BitStream[nFullPacketSize - 7]" } FIXME maximum packet size
    } = 0xAA,

    NpcHeal {
        unit_type: u8,
        unit_id: u32,
        unit_life: u8,
    } = 0xAB,

    MonsterAssign {
        unit_id: u32,
        unit_code: u16,
        unit_x: u16,
        unit_y: u16,
        life_percent: u8,
        packet_size: u8,
        bitstream: Vec<u8>, // { "BYTE" : "BitStream[nFullPacketSize - 13]" }
    } = 0xAC,

    Unknown35 {
        unknown: [u8; 8],
    } = 0xAD,

    /// TODO
    /// Figure out what to return in order to fool warden
    /// Else just send exit game packets
    WardenRequest {
        stream_size: u16,
        bitstream: [u8; 254], // { "BYTE" : "Stream[nStreamSize]" } FIXME maximum packet size
    } = 0xAE,

    /// TODO what are the compression modes
    AdvertiseCompressionMode {
        use_compression: u8,
    } = 0xAF,

    GameConnectionTerminated = 0xB0,

    Unknown36 {
        unknown: [u8; 52],
    } = 0xB1,

    GamesInfo {
        unknown1: [u8; 16],
        unknown2: [u8; 16],
        unknown3: [u8; 16],
        clients_count: u16,
        game_token: u16,
    } = 0xB2,

    /// Apparently sent in game before disconnect to cache character data on client side
    DownloadSave {
        chunk_size: u8,
        first: u8,
        fillsize: u32,
        bitstream: [u8; 250], //{ "BYTE" : "Stream[nChunkSize]" } FIXME packet size
    } = 0xB3,

    ConnectionRefused {
        reason: u32,
    } = 0xB4,

    /// TODO is 'message' an array oder an id of a previously sent message?
    OverHead {
        unknown1: [u8; 3],
        unit_type: u8,
        unit_id: u32,
        unknown2: u16,
        unknown3: u8,
        message: u8, // is this a message id to be sent before/after?
        unknown4: u8,
    } = 0xB5,

    UnknownFF = 0xFF,
}

////////////////////////////////////////////////

// Additional Containers and Bitfields
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct SkillDescription {
    pub skill: u16,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessageParseError {
    EmptyPacket,
    UnsupportedPacketId(u8),
    UnexpectedLength {
        packet_id: u8,
        expected: usize,
        actual: usize,
    },
}

impl ServerMessage {
    pub fn parse(input: &[u8]) -> Result<Self, ServerMessageParseError> {
        let packet_id = *input.first().ok_or(ServerMessageParseError::EmptyPacket)?;

        match packet_id {
            0x00 => parse_empty(input, Self::GameLoading),
            0x01 => {
                let mut cursor = PacketCursor::new(input, 8)?;
                Ok(Self::GameFlags {
                    difficulty: cursor.u8(),
                    arena_flags: cursor.u32_le(),
                    is_expansion: cursor.u8(),
                    is_ladder: cursor.u8(),
                })
            }
            0x02 => parse_empty(input, Self::LoadSuccessful),
            0x03 => {
                let mut cursor = PacketCursor::new(input, 12)?;
                Ok(Self::LoadAct {
                    act: cursor.u8(),
                    map_id: cursor.u32_le(),
                    area_id: cursor.u16_le(),
                    automap: cursor.u32_le(),
                })
            }
            0x04 => parse_empty(input, Self::LoadComplete),
            0x05 => parse_empty(input, Self::UnloadComplete),
            0x06 => parse_empty(input, Self::GameExitSuccessful),
            0x07 => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::MapReveal {
                    tile_x: cursor.u16_le(),
                    tile_y: cursor.u16_le(),
                    area_id: cursor.u8(),
                })
            }
            0x08 => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::MapHide {
                    tile_x: cursor.u16_le(),
                    tile_y: cursor.u16_le(),
                    area_id: cursor.u8(),
                })
            }
            0x09 => {
                let mut cursor = PacketCursor::new(input, 11)?;
                Ok(Self::AssignLevelWarp {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    warp_class_id: cursor.u8(),
                    warp_x: cursor.u16_le(),
                    warp_y: cursor.u16_le(),
                })
            }
            0x0A => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::RemoveObject {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                })
            }
            0x0B => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::GameHandshake {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                })
            }
            0x0C => {
                let mut cursor = PacketCursor::new(input, 9)?;
                Ok(Self::NpcHit {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    animation_id: cursor.u16_le(),
                    alive: cursor.u8(),
                })
            }
            0x0D => {
                let mut cursor = PacketCursor::new(input, 13)?;
                Ok(Self::PlayerStop {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    hit_class: cursor.u8(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    unit_hit_class: cursor.u8(),
                    alive: cursor.u8(),
                })
            }
            0x0E => {
                let mut cursor = PacketCursor::new(input, 12)?;
                Ok(Self::ObjectState {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    portal_flags: cursor.u8(),
                    is_targetable: cursor.u8(),
                    unit_state: cursor.u32_le(),
                })
            }
            0x0F => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::PlayerMove {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    move_type: cursor.u8(),
                    target_x: cursor.u16_le(),
                    target_y: cursor.u16_le(),
                    unit_hit_class: cursor.u8(),
                    current_x: cursor.u16_le(),
                    current_y: cursor.u16_le(),
                })
            }
            0x10 => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::CharToObject {
                    player_type: cursor.u8(),
                    player_id: cursor.u32_le(),
                    movement_type: cursor.u8(),
                    target_type: cursor.u8(),
                    target_id: cursor.u32_le(),
                    target_x: cursor.u16_le(),
                    target_y: cursor.u16_le(),
                })
            }
            0x11 => {
                let mut cursor = PacketCursor::new(input, 8)?;
                Ok(Self::ReportKill {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    unknown: cursor.u16_le(),
                })
            }
            0x15 => {
                let mut cursor = PacketCursor::new(input, 11)?;
                Ok(Self::ReassignPlayer {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    value: cursor.u8(),
                })
            }
            0x18 => {
                let mut cursor = PacketCursor::new(input, 15)?;
                Ok(Self::HPMPUPDATE {
                    packed_bits: cursor.array(),
                })
            }
            0x19 => {
                let mut cursor = PacketCursor::new(input, 2)?;
                Ok(Self::SmallGoldPickup {
                    amount: cursor.u8(),
                })
            }
            0x1A => {
                let mut cursor = PacketCursor::new(input, 2)?;
                Ok(Self::AddExpU8 {
                    amount: cursor.u8(),
                })
            }
            0x1B => {
                let mut cursor = PacketCursor::new(input, 3)?;
                Ok(Self::AddExpU16 {
                    amount: cursor.u16_le(),
                })
            }
            0x1C => {
                let mut cursor = PacketCursor::new(input, 5)?;
                Ok(Self::AddExpU32 {
                    amount: cursor.u32_le(),
                })
            }
            0x1D => {
                let mut cursor = PacketCursor::new(input, 3)?;
                Ok(Self::SetAttributeU8 {
                    attribute: cursor.u8(),
                    amount: cursor.u8(),
                })
            }
            0x1E => {
                let mut cursor = PacketCursor::new(input, 4)?;
                Ok(Self::SetAttributeU16 {
                    attribute: cursor.u8(),
                    amount: cursor.u16_le(),
                })
            }
            0x1F => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::SetAttributeU32 {
                    attribute: cursor.u8(),
                    amount: cursor.u32_le(),
                })
            }
            0x20 => {
                let mut cursor = PacketCursor::new(input, 10)?;
                Ok(Self::AttributeUpdate {
                    unit_id: cursor.u32_le(),
                    attribute: cursor.u8(),
                    amount: cursor.u32_le(),
                })
            }
            0x23 => {
                let mut cursor = PacketCursor::new(input, 13)?;
                Ok(Self::SetSkill {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    hand: cursor.u8(),
                    skill_id: cursor.u16_le(),
                    item_id: cursor.u32_le(),
                })
            }
            0x28 => {
                let mut cursor = PacketCursor::new(input, 103)?;
                Ok(Self::PlayerQuestInfo {
                    update_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    action_type: cursor.u8(),
                    quest_bits: cursor.array(),
                })
            }
            0x3E => {
                let mut cursor = PacketCursor::new_variable(input, 2, 1)?;
                let packet_size = cursor.u8();
                if packet_size < 2 || packet_size as usize > input.len() {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected: packet_size as usize,
                        actual: input.len(),
                    });
                }
                let bitstream_len = packet_size as usize - 2;
                let remaining = cursor.remaining();
                Ok(Self::UpdateItemStats {
                    packet_size,
                    bitstream: remaining[..bitstream_len].to_vec(),
                })
            }
            0x47 => {
                let mut cursor = PacketCursor::new(input, 11)?;
                Ok(Self::Relator1 {
                    unit_type: cursor.u8(),
                    gap: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    param2: cursor.u32_le(),
                })
            }
            0x48 => {
                let mut cursor = PacketCursor::new(input, 11)?;
                Ok(Self::Relator2 {
                    unit_type: cursor.u8(),
                    gap: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    param2: cursor.u32_le(),
                })
            }
            0x4C => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::UnitSkillOnTarget {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    skill_id: cursor.u16_le(),
                    skill_level: cursor.u8(),
                    target_type: cursor.u8(),
                    target_id: cursor.u32_le(),
                    unused: cursor.u16_le(),
                })
            }
            0x4D => {
                let mut cursor = PacketCursor::new(input, 17)?;
                Ok(Self::UnitSkillOnLocation {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    skill: cursor.u16_le(),
                    unknown1: cursor.u16_le(),
                    skill_level: cursor.u8(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    unknown2: cursor.u16_le(),
                })
            }
            0x51 => {
                let mut cursor = PacketCursor::new(input, 14)?;
                Ok(Self::WorldObject {
                    object_type: cursor.u8(),
                    object_id: cursor.u32_le(),
                    object_class: cursor.u16_le(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    state: cursor.u8(),
                    interaction: cursor.u8(),
                })
            }
            0x53 => {
                let mut cursor = PacketCursor::new(input, 10)?;
                Ok(Self::DarknessUpdate {
                    act: cursor.u32_le(),
                    angle: cursor.u32_le(),
                    on_off: cursor.u8(),
                })
            }
            0x59 => {
                let mut cursor = PacketCursor::new(input, 26)?;
                Ok(Self::AssignPlayer {
                    unit_id: cursor.u32_le(),
                    class: cursor.u8(),
                    szname: cursor.array(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                })
            }
            0x5A => {
                let mut cursor = PacketCursor::new(input, 40)?;
                Ok(Self::EventMessages {
                    message_type: cursor.u8(),
                    color: cursor.u8(),
                    arg: cursor.u32_le(),
                    arg_type: cursor.u8(),
                    name1: cursor.array(),
                    name2: cursor.array(),
                })
            }
            0x5B => {
                let mut cursor = PacketCursor::new_variable(input, 36, 1)?;
                let packet_length = cursor.u16_le();
                if packet_length as usize != input.len() {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected: packet_length as usize,
                        actual: input.len(),
                    });
                }
                Ok(Self::PlayerJoined {
                    packet_length,
                    player_id: cursor.u32_le(),
                    character_class: cursor.u8(),
                    character_name: cursor.array(),
                    character_level: cursor.u16_le(),
                    party_id: cursor.u16_le(),
                    unknown: cursor.array(),
                })
            }
            0x5C => {
                let mut cursor = PacketCursor::new(input, 5)?;
                Ok(Self::PlayerLeft {
                    player_id: cursor.u32_le(),
                })
            }
            0x67 => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::NpcMove {
                    unit_id: cursor.u32_le(),
                    unit_type: cursor.u8(),
                    target_x: cursor.u16_le(),
                    target_y: cursor.u16_le(),
                    unknown1: cursor.u16_le(),
                    unknown2: cursor.u8(),
                    velocity: cursor.u16_le(),
                    unknown4: cursor.u8(),
                })
            }
            0x68 => {
                let mut cursor = PacketCursor::new(input, 21)?;
                Ok(Self::NpcMoveToEntity {
                    unit_id: cursor.u32_le(),
                    move_type: cursor.u8(),
                    target_x: cursor.u16_le(),
                    target_y: cursor.u16_le(),
                    target_unit_type: cursor.u8(),
                    target_id: cursor.u32_le(),
                    unknown1: cursor.u16_le(),
                    unknown2: cursor.u8(),
                    unused: cursor.u16_le(),
                    unknown4: cursor.u8(),
                })
            }
            0x69 => {
                let mut cursor = PacketCursor::new(input, 12)?;
                Ok(Self::NpcStateUpdate {
                    unit_id: cursor.u32_le(),
                    state: cursor.u8(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    unit_life: cursor.u8(),
                    hit_class: cursor.u8(),
                })
            }
            0x6B => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::NpcAction {
                    unit_id: cursor.u32_le(),
                    action: cursor.u8(),
                    unknown: cursor.array(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                })
            }
            0x6C => {
                let mut cursor = PacketCursor::new(input, 16)?;
                Ok(Self::NpcAttack {
                    unit_id: cursor.u32_le(),
                    attack_type: cursor.u16_le(),
                    target_id: cursor.u32_le(),
                    target_type: cursor.u8(),
                    target_x: cursor.u16_le(),
                    target_y: cursor.u16_le(),
                })
            }
            0x6D => {
                let mut cursor = PacketCursor::new(input, 10)?;
                Ok(Self::NpcStop {
                    unit_id: cursor.u32_le(),
                    x: cursor.u16_le(),
                    y: cursor.u16_le(),
                    unit_life: cursor.u8(),
                })
            }
            0x75 => {
                let mut cursor = PacketCursor::new(input, 13)?;
                Ok(Self::PlayerPartyInfo {
                    unit_id: cursor.u32_le(),
                    party_id: cursor.u16_le(),
                    character_level: cursor.u16_le(),
                    relationship: cursor.u16_le(),
                    in_party: cursor.u16_le(),
                })
            }
            0x76 => {
                let mut cursor = PacketCursor::new(input, 6)?;
                Ok(Self::PlayerInProximity {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                })
            }
            0x77 => {
                let mut cursor = PacketCursor::new(input, 2)?;
                Ok(Self::TradeAction {
                    request_type: cursor.u8(),
                })
            }
            0x7D => {
                let mut cursor = PacketCursor::new(input, 18)?;
                Ok(Self::SetItemState {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    item_id: cursor.u32_le(),
                    and_value: cursor.u32_le(),
                    flags: cursor.u32_le(),
                })
            }
            0x8F => {
                let mut cursor = PacketCursor::new(input, 33)?;
                Ok(Self::Pong {
                    pong1: cursor.u32_le(),
                    pong2: cursor.u32_le(),
                    pong3: cursor.u32_le(),
                    count: cursor.u32_le(),
                    pong5: cursor.u32_le(),
                    pong6_warden: cursor.u32_le(),
                    pong7_warden: cursor.u32_le(),
                    pong8_warden: cursor.u32_le(),
                })
            }
            0x90 => {
                let mut cursor = PacketCursor::new(input, 13)?;
                Ok(Self::PlayerMapUpdate {
                    player_id: cursor.u32_le(),
                    player_x: cursor.u32_le(),
                    player_y: cursor.u32_le(),
                })
            }
            0x94 => {
                let mut cursor = PacketCursor::new_variable(input, 6, 1)?;
                let skills_count = cursor.u8();
                let expected = skills_count as usize * 3 + 6;
                if input.len() != expected {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected,
                        actual: input.len(),
                    });
                }
                let player_id = cursor.u32_le();
                let mut skills = Vec::with_capacity(skills_count as usize);
                for _ in 0..skills_count {
                    skills.push(SkillDescription {
                        skill: cursor.u16_le(),
                        level: cursor.u8(),
                    });
                }
                Ok(Self::PlayerSkillsInfo {
                    skills_count,
                    player_id,
                    skills,
                })
            }
            0x95 => {
                let mut cursor = PacketCursor::new(input, 13)?;
                Ok(Self::LifeManaUpdate {
                    bitfield: cursor.array(),
                })
            }
            0x96 => {
                let mut cursor = PacketCursor::new(input, 9)?;
                Ok(Self::WalkUpdate {
                    bitfield: cursor.array(),
                })
            }
            0x9C => {
                let mut cursor = PacketCursor::new_variable(input, 8, 2)?;
                let action = cursor.u8();
                let packet_size = cursor.u8();
                if packet_size as usize != input.len() {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected: packet_size as usize,
                        actual: input.len(),
                    });
                }
                Ok(Self::ItemActionWorld {
                    action,
                    packet_size,
                    category: cursor.u8(),
                    item_id: cursor.u32_le(),
                    bitstream: cursor.remaining().to_vec(),
                })
            }
            0x9D => {
                let mut cursor = PacketCursor::new_variable(input, 13, 2)?;
                let action = cursor.u8();
                let packet_size = cursor.u8();
                if packet_size as usize != input.len() {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected: packet_size as usize,
                        actual: input.len(),
                    });
                }
                Ok(Self::ItemActionOwned {
                    action,
                    packet_size,
                    category: cursor.u8(),
                    item_id: cursor.u32_le(),
                    owner_type: cursor.u8(),
                    owner_id: cursor.u32_le(),
                    bitstream: cursor.remaining().to_vec(),
                })
            }
            0xA9 => {
                let mut cursor = PacketCursor::new(input, 7)?;
                Ok(Self::EndState {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    state: cursor.u8(),
                })
            }
            0xAB => {
                let mut cursor = PacketCursor::new(input, 7)?;
                Ok(Self::NpcHeal {
                    unit_type: cursor.u8(),
                    unit_id: cursor.u32_le(),
                    unit_life: cursor.u8(),
                })
            }
            0xAC => {
                let mut cursor = PacketCursor::new_variable(input, 13, 12)?;
                let unit_id = cursor.u32_le();
                let unit_code = cursor.u16_le();
                let unit_x = cursor.u16_le();
                let unit_y = cursor.u16_le();
                let life_percent = cursor.u8();
                let packet_size = cursor.u8();
                if packet_size as usize != input.len() {
                    return Err(ServerMessageParseError::UnexpectedLength {
                        packet_id,
                        expected: packet_size as usize,
                        actual: input.len(),
                    });
                }
                Ok(Self::MonsterAssign {
                    unit_id,
                    unit_code,
                    unit_x,
                    unit_y,
                    life_percent,
                    packet_size,
                    bitstream: cursor.remaining().to_vec(),
                })
            }
            0xAF => {
                let mut cursor = PacketCursor::new(input, 2)?;
                Ok(Self::AdvertiseCompressionMode {
                    use_compression: cursor.u8(),
                })
            }
            0xB0 => parse_empty(input, Self::GameConnectionTerminated),
            _ => Err(ServerMessageParseError::UnsupportedPacketId(packet_id)),
        }
    }
}

impl TryFrom<&D2GSPacket> for ServerMessage {
    type Error = ServerMessageParseError;

    fn try_from(packet: &D2GSPacket) -> Result<Self, Self::Error> {
        Self::parse(&packet.data)
    }
}

fn parse_empty(
    input: &[u8],
    message: ServerMessage,
) -> Result<ServerMessage, ServerMessageParseError> {
    let packet_id = input[0];
    if input.len() == 1 {
        Ok(message)
    } else {
        Err(ServerMessageParseError::UnexpectedLength {
            packet_id,
            expected: 1,
            actual: input.len(),
        })
    }
}

struct PacketCursor<'a> {
    packet_id: u8,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> PacketCursor<'a> {
    fn new(input: &'a [u8], expected_len: usize) -> Result<Self, ServerMessageParseError> {
        let packet_id = input[0];
        if input.len() != expected_len {
            return Err(ServerMessageParseError::UnexpectedLength {
                packet_id,
                expected: expected_len,
                actual: input.len(),
            });
        }

        Ok(Self {
            packet_id,
            bytes: &input[1..],
            position: 0,
        })
    }

    fn new_variable(
        input: &'a [u8],
        min_len: usize,
        length_offset: usize,
    ) -> Result<Self, ServerMessageParseError> {
        let packet_id = input[0];
        if input.len() < min_len {
            let expected = input
                .get(length_offset)
                .copied()
                .map(usize::from)
                .unwrap_or(min_len);
            return Err(ServerMessageParseError::UnexpectedLength {
                packet_id,
                expected,
                actual: input.len(),
            });
        }

        Ok(Self {
            packet_id,
            bytes: &input[1..],
            position: 0,
        })
    }

    fn u8(&mut self) -> u8 {
        let value = self.bytes[self.position];
        self.position += 1;
        value
    }

    fn u16_le(&mut self) -> u16 {
        u16::from_le_bytes(self.array())
    }

    fn u32_le(&mut self) -> u32 {
        u32::from_le_bytes(self.array())
    }

    fn array<const N: usize>(&mut self) -> [u8; N] {
        let end = self.position + N;
        let value = self.bytes[self.position..end]
            .try_into()
            .unwrap_or_else(|_| panic!("packet 0x{:02X} parser over-read", self.packet_id));
        self.position = end;
        value
    }

    fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.position..]
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerMessage, ServerMessageParseError, SkillDescription};
    use crate::core::network::d2gs::D2GSReader;

    #[test]
    fn parse_game_flags_reads_little_endian_arena_flags() {
        let message = ServerMessage::parse(&[0x01, 0x02, 0x04, 0x00, 0x10, 0x00, 0x01, 0x00])
            .expect("game flags should parse");

        assert_eq!(
            message,
            ServerMessage::GameFlags {
                difficulty: 0x02,
                arena_flags: 0x0010_0004,
                is_expansion: 0x01,
                is_ladder: 0x00,
            }
        );
    }

    #[test]
    fn parse_load_act_reads_seed_area_and_automap() {
        let message = ServerMessage::parse(&[
            0x03, 0x01, 0x44, 0x33, 0x22, 0x11, 0x28, 0x00, 0xDD, 0xCC, 0xBB, 0xAA,
        ])
        .expect("load act should parse");

        assert_eq!(
            message,
            ServerMessage::LoadAct {
                act: 1,
                map_id: 0x1122_3344,
                area_id: 0x0028,
                automap: 0xAABB_CCDD,
            }
        );
    }

    #[test]
    fn parse_player_move_reads_target_and_current_coordinates() {
        let message = ServerMessage::parse(&[
            0x0F, 0x00, 0x78, 0x56, 0x34, 0x12, 0x17, 0x40, 0x1F, 0x41, 0x1F, 0x02, 0x38, 0x1F,
            0x39, 0x1F,
        ])
        .expect("player move should parse");

        assert_eq!(
            message,
            ServerMessage::PlayerMove {
                unit_type: 0,
                unit_id: 0x1234_5678,
                move_type: 0x17,
                target_x: 8000,
                target_y: 8001,
                unit_hit_class: 2,
                current_x: 7992,
                current_y: 7993,
            }
        );
    }

    #[test]
    fn parse_variable_item_packets_preserves_envelopes_and_bitstreams() {
        let world = ServerMessage::parse(&[
            0x9C, 0x01, 0x0b, 0x04, 0x44, 0x33, 0x22, 0x11, 0xaa, 0xbb, 0xcc,
        ])
        .expect("world item should parse");
        assert_eq!(
            world,
            ServerMessage::ItemActionWorld {
                action: 0x01,
                packet_size: 0x0b,
                category: 0x04,
                item_id: 0x1122_3344,
                bitstream: vec![0xaa, 0xbb, 0xcc],
            }
        );

        let owned = ServerMessage::parse(&[
            0x9D, 0x02, 0x0f, 0x05, 0x88, 0x77, 0x66, 0x55, 0x00, 0x04, 0x03, 0x02, 0x01, 0xdd,
            0xee,
        ])
        .expect("owned item should parse");
        assert_eq!(
            owned,
            ServerMessage::ItemActionOwned {
                action: 0x02,
                packet_size: 0x0f,
                category: 0x05,
                item_id: 0x5566_7788,
                owner_type: 0x00,
                owner_id: 0x0102_0304,
                bitstream: vec![0xdd, 0xee],
            }
        );

        let stats = ServerMessage::parse(&[0x3E, 0x05, 0x10, 0x20, 0x30])
            .expect("item stat update should parse");
        assert_eq!(
            stats,
            ServerMessage::UpdateItemStats {
                packet_size: 0x05,
                bitstream: vec![0x10, 0x20, 0x30],
            }
        );

        let mut padded_stats = vec![0x3E, 0x05, 0x10, 0x20, 0x30];
        padded_stats.resize(34, 0);
        assert_eq!(
            ServerMessage::parse(&padded_stats).expect("padded item stat update should parse"),
            ServerMessage::UpdateItemStats {
                packet_size: 0x05,
                bitstream: vec![0x10, 0x20, 0x30],
            }
        );
    }

    #[test]
    fn parse_player_skills_info_reads_variable_skill_list() {
        let message = ServerMessage::parse(&[
            0x94, 0x02, 0x44, 0x33, 0x22, 0x11, 0x24, 0x00, 0x05, 0x40, 0x00, 0x01,
        ])
        .expect("skill list should parse");

        assert_eq!(
            message,
            ServerMessage::PlayerSkillsInfo {
                skills_count: 2,
                player_id: 0x1122_3344,
                skills: vec![
                    SkillDescription {
                        skill: 36,
                        level: 5,
                    },
                    SkillDescription {
                        skill: 64,
                        level: 1,
                    },
                ],
            }
        );
    }

    #[test]
    fn parse_player_skills_info_rejects_size_mismatch() {
        let error = ServerMessage::parse(&[0x94, 0x02, 0x44, 0x33, 0x22, 0x11, 0x24, 0x00, 0x05])
            .expect_err("truncated skill list should fail");

        assert_eq!(
            error,
            ServerMessageParseError::UnexpectedLength {
                packet_id: 0x94,
                expected: 12,
                actual: 9,
            }
        );
    }

    #[test]
    fn parse_variable_item_packet_rejects_size_mismatch() {
        let error = ServerMessage::parse(&[0x9C, 0x01, 0x0c, 0x04, 0, 0, 0, 0, 0xaa])
            .expect_err("declared packet size is wrong");

        assert_eq!(
            error,
            ServerMessageParseError::UnexpectedLength {
                packet_id: 0x9C,
                expected: 0x0c,
                actual: 9,
            }
        );
    }

    #[test]
    fn parse_relator_packets_reads_unit_id_and_trailing_parameter() {
        assert_eq!(
            ServerMessage::parse(&[0x47, 0x00, 0x00, 0x7A, 0x94, 0xCD, 0x83, 0, 0, 0, 0])
                .expect("relator1 should parse"),
            ServerMessage::Relator1 {
                unit_type: 0,
                gap: 0,
                unit_id: 0x83CD_947A,
                param2: 0,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x48, 0x01, 0x02, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55
            ])
            .expect("relator2 should parse"),
            ServerMessage::Relator2 {
                unit_type: 1,
                gap: 2,
                unit_id: 0x1122_3344,
                param2: 0x5566_7788,
            }
        );
    }

    #[test]
    fn parse_set_item_state_reads_item_guid_and_flags() {
        let message = ServerMessage::parse(&[
            0x7D, 0x00, 0x04, 0x03, 0x02, 0x01, 0x88, 0x77, 0x66, 0x55, 0xDD, 0xCC, 0xBB, 0xAA,
            0x44, 0x33, 0x22, 0x11,
        ])
        .expect("set item state should parse");

        assert_eq!(
            message,
            ServerMessage::SetItemState {
                unit_type: 0,
                unit_id: 0x0102_0304,
                item_id: 0x5566_7788,
                and_value: 0xAABB_CCDD,
                flags: 0x1122_3344,
            }
        );
    }

    #[test]
    fn lod_1_14d_assumed_parse_common_live_capture_packets() {
        // Live-shaped bytes captured before fixture metadata existed; assume LoD 1.14d.
        assert_eq!(
            ServerMessage::parse(&[0xAF, 0x00]).expect("compression mode should parse"),
            ServerMessage::AdvertiseCompressionMode { use_compression: 0 }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x90, 0xB7, 0xB4, 0xB9, 0xB0, 0xFE, 0x13, 0x00, 0x00, 0x30, 0x14, 0x00, 0x00,
            ])
            .expect("player map update should parse"),
            ServerMessage::PlayerMapUpdate {
                player_id: 0xB0B9_B4B7,
                player_x: 0x13FE,
                player_y: 0x1430,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x18, 0x4F, 0x80, 0x33, 0x8B, 0xD6, 0x08, 0xFF, 0x7E, 0x01, 0x02, 0x03, 0x04, 0x05,
                0x06,
            ])
            .expect("hpmp update should parse"),
            ServerMessage::HPMPUPDATE {
                packed_bits: [
                    0x4F, 0x80, 0x33, 0x8B, 0xD6, 0x08, 0xFF, 0x7E, 0x01, 0x02, 0x03, 0x04, 0x05,
                    0x06,
                ],
            }
        );

        assert_eq!(
            ServerMessage::parse(&[0x96, 0x4F, 0x80, 0x33, 0x8B, 0xD6, 0x08, 0xFF, 0x7E])
                .expect("walk update should parse"),
            ServerMessage::WalkUpdate {
                bitfield: [0x4F, 0x80, 0x33, 0x8B, 0xD6, 0x08, 0xFF, 0x7E],
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x75, 0x6D, 0x13, 0x9C, 0x41, 0xFF, 0xFF, 0x58, 0x00, 0x00, 0x00, 0x01, 0x00,
            ])
            .expect("player party info should parse"),
            ServerMessage::PlayerPartyInfo {
                unit_id: 0x419C_136D,
                party_id: 0xFFFF,
                character_level: 88,
                relationship: 0,
                in_party: 1,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[0x76, 0x00, 0x9B, 0xB0, 0x0C, 0x67])
                .expect("player proximity should parse"),
            ServerMessage::PlayerInProximity {
                unit_type: 0,
                unit_id: 0x670C_B09B,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x4C, 0x01, 0x11, 0x00, 0x00, 0x00, 0x4A, 0x01, 0x04, 0x00, 0xC3, 0x38, 0xB6, 0x70,
                0x00, 0x00,
            ])
            .expect("unit skill on target should parse"),
            ServerMessage::UnitSkillOnTarget {
                unit_type: 1,
                unit_id: 0x11,
                skill_id: 330,
                skill_level: 4,
                target_type: 0,
                target_id: 0x70B6_38C3,
                unused: 0,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x4D, 0x01, 0x11, 0x00, 0x00, 0x00, 0x4A, 0x01, 0x00, 0x00, 0x04, 0xCF, 0x0E, 0xFE,
                0x13, 0x00, 0x00,
            ])
            .expect("unit skill on location should parse"),
            ServerMessage::UnitSkillOnLocation {
                unit_type: 1,
                unit_id: 0x11,
                skill: 330,
                unknown1: 0,
                skill_level: 4,
                x: 3791,
                y: 5118,
                unknown2: 0,
            }
        );

        assert_eq!(
            ServerMessage::parse(&[
                0x23, 0x00, 0xC3, 0x38, 0xB6, 0x70, 0x00, 0x46, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
            ])
            .expect("set skill should parse"),
            ServerMessage::SetSkill {
                unit_type: 0,
                unit_id: 0x70B6_38C3,
                hand: 0,
                skill_id: 0x46,
                item_id: 0xFFFF_FFFF,
            }
        );

        let mut quest_info = vec![0x28, 0x06, 0xC3, 0x38, 0xB6, 0x70, 0x01];
        quest_info.extend(0..96);
        let mut quest_bits = [0u8; 96];
        for (index, byte) in quest_bits.iter_mut().enumerate() {
            *byte = index as u8;
        }
        assert_eq!(
            ServerMessage::parse(&quest_info).expect("player quest info should parse"),
            ServerMessage::PlayerQuestInfo {
                update_type: 6,
                unit_id: 0x70B6_38C3,
                action_type: 1,
                quest_bits,
            }
        );
    }

    #[test]
    fn decoded_plain_packet_can_be_parsed_as_assign_player() {
        let mut reader = D2GSReader::new();
        let mut packet = vec![0x59, 0x04, 0x03, 0x02, 0x01, 0x03];
        packet.extend_from_slice(b"Rusty\0\0\0\0\0\0\0\0\0\0\0");
        packet.extend_from_slice(&1234u16.to_le_bytes());
        packet.extend_from_slice(&5678u16.to_le_bytes());

        reader.read(&packet);
        let decoded = reader.next().expect("plain D2GS packet should be queued");
        let message = ServerMessage::try_from(&decoded).expect("assign player should parse");

        assert_eq!(
            message,
            ServerMessage::AssignPlayer {
                unit_id: 0x0102_0304,
                class: 3,
                szname: *b"Rusty\0\0\0\0\0\0\0\0\0\0\0",
                x: 1234,
                y: 5678,
            }
        );
    }

    #[test]
    fn parse_rejects_truncated_fixed_length_packet() {
        let error = ServerMessage::parse(&[0x0F, 0x00]).expect_err("packet is truncated");

        assert_eq!(
            error,
            ServerMessageParseError::UnexpectedLength {
                packet_id: 0x0F,
                expected: 16,
                actual: 2,
            }
        );
    }

    #[test]
    fn parse_rejects_unsupported_packet_id() {
        let error = ServerMessage::parse(&[0xB1]).expect_err("packet parser is not complete yet");

        assert_eq!(error, ServerMessageParseError::UnsupportedPacketId(0xB1));
    }

    #[test]
    fn parse_rejects_empty_input() {
        let error = ServerMessage::parse(&[]).expect_err("empty packet cannot be parsed");

        assert_eq!(error, ServerMessageParseError::EmptyPacket);
    }
}
