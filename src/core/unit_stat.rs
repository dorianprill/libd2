// from https://github.com/noah-/d2bs/blob/master/Constants.h
#[repr(u16)]
pub enum UnitStat {
    Strength = 0,           // str
    Energy = 1,             // energy
    Dexterity = 2,          // dexterity
    Vitality = 3,           // vitality
    StatPointsUnspent = 4,  // unspent stat points
    SkillPointsUnspent = 5, // unspent skill points
    Life = 6,               // life
    LifeMax = 7,            // max life
    Mana = 8,               // mana
    ManaMax = 9,            // max mana
    Stamina = 10,           // stamina
    StaminaMax = 11,        // max stamina
    Level = 12,             // character level
    Experience = 13,        // experience
    GoldOnCharacter = 14,   // gold
    GoldInStash = 15,       // stash gold
    EnhancedDefense = 16,   // enahcned defense percent
    EnhancedDamageMax = 17, // njipStats["itemmaxdamagepercent"]=[17,0];
    EnhancedDamageMin = 18, // njipStats["itemmindamagepercent"]=[18,0];	njipStats["enhanceddamage"]=[18,0];
    ChanceToBlock = 20,     // chance to block percent
    LastLevelExperience = 29,
    NextLevelExperience = 30,
    DamageReduction = 36,           // damage reduction (absolute?)
    MagicDamageReduction = 35,      // magic damage reduction
    MagicResist = 37,               // magic resist
    MagicResistMax = 38,            // max magic resist
    FireResist = 39,                // fire resist
    FireResistMax = 40,             // max fire resist
    LightningResist = 41,           // lightning resist
    LightningResistMax = 42,        // max lightning resist
    ColdResist = 43,                // cold resist
    ColdResistMax = 44,             // max cold resist
    PoisonResist = 45,              // poison resist
    PoisonResistMaximum = 46,       // max poison resist
    LifeLeech = 60,                 // Life Leech
    ManaLeech = 62,                 // Mana Leech
    VelocityPercent = 67,           // effective run/walk (i.e. charge?)
    AmmoQuantity = 70,              // ammo quantity(arrow/bolt/throwing)
    ItemDurability = 72,            // item durability
    ItemDurabilityMax = 73,         // max item durability
    GoldFind = 79,                  // Gold find (GF)
    MagicFind = 80,                 // magic find (MF)
    ItemLevelReq = 92,
    IncreasedAttackSpeed = 93,      // IAS
    FasterRunWalk = 96,             // faster run/walk
    FasterHitRecovery = 99,         // faster hit recovery
    FasterBlockRate = 102,          // faster block rate
    FasterCastRate = 105,           // faster cast rate
    PoisonLengthReduction = 110,    // Poison length reduction
    OpenWounds = 135,               // Open Wounds
    CrushingBlow = 136,             // crushing blow
    DeadlyStrike = 141,             // deadly strike
    FireAbsorbPercent = 142,        // fire absorb %
    FireAbsorbAbsolute = 143,       // fire absorb
    LightningAbsorbPercent = 144,   // lightning absorb %
    LightningAbsorbAbsolute = 145,  // lightning absorb
    ColdAbsorbPercent = 148,        // cold absorb %
    ColdAbsorbAbsolute = 149,       // cold absorb
    SlowPercent = 150,              // slow %
}