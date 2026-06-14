use crate::core::character_class::CharacterClass;

/// A display group for one class skill tab/tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillCategory {
    /// In-game skill tab name.
    pub name: &'static str,
    /// Class-local save skill slots in this category.
    pub slots: &'static [usize],
}

/// Minimum character level and prerequisite slots for one class-local skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillRequirement {
    /// Minimum character level required to allocate the skill.
    pub level: u32,
    /// Class-local prerequisite skill slots.
    pub prereqs: &'static [usize],
}

const AMAZON_JAVELIN_AND_SPEAR: [usize; 10] = [4, 8, 9, 13, 14, 18, 19, 24, 28, 29];
const AMAZON_PASSIVE_AND_MAGIC: [usize; 10] = [2, 3, 7, 11, 12, 17, 22, 23, 26, 27];
const AMAZON_BOW_AND_CROSSBOW: [usize; 10] = [0, 1, 5, 6, 10, 15, 16, 20, 21, 25];
const AMAZON_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Bow and Crossbow",
        slots: &AMAZON_BOW_AND_CROSSBOW,
    },
    SkillCategory {
        name: "Passive and Magic",
        slots: &AMAZON_PASSIVE_AND_MAGIC,
    },
    SkillCategory {
        name: "Javelin and Spear",
        slots: &AMAZON_JAVELIN_AND_SPEAR,
    },
];

const SORCERESS_FIRE: [usize; 10] = [0, 1, 5, 10, 11, 15, 16, 20, 25, 26];
const SORCERESS_LIGHTNING: [usize; 10] = [2, 6, 7, 12, 13, 17, 18, 21, 22, 27];
const SORCERESS_COLD: [usize; 10] = [3, 4, 8, 9, 14, 19, 23, 24, 28, 29];
const SORCERESS_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Fire Spells",
        slots: &SORCERESS_FIRE,
    },
    SkillCategory {
        name: "Lightning Spells",
        slots: &SORCERESS_LIGHTNING,
    },
    SkillCategory {
        name: "Cold Spells",
        slots: &SORCERESS_COLD,
    },
];

const NECROMANCER_SUMMONING: [usize; 10] = [3, 4, 9, 13, 14, 19, 23, 24, 28, 29];
const NECROMANCER_POISON_AND_BONE: [usize; 10] = [1, 2, 7, 8, 12, 17, 18, 22, 26, 27];
const NECROMANCER_CURSES: [usize; 10] = [0, 5, 6, 10, 11, 15, 16, 20, 21, 25];
const NECROMANCER_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Summoning Spells",
        slots: &NECROMANCER_SUMMONING,
    },
    SkillCategory {
        name: "Poison and Bone",
        slots: &NECROMANCER_POISON_AND_BONE,
    },
    SkillCategory {
        name: "Curses",
        slots: &NECROMANCER_CURSES,
    },
];

const PALADIN_COMBAT: [usize; 10] = [0, 1, 5, 10, 11, 15, 16, 20, 21, 25];
const PALADIN_OFFENSIVE_AURAS: [usize; 10] = [2, 6, 7, 12, 17, 18, 22, 23, 26, 27];
const PALADIN_DEFENSIVE_AURAS: [usize; 10] = [3, 4, 8, 9, 13, 14, 19, 24, 28, 29];
const PALADIN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Defensive Auras",
        slots: &PALADIN_DEFENSIVE_AURAS,
    },
    SkillCategory {
        name: "Offensive Auras",
        slots: &PALADIN_OFFENSIVE_AURAS,
    },
    SkillCategory {
        name: "Combat Skills",
        slots: &PALADIN_COMBAT,
    },
];

const BARBARIAN_COMBAT: [usize; 10] = [0, 6, 7, 13, 14, 17, 18, 21, 25, 26];
const BARBARIAN_MASTERIES: [usize; 10] = [1, 2, 3, 8, 9, 10, 15, 19, 22, 27];
const BARBARIAN_WARCRIES: [usize; 10] = [4, 5, 11, 12, 16, 20, 23, 24, 28, 29];
const BARBARIAN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Warcries",
        slots: &BARBARIAN_WARCRIES,
    },
    SkillCategory {
        name: "Combat Masteries",
        slots: &BARBARIAN_MASTERIES,
    },
    SkillCategory {
        name: "Combat Skills",
        slots: &BARBARIAN_COMBAT,
    },
];

const DRUID_ELEMENTAL: [usize; 10] = [4, 8, 9, 13, 14, 19, 23, 24, 28, 29];
const DRUID_SHAPE_SHIFTING: [usize; 10] = [2, 3, 7, 11, 12, 17, 18, 21, 22, 27];
const DRUID_SUMMONING: [usize; 10] = [0, 1, 5, 6, 10, 15, 16, 20, 25, 26];
const DRUID_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Elemental",
        slots: &DRUID_ELEMENTAL,
    },
    SkillCategory {
        name: "Shape Shifting",
        slots: &DRUID_SHAPE_SHIFTING,
    },
    SkillCategory {
        name: "Summoning",
        slots: &DRUID_SUMMONING,
    },
];

const ASSASSIN_MARTIAL_ARTS: [usize; 10] = [3, 4, 8, 9, 14, 18, 19, 23, 24, 29];
const ASSASSIN_SHADOW_DISCIPLINES: [usize; 10] = [1, 2, 7, 12, 13, 16, 17, 22, 27, 28];
const ASSASSIN_TRAPS: [usize; 10] = [0, 5, 6, 10, 11, 15, 20, 21, 25, 26];
const ASSASSIN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Martial Arts",
        slots: &ASSASSIN_MARTIAL_ARTS,
    },
    SkillCategory {
        name: "Shadow Disciplines",
        slots: &ASSASSIN_SHADOW_DISCIPLINES,
    },
    SkillCategory {
        name: "Traps",
        slots: &ASSASSIN_TRAPS,
    },
];

const WARLOCK_SKILLS: [usize; 30] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29,
];
const WARLOCK_SKILL_CATEGORIES: [SkillCategory; 1] = [SkillCategory {
    name: "Warlock Skills",
    slots: &WARLOCK_SKILLS,
}];

/// Returns the localized English skill name for a class-local save slot.
pub fn skill_name(class: CharacterClass, slot: usize) -> &'static str {
    match class {
        CharacterClass::Amazon => match slot {
            0 => "Magic Arrow",
            1 => "Fire Arrow",
            2 => "Inner Sight",
            3 => "Critical Strike",
            4 => "Jab",
            5 => "Cold Arrow",
            6 => "Multiple Shot",
            7 => "Dodge",
            8 => "Power Strike",
            9 => "Poison Javelin",
            10 => "Exploding Arrow",
            11 => "Slow Missiles",
            12 => "Avoid",
            13 => "Impale",
            14 => "Lightning Bolt",
            15 => "Ice Arrow",
            16 => "Guided Arrow",
            17 => "Penetrate",
            18 => "Charged Strike",
            19 => "Plague Javelin",
            20 => "Strafe",
            21 => "Immolation Arrow",
            22 => "Dopplezon",
            23 => "Evade",
            24 => "Fend",
            25 => "Freezing Arrow",
            26 => "Valkyrie",
            27 => "Pierce",
            28 => "Lightning Strike",
            29 => "Lightning Fury",
            _ => "Unknown",
        },
        CharacterClass::Sorceress => match slot {
            0 => "Fire Bolt",
            1 => "Warmth",
            2 => "Charged Bolt",
            3 => "Ice Bolt",
            4 => "Frozen Armor",
            5 => "Inferno",
            6 => "Static Field",
            7 => "Telekinesis",
            8 => "Frost Nova",
            9 => "Ice Blast",
            10 => "Blaze",
            11 => "Fire Ball",
            12 => "Nova",
            13 => "Lightning",
            14 => "Shiver Armor",
            15 => "Fire Wall",
            16 => "Enchant",
            17 => "Chain Lightning",
            18 => "Teleport",
            19 => "Glacial Spike",
            20 => "Meteor",
            21 => "Thunder Storm",
            22 => "Energy Shield",
            23 => "Blizzard",
            24 => "Chilling Armor",
            25 => "Fire Mastery",
            26 => "Hydra",
            27 => "Lightning Mastery",
            28 => "Frozen Orb",
            29 => "Cold Mastery",
            _ => "Unknown",
        },
        CharacterClass::Necromancer => match slot {
            0 => "Amplify Damage",
            1 => "Teeth",
            2 => "Bone Armor",
            3 => "Skeleton Mastery",
            4 => "Raise Skeleton",
            5 => "Dim Vision",
            6 => "Weaken",
            7 => "Poison Dagger",
            8 => "Corpse Explosion",
            9 => "Clay Golem",
            10 => "Iron Maiden",
            11 => "Terror",
            12 => "Bone Wall",
            13 => "Golem Mastery",
            14 => "Raise Skeletal Mage",
            15 => "Confuse",
            16 => "Life Tap",
            17 => "Poison Explosion",
            18 => "Bone Spear",
            19 => "Blood Golem",
            20 => "Attract",
            21 => "Decrepify",
            22 => "Bone Prison",
            23 => "Summon Resist",
            24 => "Iron Golem",
            25 => "Lower Resist",
            26 => "Poison Nova",
            27 => "Bone Spirit",
            28 => "Fire Golem",
            29 => "Revive",
            _ => "Unknown",
        },
        CharacterClass::Paladin => match slot {
            0 => "Sacrifice",
            1 => "Smite",
            2 => "Might",
            3 => "Prayer",
            4 => "Resist Fire",
            5 => "Holy Bolt",
            6 => "Holy Fire",
            7 => "Thorns",
            8 => "Defiance",
            9 => "Resist Cold",
            10 => "Zeal",
            11 => "Charge",
            12 => "Blessed Aim",
            13 => "Cleansing",
            14 => "Resist Lightning",
            15 => "Vengeance",
            16 => "Blessed Hammer",
            17 => "Concentration",
            18 => "Holy Freeze",
            19 => "Vigor",
            20 => "Conversion",
            21 => "Holy Shield",
            22 => "Holy Shock",
            23 => "Sanctuary",
            24 => "Meditation",
            25 => "Fist Of The Heavens",
            26 => "Fanaticism",
            27 => "Conviction",
            28 => "Redemption",
            29 => "Salvation",
            _ => "Unknown",
        },
        CharacterClass::Barbarian => match slot {
            0 => "Bash",
            1 => "Sword Mastery",
            2 => "Axe Mastery",
            3 => "Mace Mastery",
            4 => "Howl",
            5 => "Find Potion",
            6 => "Leap",
            7 => "Double Swing",
            8 => "Pole Arm Mastery",
            9 => "Throwing Mastery",
            10 => "Spear Mastery",
            11 => "Taunt",
            12 => "Shout",
            13 => "Stun",
            14 => "Double Throw",
            15 => "Increased Stamina",
            16 => "Find Item",
            17 => "Leap Attack",
            18 => "Concentrate",
            19 => "Iron Skin",
            20 => "Battle Cry",
            21 => "Frenzy",
            22 => "Increased Speed",
            23 => "Battle Orders",
            24 => "Grim Ward",
            25 => "Whirlwind",
            26 => "Berserk",
            27 => "Natural Resistance",
            28 => "War Cry",
            29 => "Battle Command",
            _ => "Unknown",
        },
        CharacterClass::Druid => match slot {
            0 => "Raven",
            1 => "Poison Creeper",
            2 => "Werewolf",
            3 => "Lycanthropy",
            4 => "Firestorm",
            5 => "Oak Sage",
            6 => "Summon Spirit Wolf",
            7 => "Werebear",
            8 => "Molten Boulder",
            9 => "Arctic Blast",
            10 => "Carrion Vine",
            11 => "Feral Rage",
            12 => "Maul",
            13 => "Fissure",
            14 => "Cyclone Armor",
            15 => "Heart of Wolverine",
            16 => "Summon Dire Wolf",
            17 => "Rabies",
            18 => "Fire Claws",
            19 => "Twister",
            20 => "Solar Creeper",
            21 => "Hunger",
            22 => "Shock Wave",
            23 => "Volcano",
            24 => "Tornado",
            25 => "Spirit of Barbs",
            26 => "Summon Grizzly",
            27 => "Fury",
            28 => "Armageddon",
            29 => "Hurricane",
            _ => "Unknown",
        },
        CharacterClass::Assassin => match slot {
            0 => "Fire Blast",
            1 => "Claw Mastery",
            2 => "Psychic Hammer",
            3 => "Tiger Strike",
            4 => "Dragon Talon",
            5 => "Shock Web",
            6 => "Blade Sentinel",
            7 => "Burst of Speed",
            8 => "Fists of Fire",
            9 => "Dragon Claw",
            10 => "Charged Bolt Sentry",
            11 => "Wake of Fire",
            12 => "Weapon Block",
            13 => "Cloak of Shadows",
            14 => "Cobra Strike",
            15 => "Blade Fury",
            16 => "Fade",
            17 => "Shadow Warrior",
            18 => "Claws of Thunder",
            19 => "Dragon Tail",
            20 => "Lightning Sentry",
            21 => "Wake of Inferno",
            22 => "Mind Blast",
            23 => "Blades of Ice",
            24 => "Dragon Flight",
            25 => "Death Sentry",
            26 => "Blade Shield",
            27 => "Venom",
            28 => "Shadow Master",
            29 => "Phoenix Strike",
            _ => "Unknown",
        },
        CharacterClass::Warlock => "Warlock Skill",
    }
}

/// Returns the in-game skill-tab grouping for `class`.
pub fn skill_categories(class: CharacterClass) -> &'static [SkillCategory] {
    match class {
        CharacterClass::Amazon => &AMAZON_SKILL_CATEGORIES,
        CharacterClass::Sorceress => &SORCERESS_SKILL_CATEGORIES,
        CharacterClass::Necromancer => &NECROMANCER_SKILL_CATEGORIES,
        CharacterClass::Paladin => &PALADIN_SKILL_CATEGORIES,
        CharacterClass::Barbarian => &BARBARIAN_SKILL_CATEGORIES,
        CharacterClass::Druid => &DRUID_SKILL_CATEGORIES,
        CharacterClass::Assassin => &ASSASSIN_SKILL_CATEGORIES,
        CharacterClass::Warlock => &WARLOCK_SKILL_CATEGORIES,
    }
}

/// Returns level and prerequisite requirements for one class-local skill slot.
pub fn skill_requirement(class: CharacterClass, slot: usize) -> SkillRequirement {
    match class {
        CharacterClass::Amazon => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[0],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[4],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[1, 6],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[7],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[4],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[9],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[5],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[5, 6],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[3],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[8, 14],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[14],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[16],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[10],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[11],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[12],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[13],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[15],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[22, 23],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[17],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[18],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[19],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Sorceress => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[3],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[5],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[0],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[6],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[9, 4],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[10],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[1, 11],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[13],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[7],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[9],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[11, 15],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[12, 17],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[18, 17],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[8, 19],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[14],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[16],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[23],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Necromancer => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[4],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[0],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[1],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[0],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[6],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[9],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[4],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[5],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[10],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[7, 8],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[8],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[9],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[15],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[11],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[12, 18],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[13],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[19],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[16, 21],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[17],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[18],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[24],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[14, 24],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Paladin => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[2],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[0],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[1],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[3],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[10],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[5],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[12],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[6],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[13, 8],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[15],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[11, 16],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[18],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[7, 18],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[13],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[16, 20],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[17],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[23],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[19],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Barbarian => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[0],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            10 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            11 => SkillRequirement {
                level: 6,
                prereqs: &[4],
            },
            12 => SkillRequirement {
                level: 6,
                prereqs: &[4],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[0],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[7],
            },
            15 => SkillRequirement {
                level: 12,
                prereqs: &[],
            },
            16 => SkillRequirement {
                level: 12,
                prereqs: &[5],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[6],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[13],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[],
            },
            20 => SkillRequirement {
                level: 18,
                prereqs: &[11],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[14],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[15],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[12],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[16],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[17, 18],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[18],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[19],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[20, 23],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[23],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Druid => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[2],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[0],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[4],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[1],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[7],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[8],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[9],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[5],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[5, 6],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[11],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[11, 12],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[14],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[10],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[18],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[12],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[13],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[19],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[15],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[16],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[17],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[23, 29],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[24],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Assassin => match slot {
            0 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            1 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            2 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            3 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            4 => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
            5 => SkillRequirement {
                level: 6,
                prereqs: &[0],
            },
            6 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            7 => SkillRequirement {
                level: 6,
                prereqs: &[1],
            },
            8 => SkillRequirement {
                level: 6,
                prereqs: &[],
            },
            9 => SkillRequirement {
                level: 6,
                prereqs: &[4],
            },
            10 => SkillRequirement {
                level: 12,
                prereqs: &[5],
            },
            11 => SkillRequirement {
                level: 12,
                prereqs: &[0],
            },
            12 => SkillRequirement {
                level: 12,
                prereqs: &[1],
            },
            13 => SkillRequirement {
                level: 12,
                prereqs: &[2],
            },
            14 => SkillRequirement {
                level: 12,
                prereqs: &[3],
            },
            15 => SkillRequirement {
                level: 18,
                prereqs: &[6, 11],
            },
            16 => SkillRequirement {
                level: 18,
                prereqs: &[7],
            },
            17 => SkillRequirement {
                level: 18,
                prereqs: &[13, 12],
            },
            18 => SkillRequirement {
                level: 18,
                prereqs: &[8],
            },
            19 => SkillRequirement {
                level: 18,
                prereqs: &[9],
            },
            20 => SkillRequirement {
                level: 24,
                prereqs: &[10],
            },
            21 => SkillRequirement {
                level: 24,
                prereqs: &[11],
            },
            22 => SkillRequirement {
                level: 24,
                prereqs: &[13],
            },
            23 => SkillRequirement {
                level: 24,
                prereqs: &[18],
            },
            24 => SkillRequirement {
                level: 24,
                prereqs: &[19],
            },
            25 => SkillRequirement {
                level: 30,
                prereqs: &[20],
            },
            26 => SkillRequirement {
                level: 30,
                prereqs: &[15],
            },
            27 => SkillRequirement {
                level: 30,
                prereqs: &[16],
            },
            28 => SkillRequirement {
                level: 30,
                prereqs: &[17],
            },
            29 => SkillRequirement {
                level: 30,
                prereqs: &[14, 23],
            },
            _ => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        },
        CharacterClass::Warlock => SkillRequirement {
            level: 1,
            prereqs: &[],
        },
    }
}

/// Returns prerequisite slots that must receive one hard point before `slot` can be increased.
pub fn missing_skill_prereqs(class: CharacterClass, skills: &[u8; 30], slot: usize) -> Vec<usize> {
    let mut missing = Vec::new();
    collect_missing_skill_prereqs(class, skills, slot, &mut missing);
    missing
}

fn collect_missing_skill_prereqs(
    class: CharacterClass,
    skills: &[u8; 30],
    slot: usize,
    missing: &mut Vec<usize>,
) {
    if slot >= 30 {
        return;
    }

    for &prereq in skill_requirement(class, slot).prereqs {
        collect_missing_skill_prereqs(class, skills, prereq, missing);
        if skills[prereq] == 0 && !missing.contains(&prereq) {
            missing.push(prereq);
        }
    }
}

/// Returns whether `level` satisfies `slot` and all prerequisite level gates.
pub fn has_required_level_for_skill_tree(
    class: CharacterClass,
    level: u32,
    slot: usize,
    visited: &mut [bool; 30],
) -> bool {
    if slot >= 30 || visited[slot] {
        return true;
    }

    visited[slot] = true;
    let requirement = skill_requirement(class, slot);
    level >= requirement.level
        && requirement
            .prereqs
            .iter()
            .all(|&prereq| has_required_level_for_skill_tree(class, level, prereq, visited))
}

/// Returns the hard points needed to add one point to `slot`, including missing prerequisites.
pub fn skill_points_needed_to_increase(
    class: CharacterClass,
    skills: &[u8; 30],
    slot: usize,
) -> u32 {
    1 + missing_skill_prereqs(class, skills, slot).len() as u32
}

/// Returns whether one hard point can be added to `slot` under vanilla prerequisites.
pub fn can_increase_skill(
    class: CharacterClass,
    level: u32,
    skills: &[u8; 30],
    skill_points_remaining: u32,
    slot: usize,
) -> bool {
    if slot >= 30 || skills[slot] >= 20 {
        return false;
    }
    if !has_required_level_for_skill_tree(class, level, slot, &mut [false; 30]) {
        return false;
    }
    skill_points_remaining >= skill_points_needed_to_increase(class, skills, slot)
}

/// Adds one hard point to `slot`, auto-filling missing prerequisites when possible.
pub fn increase_skill(
    class: CharacterClass,
    level: u32,
    skills: &mut [u8; 30],
    skill_points_remaining: &mut u32,
    slot: usize,
) -> bool {
    if !can_increase_skill(class, level, skills, *skill_points_remaining, slot) {
        return false;
    }

    let missing = missing_skill_prereqs(class, skills, slot);
    let spent = 1 + missing.len() as u32;
    for prereq in missing {
        skills[prereq] = 1;
    }
    skills[slot] += 1;
    *skill_points_remaining -= spent;
    true
}

/// Returns whether `slot` has `dependency` in its prerequisite chain.
pub fn skill_depends_on(
    class: CharacterClass,
    slot: usize,
    dependency: usize,
    visited: &mut [bool; 30],
) -> bool {
    if slot >= 30 || visited[slot] {
        return false;
    }

    visited[slot] = true;
    for &prereq in skill_requirement(class, slot).prereqs {
        if prereq == dependency || skill_depends_on(class, prereq, dependency, visited) {
            return true;
        }
    }
    false
}

/// Returns whether an allocated skill depends on `slot` retaining a hard point.
pub fn has_allocated_dependent_skill(
    class: CharacterClass,
    skills: &[u8; 30],
    slot: usize,
) -> bool {
    (0..30).any(|other| {
        other != slot && skills[other] > 0 && skill_depends_on(class, other, slot, &mut [false; 30])
    })
}

/// Returns whether one hard point can be removed from `slot` without breaking prerequisites.
pub fn can_decrease_skill(class: CharacterClass, skills: &[u8; 30], slot: usize) -> bool {
    if slot >= 30 || skills[slot] == 0 {
        return false;
    }
    skills[slot] > 1 || !has_allocated_dependent_skill(class, skills, slot)
}

/// Removes one hard point from `slot` when doing so keeps dependent skills valid.
pub fn decrease_skill(
    class: CharacterClass,
    skills: &mut [u8; 30],
    skill_points_remaining: &mut u32,
    slot: usize,
) -> bool {
    if !can_decrease_skill(class, skills, slot) {
        return false;
    }

    skills[slot] -= 1;
    *skill_points_remaining += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_categories_use_class_tree_names_and_slots() {
        let categories = skill_categories(CharacterClass::Paladin);

        assert_eq!(categories[0].name, "Defensive Auras");
        assert_eq!(categories[1].name, "Offensive Auras");
        assert_eq!(categories[2].name, "Combat Skills");
        assert_eq!(categories[2].slots, &[0, 1, 5, 10, 11, 15, 16, 20, 21, 25]);
    }

    #[test]
    fn paladin_holy_shield_adds_recursive_prerequisites() {
        let mut skills = [0; 30];
        let mut remaining = 98;

        assert!(increase_skill(
            CharacterClass::Paladin,
            99,
            &mut skills,
            &mut remaining,
            21
        ));

        for slot in [1, 5, 11, 16, 21] {
            assert_eq!(
                skills[slot],
                1,
                "{} should receive one hard point",
                skill_name(CharacterClass::Paladin, slot)
            );
        }
        assert_eq!(remaining, 93);
    }

    #[test]
    fn advanced_skill_requires_enough_points_for_prerequisites() {
        let mut skills = [0; 30];
        let mut remaining = 4;

        assert!(!can_increase_skill(
            CharacterClass::Paladin,
            99,
            &skills,
            remaining,
            21
        ));
        assert!(!increase_skill(
            CharacterClass::Paladin,
            99,
            &mut skills,
            &mut remaining,
            21
        ));

        assert_eq!(skills.iter().copied().sum::<u8>(), 0);
        assert_eq!(remaining, 4);
    }

    #[test]
    fn last_prerequisite_point_cannot_be_removed_while_dependent_is_allocated() {
        let mut skills = [0; 30];
        let mut remaining = 98;
        increase_skill(CharacterClass::Paladin, 99, &mut skills, &mut remaining, 21);

        assert!(!can_decrease_skill(CharacterClass::Paladin, &skills, 1));
        assert!(!decrease_skill(
            CharacterClass::Paladin,
            &mut skills,
            &mut remaining,
            1
        ));

        assert_eq!(skills[1], 1);
        assert_eq!(remaining, 93);
    }

    #[test]
    fn extra_prerequisite_points_can_be_removed_back_to_one() {
        let mut skills = [0; 30];
        let mut remaining = 98;
        increase_skill(CharacterClass::Paladin, 99, &mut skills, &mut remaining, 21);
        increase_skill(CharacterClass::Paladin, 99, &mut skills, &mut remaining, 1);

        assert!(can_decrease_skill(CharacterClass::Paladin, &skills, 1));
        assert!(decrease_skill(
            CharacterClass::Paladin,
            &mut skills,
            &mut remaining,
            1
        ));

        assert_eq!(skills[1], 1);
        assert_eq!(remaining, 93);
    }
}
