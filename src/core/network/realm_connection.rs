pub mod server_message;

#[repr(u32)]
pub enum RealmStatusCode{
    Success             = 0x00,
    RealmDown1          = 0x02,     // Realm Unavailable: No Battle.net connection detected.
    CharacterExists     = 0x14,     // Character already exists, or maximum number of characters (currently 8) reached.
    InvalidCharacterName = 0x15,    // Invalid name
    RealmDown2          = 0x0A,     // 0x0A-0x0D: Realm Unavailable: No Battle.net connection detected.
    RealmDown3          = 0x0B,     // 0x0A-0x0D: Realm Unavailable: No Battle.net connection detected.
    RealmDown4          = 0x0C,     // 0x0A-0x0D: Realm Unavailable: No Battle.net connection detected.
    RealmDown5          = 0x0D,     // 0x0A-0x0D: Realm Unavailable: No Battle.net connection detected.
    GameExists          = 0x1F,     // Game already exists.
    InvalidGameName     = 0x1E,     // Invalid game name.
    ServerDown          = 0x20,     // Game servers are down.
    PasswordIncorrect   = 0x29,
    GameDoesNotExist    = 0x2A,
    GameFull            = 0x2B,
    LevelRequirementNotMet = 0x2C,  // 0x2C: You do not meet the level requirements for this game.
    CharacterNotFound   = 0x46,     // Character not found
    CantJoinDead        = 0x6E,     // A dead hardcore character cannot create/join a game.
    CantJoinHardcore    = 0x71,     // A non-hardcore character cannot join a game created by a Hardcore character.
    CantJoinNightmare   = 0x73,     // Unable to join a Nightmare game.
    CantJoinHell        = 0x74,     // Unable to join a Hell game.
    CantJoinExpansion   = 0x78,     // A non-expansion character cannot join a game created by an Expansion character.
    CantJoinNonExpansion = 0x79,    // A Expansion character cannot join a game created by a non-expansion character.
    LogonFailed         = 0x7A,     // Logon failed / Expansion Upgrade failed
    CharacterExpired    = 0x7B,     // Character expired
    CantJoinLadder      = 0x7D,     // A non-ladder character cannot join a game created by a Ladder character.
    CdKeyBanned         = 0x7E,     // CDKey banned from realm play.
    TemporaryBan        = 0x7F      // Temporary IP ban "Your connection has been temporarily restricted from this realm. Please try to log in at another time."
}

    
    