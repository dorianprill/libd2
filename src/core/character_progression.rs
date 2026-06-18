use crate::core::character_class::CharacterClass;

/// Highest character level supported by vanilla Diablo II character saves.
pub const MAX_CHARACTER_LEVEL: u32 = 99;

/// Maximum stat value that fits the 25-bit legacy gold save stat.
pub const LEGACY_GOLD_MAX_ENCODED: u32 = (1 << 25) - 1;

/// Canonical cumulative Lord of Destruction experience breakpoints for levels 1-99.
pub const EXPERIENCE_BY_LEVEL: [u32; MAX_CHARACTER_LEVEL as usize] = [
    0,
    500,
    1_500,
    3_750,
    7_875,
    14_175,
    22_680,
    32_886,
    44_396,
    57_715,
    72_144,
    90_180,
    112_725,
    140_906,
    176_132,
    220_165,
    275_207,
    344_008,
    430_010,
    537_513,
    671_891,
    839_864,
    1_049_830,
    1_312_287,
    1_640_359,
    2_050_449,
    2_563_061,
    3_203_826,
    3_902_260,
    4_663_553,
    5_493_363,
    6_397_855,
    7_383_752,
    8_458_379,
    9_629_723,
    10_906_488,
    12_298_162,
    13_815_086,
    15_468_534,
    17_270_791,
    19_235_252,
    21_376_515,
    23_710_491,
    26_254_525,
    29_027_522,
    32_050_088,
    35_344_686,
    38_935_798,
    42_850_109,
    47_116_709,
    51_767_302,
    56_836_449,
    62_361_819,
    68_384_473,
    74_949_165,
    82_104_680,
    89_904_191,
    98_405_658,
    107_672_256,
    117_772_849,
    128_782_495,
    140_783_010,
    153_863_570,
    168_121_381,
    183_662_396,
    200_602_101,
    219_066_380,
    239_192_444,
    261_129_853,
    285_041_630,
    311_105_466,
    339_515_048,
    370_481_492,
    404_234_916,
    441_026_148,
    481_128_591,
    524_840_254,
    572_485_967,
    624_419_793,
    681_027_665,
    742_730_244,
    809_986_056,
    883_294_891,
    963_201_521,
    1_050_299_747,
    1_145_236_814,
    1_248_718_217,
    1_361_512_946,
    1_484_459_201,
    1_618_470_619,
    1_764_543_065,
    1_923_762_030,
    2_097_310_703,
    2_286_478_756,
    2_492_671_933,
    2_717_422_497,
    2_962_400_612,
    3_229_426_756,
    3_520_485_254,
];

/// Base class stats and starting resources from vanilla LoD `CharStats.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseStats {
    pub str: u32,
    pub dex: u32,
    pub vit: u32,
    pub eng: u32,
    pub hp: u32,
    pub mana: u32,
    pub stamina: u32,
}

impl BaseStats {
    /// Returns the starting stats and resources for `class`.
    pub const fn for_class(class: CharacterClass) -> Self {
        match class {
            CharacterClass::Amazon => Self {
                str: 20,
                dex: 25,
                vit: 20,
                eng: 15,
                hp: 50,
                mana: 15,
                stamina: 84,
            },
            CharacterClass::Sorceress => Self {
                str: 10,
                dex: 25,
                vit: 10,
                eng: 35,
                hp: 40,
                mana: 35,
                stamina: 74,
            },
            CharacterClass::Necromancer => Self {
                str: 15,
                dex: 25,
                vit: 15,
                eng: 25,
                hp: 45,
                mana: 25,
                stamina: 79,
            },
            CharacterClass::Paladin => Self {
                str: 25,
                dex: 20,
                vit: 25,
                eng: 15,
                hp: 55,
                mana: 15,
                stamina: 89,
            },
            CharacterClass::Barbarian => Self {
                str: 30,
                dex: 20,
                vit: 25,
                eng: 10,
                hp: 55,
                mana: 10,
                stamina: 92,
            },
            CharacterClass::Druid => Self {
                str: 15,
                dex: 20,
                vit: 25,
                eng: 20,
                hp: 55,
                mana: 20,
                stamina: 84,
            },
            CharacterClass::Assassin => Self {
                str: 20,
                dex: 20,
                vit: 20,
                eng: 25,
                hp: 50,
                mana: 25,
                stamina: 95,
            },
            CharacterClass::Warlock => Self {
                str: 15,
                dex: 20,
                vit: 25,
                eng: 20,
                hp: 55,
                mana: 20,
                stamina: 86,
            },
        }
    }
}

/// Quarter-point resource growth values from vanilla LoD `CharStats.txt`.
///
/// The game data represents half/quarter resource changes as integer quarters.
/// For example, `6` means `1.5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassGrowth {
    pub life_per_level_quarters: u16,
    pub stamina_per_level_quarters: u16,
    pub mana_per_level_quarters: u16,
    pub life_per_vitality_quarters: u16,
    pub stamina_per_vitality_quarters: u16,
    pub mana_per_energy_quarters: u16,
    pub stat_points_per_level: u32,
    pub skill_points_per_level: u32,
}

impl ClassGrowth {
    /// Converts quarter-point values to whole resource units by truncating
    /// toward zero, matching callers that only store whole displayed values.
    pub const fn whole_units(quarters: u16) -> u32 {
        quarters as u32 / 4
    }

    /// Whole life gained by one allocated vitality point.
    pub const fn whole_life_per_vitality(self) -> u32 {
        Self::whole_units(self.life_per_vitality_quarters)
    }

    /// Whole stamina gained by one allocated vitality point.
    pub const fn whole_stamina_per_vitality(self) -> u32 {
        Self::whole_units(self.stamina_per_vitality_quarters)
    }

    /// Whole mana gained by one allocated energy point.
    pub const fn whole_mana_per_energy(self) -> u32 {
        Self::whole_units(self.mana_per_energy_quarters)
    }

    pub fn life_for_level(&self, level: u32) -> u32 {
        (level.saturating_sub(1) * self.life_per_level_quarters as u32) / 4
    }

    pub fn mana_for_level(&self, level: u32) -> u32 {
        (level.saturating_sub(1) * self.mana_per_level_quarters as u32) / 4
    }

    pub fn stamina_for_level(&self, level: u32) -> u32 {
        (level.saturating_sub(1) * self.stamina_per_level_quarters as u32) / 4
    }

    pub fn life_for_vitality(&self, vitality: u32, base_vitality: u32) -> u32 {
        (vitality.saturating_sub(base_vitality) * self.life_per_vitality_quarters as u32) / 4
    }

    pub fn mana_for_energy(&self, energy: u32, base_energy: u32) -> u32 {
        (energy.saturating_sub(base_energy) * self.mana_per_energy_quarters as u32) / 4
    }

    pub fn stamina_for_vitality(&self, vitality: u32, base_vitality: u32) -> u32 {
        (vitality.saturating_sub(base_vitality) * self.stamina_per_vitality_quarters as u32) / 4
    }

    /// Returns class-specific resource and point growth for `class`.
    pub const fn for_class(class: CharacterClass) -> Self {
        match class {
            CharacterClass::Amazon => Self {
                life_per_level_quarters: 8,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 6,
                life_per_vitality_quarters: 12,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 6,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Sorceress => Self {
                life_per_level_quarters: 4,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 8,
                life_per_vitality_quarters: 8,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 8,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Necromancer => Self {
                life_per_level_quarters: 6,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 8,
                life_per_vitality_quarters: 8,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 8,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Paladin => Self {
                life_per_level_quarters: 8,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 6,
                life_per_vitality_quarters: 12,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 6,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Barbarian => Self {
                life_per_level_quarters: 8,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 4,
                life_per_vitality_quarters: 16,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 4,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Druid => Self {
                life_per_level_quarters: 6,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 8,
                life_per_vitality_quarters: 8,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 8,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Warlock => Self {
                life_per_level_quarters: 12,
                stamina_per_level_quarters: 4,
                mana_per_level_quarters: 8,
                life_per_vitality_quarters: 12,
                stamina_per_vitality_quarters: 4,
                mana_per_energy_quarters: 8,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
            CharacterClass::Assassin => Self {
                life_per_level_quarters: 8,
                stamina_per_level_quarters: 5,
                mana_per_level_quarters: 6,
                life_per_vitality_quarters: 12,
                stamina_per_vitality_quarters: 5,
                mana_per_energy_quarters: 7,
                stat_points_per_level: 5,
                skill_points_per_level: 1,
            },
        }
    }
}

/// Returns the cumulative experience required for `level`, clamped to 1-99.
pub fn experience_for_level(level: u32) -> u32 {
    let index = level.clamp(1, MAX_CHARACTER_LEVEL) as usize - 1;
    EXPERIENCE_BY_LEVEL[index]
}

/// Returns stat points earned from level-ups, excluding quest rewards.
pub fn stat_points_from_level(level: u32) -> u32 {
    level
        .clamp(1, MAX_CHARACTER_LEVEL)
        .saturating_sub(1)
        .saturating_mul(ClassGrowth::for_class(CharacterClass::Amazon).stat_points_per_level)
}

/// Returns skill points earned from level-ups, excluding quest rewards.
pub fn skill_points_from_level(level: u32) -> u32 {
    level
        .clamp(1, MAX_CHARACTER_LEVEL)
        .saturating_sub(1)
        .saturating_mul(ClassGrowth::for_class(CharacterClass::Amazon).skill_points_per_level)
}

/// Returns the maximum inventory gold for a legacy character at `level`.
pub fn max_inventory_gold(level: u32) -> u32 {
    (level.clamp(1, MAX_CHARACTER_LEVEL) * 10_000).min(LEGACY_GOLD_MAX_ENCODED)
}

/// Returns the maximum stash gold for a legacy character at `level`.
pub fn max_stash_gold(level: u32) -> u32 {
    let level = level.clamp(1, MAX_CHARACTER_LEVEL);
    let multiplier = if level <= 30 {
        level / 10 + 1
    } else {
        level / 2 + 1
    };
    (multiplier * 50_000).min(LEGACY_GOLD_MAX_ENCODED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_table_matches_lord_of_destruction_levels() {
        assert_eq!(experience_for_level(1), 0);
        assert_eq!(experience_for_level(80), 681_027_665);
        assert_eq!(experience_for_level(81), 742_730_244);
        assert_eq!(experience_for_level(90), 1_618_470_619);
        assert_eq!(experience_for_level(95), 2_492_671_933);
        assert_eq!(experience_for_level(96), 2_717_422_497);
        assert_eq!(experience_for_level(97), 2_962_400_612);
        assert_eq!(experience_for_level(98), 3_229_426_756);
        assert_eq!(experience_for_level(99), 3_520_485_254);
        assert_eq!(experience_for_level(100), 3_520_485_254);
    }

    #[test]
    fn exposes_class_base_stats_and_growth() {
        let amazon = BaseStats::for_class(CharacterClass::Amazon);
        assert_eq!(amazon.str, 20);
        assert_eq!(amazon.dex, 25);
        assert_eq!(amazon.hp, 50);

        let sorceress = BaseStats::for_class(CharacterClass::Sorceress);
        assert_eq!(sorceress.str, 10);
        assert_eq!(sorceress.eng, 35);
        assert_eq!(sorceress.mana, 35);

        let barbarian = ClassGrowth::for_class(CharacterClass::Barbarian);
        assert_eq!(barbarian.life_per_vitality_quarters, 16);
        assert_eq!(barbarian.whole_life_per_vitality(), 4);

        let assassin = ClassGrowth::for_class(CharacterClass::Assassin);
        assert_eq!(assassin.stamina_per_level_quarters, 5);
        assert_eq!(assassin.mana_per_energy_quarters, 7);
    }

    #[test]
    fn legacy_gold_caps_match_in_game_limits() {
        assert_eq!(max_inventory_gold(1), 10_000);
        assert_eq!(max_inventory_gold(99), 990_000);
        assert_eq!(max_stash_gold(1), 50_000);
        assert_eq!(max_stash_gold(99), 2_500_000);
    }
}
