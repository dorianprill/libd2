/// Message Definitions according to https://bnetdocs.org/packet/index?order=packet-id-asc&pktapplayer%5B%5D=3

#[repr(u8)]
pub enum ServerMessage {

    KeepAlive = 0x00,

    StartUp {
        result: RealmStatusCode
    } = 0x01,

    CreateCharacter {
        result: RealmStatusCode
    } = 0x02,

    /// Game creation succeeded on server. 
    /// This does NOT automatically join the game - the client must also send packet MCP_JOINGAME. Results:
    CreateGame {
        request_id: u16,
        game_token: u16,
        unknown:    u16,
        result:     RealmStatusCode
    } = 0x03,


    JoinGame {
        request_id: u16,
        unknown:    u16,
        game_server_ip: u32,
        game_hash:  u32,
        result:     RealmStatusCode
    } = 0x04,

    /// Flag values (OR them together), if valid:
    /// Difficulty:
    ///     0x0000: Normal
    ///     0x1000: Nightmare
    ///     0x2000: Hell
    /// Type:
    ///     0x200000: Ladder
    ///     0x100000: Expansion
    ///     0x800:    Hardcore
    GameList {
        request_id: u16,
        index:      u32,
        nplayers:   u8,
        game_flags: u32,
        game_name:  [u8; 16],
        game_description: [u8; 32] // FIXME dynamic length of this field?
    } = 0x05,

    /// game_flags are the same as in message GameList
    /// The level range shown in game is calculated from the level restriction values.
    /// Level and difference bytes are used to make the range:
    /// max(1, level - difference) to min(99, level + difference).
    /// Internally, there are 16 character slots, but the last 8 are always empty.
    /// This value sometimes includes some empty character slots.
    /// Then, some empty strings are added to the end of the packet.
    /// To determine the number of characters really in the game:
    ///   CharsInGameReal = CharsInGameFake - AmountOfEmptyCharNames;
    /// Byte N here refers to character in slot N, or 0 if the slot is empty.
    GameInfo {
        request_id: u16,
        game_flags: u32,
        game_uptime_seconds: u32,
        level_restriction_level: u8,
        level_restriction_diff: u8,
        max_players: u8,
        nplayers:    u8,
        player_classes: [u8;16],
        player_levels:  [u8;16],
        // TODO how to generate variable length fields with deku? 
        //(STRING)     Game description
        //(STRING)[16] Character names **

    } = 0x06,


    CharacterLogon {
        result: RealmStatusCode
    } = 0x07,


    DeleteCharacter {
        result: RealmStatusCode
    } = 0x0A,


    /// https://bnetdocs.org/packet/354/mcp-requestladderdata
    RequestLadderData {
        ladder_type: u8,
        total_response_length: u16,
        current_message_length: u16,
        unreceived_message_length: u16,
        first_entry_rank: u16,
        unknown: u16,
        //Message data:
        nentries: u32,
        unknown: u32
        //For each entry: TODO MAKE STRUCT
        //   (UINT64)     Character experience
        //    (UINT8)     Character flags
        //    (UINT8)     Character title
        //    (UINT16)     Character level
        //    (UINT8)[16] Character name 
    } = 0x11,


    /// https://bnetdocs.org/packet/325/mcp-motd
    MessageOfTheDay {
        nknown: u8,
        message: [u8;512]
    } = 0x12,

    CreateQueue {
        position: u32
    } = 0x14,

    CharacterRank = 0x16,

    CharacterList {
        nchars_requested: u16,  // Number of characters requested
        nchars_exist: u32,      // Number of characters that exist on this account
        nchars_returned: u16,   // Number of characters returned
        //TODO For each character returned: 
        //(STRING) Character name
        //(STRING) Character statstring
    } = 0x17,

    UpgradeCharacter {
        result: RealmStatusCode
    }= 0x18,

    /// The expiration date is a second count. To determine when the character will expire, 
    /// add this time to January 1 00:00:00 UTC 1970 and determine the 
    /// difference between that value and now (all in seconds).
    CharacterList2 {
        nchars_requested: u16,  // Number of characters requested
        nchars_exist: u32,      // Number of characters that exist on this account
        nchars_returned: u16,   // Number of characters returned
        // TODO For each character returned: 
        // (UINT32) Expiration date seconds
        // (STRING) Character name
        // (STRING) Character statstring
    }= 0x19

}