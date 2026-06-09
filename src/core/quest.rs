/// Packet-derived local-player quest-log state for one difficulty.
///
/// Legacy D2GS packet `0x52 PlayerQuestLog` carries 41 bytes. That count matches
/// the 41 two-byte quest-log words in the legacy `.d2s` quest section for one
/// difficulty, so this type models the packet as the low byte of each quest-log
/// slot in save-file order. That gives stable names for mandatory quests, optional
/// quests, act-travel markers, and the few non-quest slots preserved by the
/// original layout without inventing richer semantics than the packet exposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestLog {
    entries: [u8; Self::ENTRY_COUNT],
}

impl PlayerQuestLog {
    pub const ENTRY_COUNT: usize = 41;

    pub fn new(entries: [u8; Self::ENTRY_COUNT]) -> Self {
        Self { entries }
    }

    pub fn raw_entries(&self) -> &[u8; Self::ENTRY_COUNT] {
        &self.entries
    }

    pub fn entry(&self, entry: QuestLogEntry) -> QuestLogEntryState {
        QuestLogEntryState::new(self.entries[entry.index()])
    }
}

impl Default for PlayerQuestLog {
    fn default() -> Self {
        Self {
            entries: [0; Self::ENTRY_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogEntryState {
    raw: u8,
}

impl QuestLogEntryState {
    pub const COMPLETED_BIT: u8 = 1 << 0;
    pub const REQUIREMENT_COMPLETED_BIT: u8 = 1 << 1;

    pub const fn new(raw: u8) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u8 {
        self.raw
    }

    pub const fn is_set(self) -> bool {
        self.raw != 0
    }

    pub const fn is_completed(self) -> bool {
        self.raw & Self::COMPLETED_BIT != 0
    }

    pub const fn is_requirement_completed(self) -> bool {
        self.raw & Self::REQUIREMENT_COMPLETED_BIT != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuestLogEntry {
    ActIIntroduction = 0,
    DenOfEvil = 1,
    SistersBurialGrounds = 2,
    ToolsOfTheTrade = 3,
    TheSearchForCain = 4,
    TheForgottenTower = 5,
    SistersToTheSlaughter = 6,
    TravelToActII = 7,
    ActIIIntroduction = 8,
    RadamentsLair = 9,
    TheHoradricStaff = 10,
    TaintedSun = 11,
    ArcaneSanctuary = 12,
    TheSummoner = 13,
    TheSevenTombs = 14,
    TravelToActIII = 15,
    ActIIIIntroduction = 16,
    LamEsensTome = 17,
    KhalimsWill = 18,
    BladeOfTheOldReligion = 19,
    TheGoldenBird = 20,
    TheBlackenedTemple = 21,
    TheGuardian = 22,
    TravelToActIV = 23,
    ActIVIntroduction = 24,
    TheFallenAngel = 25,
    TerrorsEnd = 26,
    Hellforge = 27,
    ActIVUnused1 = 28,
    ActIVUnused2 = 29,
    ActIVUnused3 = 30,
    TravelToActV = 31,
    PostTerrorsEndCain = 32,
    ActVPadding1 = 33,
    ActVPadding2 = 34,
    SiegeOnHarrogath = 35,
    RescueOnMountArreat = 36,
    PrisonOfIce = 37,
    BetrayalOfHarrogath = 38,
    RiteOfPassage = 39,
    EveOfDestruction = 40,
}

impl QuestLogEntry {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerQuestLog, QuestLogEntry, QuestLogEntryState};

    #[test]
    fn player_quest_log_exposes_named_entries() {
        let mut entries = [0u8; PlayerQuestLog::ENTRY_COUNT];
        entries[QuestLogEntry::TheSearchForCain.index()] =
            QuestLogEntryState::COMPLETED_BIT | QuestLogEntryState::REQUIREMENT_COMPLETED_BIT;

        let log = PlayerQuestLog::new(entries);
        let cain = log.entry(QuestLogEntry::TheSearchForCain);

        assert!(cain.is_completed());
        assert!(cain.is_requirement_completed());
        assert_eq!(cain.raw(), 0b11);
    }
}
