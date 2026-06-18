/// Number of legacy waypoint bits stored per difficulty.
pub const WAYPOINT_COUNT: usize = 39;
/// Total bytes reserved for one difficulty in the legacy waypoint section.
pub const LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY: usize = 24;
/// Header bytes before waypoint bit data for one difficulty.
pub const LEGACY_WAYPOINT_DIFFICULTY_HEADER_BYTES: usize = 2;
/// Bytes needed to store all waypoint bits for one difficulty.
pub const LEGACY_WAYPOINT_DATA_BYTES: usize = WAYPOINT_COUNT.div_ceil(8);
/// Bytes from the `WS` marker through the fixed waypoint-section header.
pub const LEGACY_WAYPOINT_SECTION_HEADER_BYTES: usize = 8;
/// Legacy waypoint-section marker.
pub const LEGACY_WAYPOINT_SECTION_MARKER: [u8; 2] = *b"WS";
/// Fixed bytes after `WS` in legacy waypoint sections.
pub const LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER: [u8; 6] =
    [0x06, 0x00, 0x00, 0x00, 0x50, 0x00];
/// Fixed bytes after `WS` in Resurrected v105/RotW waypoint sections.
pub const V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER: [u8; 6] = [0x01, 0x00, 0x00, 0x00, 0x50, 0x00];
/// Legacy waypoint-section trailer byte offset from the `WS` marker.
pub const LEGACY_WAYPOINT_TRAILER_OFFSET: usize =
    LEGACY_WAYPOINT_SECTION_HEADER_BYTES + 3 * LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY;
/// Legacy waypoint-section trailer byte.
pub const LEGACY_WAYPOINT_TRAILER: u8 = 0x01;

/// Packet-derived local-player waypoint state.
///
/// D2GS `0x63 WaypointMenu` exposes the waypoint bits for the difficulty whose
/// waypoint menu was opened. Difficulties that were not observed stay empty
/// instead of being guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerWaypointState {
    difficulties: [[bool; WAYPOINT_COUNT]; 3],
}

impl Default for PlayerWaypointState {
    fn default() -> Self {
        Self {
            difficulties: [[false; WAYPOINT_COUNT]; 3],
        }
    }
}

impl PlayerWaypointState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_menu_bits(bits: [u8; 8]) -> [bool; WAYPOINT_COUNT] {
        let mut waypoints = [false; WAYPOINT_COUNT];
        for (idx, waypoint) in waypoints.iter_mut().enumerate() {
            let byte_idx = idx / 8;
            let bit_idx = idx % 8;
            *waypoint = bits[byte_idx] & (1 << bit_idx) != 0;
        }
        waypoints
    }

    pub fn set_difficulty_from_menu_bits(
        &mut self,
        difficulty_index: usize,
        bits: [u8; 8],
    ) -> bool {
        let Some(waypoints) = self.difficulties.get_mut(difficulty_index) else {
            return false;
        };
        *waypoints = Self::from_menu_bits(bits);
        true
    }

    pub fn as_difficulties(&self) -> &[[bool; WAYPOINT_COUNT]; 3] {
        &self.difficulties
    }

    pub fn difficulty(&self, difficulty_index: usize) -> Option<&[bool; WAYPOINT_COUNT]> {
        self.difficulties.get(difficulty_index)
    }
}

/// Display names for legacy waypoints in save-bit order.
pub const WAYPOINT_NAMES: [&str; WAYPOINT_COUNT] = [
    "Rogue Encampment",
    "Cold Plains",
    "Stony Field",
    "Dark Wood",
    "Black Marsh",
    "Outer Cloister",
    "Jail Level 1",
    "Inner Cloister",
    "Catacombs Level 2",
    "Lut Gholein",
    "Sewers Level 2",
    "Dry Hills",
    "Halls of the Dead L2",
    "Far Oasis",
    "Lost City",
    "Palace Cellar L1",
    "Arcane Sanctuary",
    "Canyon of the Magi",
    "Kurast Docks",
    "Spider Forest",
    "Great Marsh",
    "Flayer Jungle",
    "Lower Kurast",
    "Kurast Bazaar",
    "Upper Kurast",
    "Travincal",
    "Durance of Hate L2",
    "Pandemonium Fortress",
    "City of the Damned",
    "River of Flame",
    "Harrogath",
    "Frigid Highlands",
    "Arreat Plateau",
    "Crystalline Passage",
    "Halls of Pain",
    "Glacial Trail",
    "Frozen Tundra",
    "Ancients' Way",
    "Worldstone Keep L2",
];

/// Inclusive-start, exclusive-end waypoint range for one act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointAct {
    /// Act display name.
    pub name: &'static str,
    /// First waypoint index in this act.
    pub start: usize,
    /// One past the last waypoint index in this act.
    pub end: usize,
}

impl WaypointAct {
    /// Returns the waypoint indices for this act.
    pub fn indices(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// Waypoint groups in act order.
pub const WAYPOINT_ACTS: [WaypointAct; 5] = [
    WaypointAct {
        name: "Act I",
        start: 0,
        end: 9,
    },
    WaypointAct {
        name: "Act II",
        start: 9,
        end: 18,
    },
    WaypointAct {
        name: "Act III",
        start: 18,
        end: 27,
    },
    WaypointAct {
        name: "Act IV",
        start: 27,
        end: 30,
    },
    WaypointAct {
        name: "Act V",
        start: 30,
        end: WAYPOINT_COUNT,
    },
];

/// Parses legacy `.d2s` waypoint flags from a raw save buffer.
pub fn parse_legacy_waypoints(
    raw: &[u8],
    start_offset: usize,
) -> Option<[[bool; WAYPOINT_COUNT]; 3]> {
    let search_space = raw.get(start_offset..)?;
    let marker = search_space
        .windows(LEGACY_WAYPOINT_SECTION_MARKER.len())
        .position(|window| window == LEGACY_WAYPOINT_SECTION_MARKER)?
        + start_offset;
    let mut offset = marker + LEGACY_WAYPOINT_SECTION_HEADER_BYTES;
    let mut waypoints = [[false; WAYPOINT_COUNT]; 3];

    for difficulty in &mut waypoints {
        if offset + LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY <= raw.len() {
            let data_offset = offset + LEGACY_WAYPOINT_DIFFICULTY_HEADER_BYTES;
            for (idx, waypoint) in difficulty.iter_mut().enumerate() {
                let byte_idx = idx / 8;
                let bit_idx = idx % 8;
                *waypoint = raw[data_offset + byte_idx] & (1 << bit_idx) != 0;
            }
            offset += LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY;
        }
    }

    Some(waypoints)
}

/// Writes legacy `.d2s` waypoint flags into a raw save buffer when the section exists.
pub fn write_legacy_waypoints(
    raw: &mut [u8],
    start_offset: usize,
    waypoints: &[[bool; WAYPOINT_COUNT]; 3],
) -> bool {
    write_waypoints_with_header(
        raw,
        start_offset,
        waypoints,
        LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER,
    )
}

/// Writes v105 `.d2s` waypoint flags while preserving the v105 section header.
pub fn write_v105_waypoints(
    raw: &mut [u8],
    start_offset: usize,
    waypoints: &[[bool; WAYPOINT_COUNT]; 3],
) -> bool {
    write_waypoints_with_header(
        raw,
        start_offset,
        waypoints,
        V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER,
    )
}

fn write_waypoints_with_header(
    raw: &mut [u8],
    start_offset: usize,
    waypoints: &[[bool; WAYPOINT_COUNT]; 3],
    header_after_marker: [u8; 6],
) -> bool {
    let Some(search_space) = raw.get(start_offset..) else {
        return false;
    };
    let Some(marker_rel) = search_space
        .windows(LEGACY_WAYPOINT_SECTION_MARKER.len())
        .position(|window| window == LEGACY_WAYPOINT_SECTION_MARKER)
    else {
        return false;
    };
    let marker = marker_rel + start_offset;

    if marker + LEGACY_WAYPOINT_SECTION_HEADER_BYTES <= raw.len() {
        raw[marker + LEGACY_WAYPOINT_SECTION_MARKER.len()
            ..marker + LEGACY_WAYPOINT_SECTION_HEADER_BYTES]
            .copy_from_slice(&header_after_marker);
    }

    let mut offset = marker + LEGACY_WAYPOINT_SECTION_HEADER_BYTES;
    for difficulty in waypoints {
        if offset + LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY <= raw.len() {
            raw[offset] = 0x02;
            raw[offset + 1] = 0x01;
            let data_offset = offset + LEGACY_WAYPOINT_DIFFICULTY_HEADER_BYTES;
            raw[data_offset..data_offset + LEGACY_WAYPOINT_DATA_BYTES].fill(0);

            for (idx, waypoint) in difficulty.iter().enumerate() {
                if *waypoint {
                    let byte_idx = idx / 8;
                    let bit_idx = idx % 8;
                    raw[data_offset + byte_idx] |= 1 << bit_idx;
                }
            }
            offset += LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY;
        }
    }

    if let Some(trailer) = raw.get_mut(marker + LEGACY_WAYPOINT_TRAILER_OFFSET) {
        *trailer = LEGACY_WAYPOINT_TRAILER;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER, LEGACY_WAYPOINT_SECTION_HEADER_BYTES,
        LEGACY_WAYPOINT_SECTION_MARKER, LEGACY_WAYPOINT_TRAILER, LEGACY_WAYPOINT_TRAILER_OFFSET,
        PlayerWaypointState, V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER, WAYPOINT_ACTS,
        WAYPOINT_COUNT, WAYPOINT_NAMES, parse_legacy_waypoints, write_legacy_waypoints,
        write_v105_waypoints,
    };

    #[test]
    fn waypoint_names_match_legacy_bit_count() {
        assert_eq!(WAYPOINT_NAMES.len(), WAYPOINT_COUNT);
        assert_eq!(WAYPOINT_NAMES[0], "Rogue Encampment");
        assert_eq!(WAYPOINT_NAMES[30], "Harrogath");
        assert_eq!(WAYPOINT_NAMES[38], "Worldstone Keep L2");
    }

    #[test]
    fn waypoint_acts_cover_all_indices_once() {
        let indices = WAYPOINT_ACTS
            .iter()
            .flat_map(|act| act.indices())
            .collect::<Vec<_>>();

        assert_eq!(indices, (0..WAYPOINT_COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn legacy_waypoints_parse_and_write_round_trip() {
        let mut raw = vec![0u8; LEGACY_WAYPOINT_TRAILER_OFFSET + 1];
        raw[0..2].copy_from_slice(&LEGACY_WAYPOINT_SECTION_MARKER);
        raw[2..LEGACY_WAYPOINT_SECTION_HEADER_BYTES]
            .copy_from_slice(&LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER);

        let mut waypoints = [[false; WAYPOINT_COUNT]; 3];
        waypoints[0][0] = true;
        waypoints[1][17] = true;
        waypoints[2][38] = true;

        assert!(write_legacy_waypoints(&mut raw, 0, &waypoints));
        assert_eq!(raw[LEGACY_WAYPOINT_TRAILER_OFFSET], LEGACY_WAYPOINT_TRAILER);
        assert_eq!(parse_legacy_waypoints(&raw, 0), Some(waypoints));
    }

    #[test]
    fn v105_waypoints_preserve_v105_section_header() {
        let mut raw = vec![0u8; LEGACY_WAYPOINT_TRAILER_OFFSET + 1];
        raw[0..2].copy_from_slice(&LEGACY_WAYPOINT_SECTION_MARKER);
        raw[2..LEGACY_WAYPOINT_SECTION_HEADER_BYTES]
            .copy_from_slice(&V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER);
        let mut waypoints = [[false; WAYPOINT_COUNT]; 3];
        waypoints[2][38] = true;

        assert!(write_v105_waypoints(&mut raw, 0, &waypoints));

        assert_eq!(
            &raw[2..LEGACY_WAYPOINT_SECTION_HEADER_BYTES],
            &V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER
        );
        assert_eq!(raw[LEGACY_WAYPOINT_TRAILER_OFFSET], LEGACY_WAYPOINT_TRAILER);
        assert_eq!(parse_legacy_waypoints(&raw, 0), Some(waypoints));
    }

    #[test]
    fn player_waypoint_state_decodes_menu_bits_by_difficulty() {
        let mut state = PlayerWaypointState::new();

        assert!(
            state.set_difficulty_from_menu_bits(2, [0b0000_0001, 0, 0, 0, 0b0100_0000, 0, 0, 0])
        );
        assert!(state.difficulty(2).unwrap()[0]);
        assert!(state.difficulty(2).unwrap()[38]);
        assert!(!state.difficulty(0).unwrap()[0]);
        assert!(!state.set_difficulty_from_menu_bits(3, [0; 8]));
    }
}
