use std::fmt;
use std::sync::OnceLock;

const MPQ_MAGIC: &[u8; 4] = b"MPQ\x1a";
const HEADER_LEN: usize = 32;
const TABLE_ENTRY_LEN: usize = 16;
const ENCRYPTION_TABLE_LEN: usize = 0x500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum MpqHashType {
    TableOffset = 0,
    HashA = 1,
    HashB = 2,
    Table = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpqFormatVersion {
    Version1,
    Version2,
    Version3,
    Version4,
    Unknown(u16),
}

impl MpqFormatVersion {
    pub fn from_raw(raw: u16) -> Self {
        match raw {
            0 => Self::Version1,
            1 => Self::Version2,
            2 => Self::Version3,
            3 => Self::Version4,
            other => Self::Unknown(other),
        }
    }

    pub fn raw(self) -> u16 {
        match self {
            Self::Version1 => 0,
            Self::Version2 => 1,
            Self::Version3 => 2,
            Self::Version4 => 3,
            Self::Unknown(raw) => raw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpqCompressionType {
    Huffman,
    Zlib,
    PkWare,
    BZip2,
    Unknown(u8),
}

impl MpqCompressionType {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0x01 => Self::Huffman,
            0x02 => Self::Zlib,
            0x08 => Self::PkWare,
            0x10 => Self::BZip2,
            other => Self::Unknown(other),
        }
    }
}

pub struct MpqFileFlags;

impl MpqFileFlags {
    pub const IMPLODE: u32 = 0x0000_0100;
    pub const COMPRESSED: u32 = 0x0000_0200;
    pub const ENCRYPTED: u32 = 0x0001_0000;
    pub const ENCRYPTION_FIX: u32 = 0x0002_0000;
    pub const SINGLE_UNIT: u32 = 0x0100_0000;
    pub const CRC: u32 = 0x0400_0000;
    pub const EXISTS: u32 = 0x8000_0000;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpqHeader {
    pub header_size: u32,
    pub archive_size: u32,
    pub format_version: MpqFormatVersion,
    pub sector_size_shift: u16,
    pub hash_table_offset: u32,
    pub block_table_offset: u32,
    pub hash_table_entries: u32,
    pub block_table_entries: u32,
}

impl MpqHeader {
    pub fn parse(raw: &[u8]) -> Result<Self, MpqError> {
        if raw.len() < HEADER_LEN {
            return Err(MpqError::TooSmall {
                len: raw.len(),
                required: HEADER_LEN,
            });
        }

        if &raw[0..4] != MPQ_MAGIC {
            return Err(MpqError::BadMagic {
                found: [raw[0], raw[1], raw[2], raw[3]],
            });
        }

        Ok(Self {
            header_size: read_u32_le(raw, 0x04)?,
            archive_size: read_u32_le(raw, 0x08)?,
            format_version: MpqFormatVersion::from_raw(read_u16_le(raw, 0x0c)?),
            sector_size_shift: read_u16_le(raw, 0x0e)?,
            hash_table_offset: read_u32_le(raw, 0x10)?,
            block_table_offset: read_u32_le(raw, 0x14)?,
            hash_table_entries: read_u32_le(raw, 0x18)?,
            block_table_entries: read_u32_le(raw, 0x1c)?,
        })
    }

    pub fn sector_size(self) -> u32 {
        512u32 << self.sector_size_shift
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpqHashEntry {
    pub hash_a: u32,
    pub hash_b: u32,
    pub locale: u16,
    pub platform: u16,
    pub block_table_index: u32,
}

impl MpqHashEntry {
    pub fn parse(raw: &[u8]) -> Result<Self, MpqError> {
        if raw.len() < TABLE_ENTRY_LEN {
            return Err(MpqError::TooSmall {
                len: raw.len(),
                required: TABLE_ENTRY_LEN,
            });
        }

        Ok(Self {
            hash_a: read_u32_le(raw, 0x00)?,
            hash_b: read_u32_le(raw, 0x04)?,
            locale: read_u16_le(raw, 0x08)?,
            platform: read_u16_le(raw, 0x0a)?,
            block_table_index: read_u32_le(raw, 0x0c)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpqBlockEntry {
    pub offset: u32,
    pub archive_size: u32,
    pub size: u32,
    pub flags: u32,
}

impl MpqBlockEntry {
    pub fn parse(raw: &[u8]) -> Result<Self, MpqError> {
        if raw.len() < TABLE_ENTRY_LEN {
            return Err(MpqError::TooSmall {
                len: raw.len(),
                required: TABLE_ENTRY_LEN,
            });
        }

        Ok(Self {
            offset: read_u32_le(raw, 0x00)?,
            archive_size: read_u32_le(raw, 0x04)?,
            size: read_u32_le(raw, 0x08)?,
            flags: read_u32_le(raw, 0x0c)?,
        })
    }

    pub fn has_flag(self, flag: u32) -> bool {
        self.flags & flag == flag
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpqError {
    TooSmall { len: usize, required: usize },
    BadMagic { found: [u8; 4] },
    InvalidDecryptRange { offset: usize, size: usize },
}

impl fmt::Display for MpqError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall { len, required } => {
                write!(
                    formatter,
                    "MPQ data is {} bytes, need at least {}",
                    len, required
                )
            }
            Self::BadMagic { found } => write!(formatter, "bad MPQ magic: {:02x?}", found),
            Self::InvalidDecryptRange { offset, size } => write!(
                formatter,
                "invalid MPQ decrypt range offset={} size={}",
                offset, size
            ),
        }
    }
}

impl std::error::Error for MpqError {}

pub fn mpq_hash(path: &str, hash_type: MpqHashType) -> u32 {
    let normalized = normalize_mpq_path(path);
    let table = encryption_table();
    let mut seed1 = 0x7fed_7fedu32;
    let mut seed2 = 0xeeee_eeeeu32;

    for byte in normalized.bytes() {
        let byte = byte.to_ascii_uppercase();
        let value = table[(hash_type as usize * 0x100) + byte as usize];
        seed1 = value ^ seed1.wrapping_add(seed2);
        seed2 = (byte as u32)
            .wrapping_add(seed1)
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
    }

    seed1
}

pub fn mpq_decryption_key(path: &str) -> u32 {
    let file_name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    mpq_hash(file_name, MpqHashType::Table)
}

pub fn decrypt_mpq_block(data: &mut [u8], key: u32) -> Result<(), MpqError> {
    decrypt_mpq_block_range(data, key, 0, data.len())
}

pub fn decrypt_mpq_block_range(
    data: &mut [u8],
    key: u32,
    offset: usize,
    size: usize,
) -> Result<(), MpqError> {
    if offset
        .checked_add(size)
        .filter(|end| *end <= data.len())
        .is_none()
        || size % 4 != 0
    {
        return Err(MpqError::InvalidDecryptRange { offset, size });
    }

    let table = encryption_table();
    let mut hash = key;
    let mut seed = 0xeeee_eeeeu32;
    let end = offset + size;
    let mut cursor = offset;

    while cursor < end {
        seed = seed.wrapping_add(table[0x400 + (hash & 0xff) as usize]);

        let mut bytes = [0; 4];
        bytes.copy_from_slice(&data[cursor..cursor + 4]);
        let mut value = u32::from_le_bytes(bytes);
        value ^= hash.wrapping_add(seed);
        data[cursor..cursor + 4].copy_from_slice(&value.to_le_bytes());

        hash = (!hash).wrapping_shl(0x15).wrapping_add(0x1111_1111) | (hash >> 0x0b);
        seed = value
            .wrapping_add(seed)
            .wrapping_add(seed << 5)
            .wrapping_add(3);
        cursor += 4;
    }

    Ok(())
}

pub fn encryption_table() -> &'static [u32; ENCRYPTION_TABLE_LEN] {
    static TABLE: OnceLock<[u32; ENCRYPTION_TABLE_LEN]> = OnceLock::new();
    TABLE.get_or_init(generate_encryption_table)
}

fn normalize_mpq_path(path: &str) -> String {
    path.replace('/', "\\")
}

fn generate_encryption_table() -> [u32; ENCRYPTION_TABLE_LEN] {
    let mut table = [0; ENCRYPTION_TABLE_LEN];
    let mut seed = 0x0010_0001u32;

    for index in 0..0x100 {
        let mut table_index = index;
        for _ in 0..5 {
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let high = (seed & 0xffff) << 0x10;
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let low = seed & 0xffff;
            table[table_index] = high | low;
            table_index += 0x100;
        }
    }

    table
}

fn read_u32_le(raw: &[u8], offset: usize) -> Result<u32, MpqError> {
    if raw.len() < offset + 4 {
        return Err(MpqError::TooSmall {
            len: raw.len(),
            required: offset + 4,
        });
    }
    Ok(u32::from_le_bytes([
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ]))
}

fn read_u16_le(raw: &[u8], offset: usize) -> Result<u16, MpqError> {
    if raw.len() < offset + 2 {
        return Err(MpqError::TooSmall {
            len: raw.len(),
            required: offset + 2,
        });
    }
    Ok(u16::from_le_bytes([raw[offset], raw[offset + 1]]))
}

#[cfg(test)]
mod tests {
    use super::{
        decrypt_mpq_block, encryption_table, mpq_decryption_key, mpq_hash, MpqBlockEntry,
        MpqFileFlags, MpqFormatVersion, MpqHashEntry, MpqHashType, MpqHeader,
    };

    #[test]
    fn generated_encryption_table_matches_known_prefix() {
        let table = encryption_table();

        assert_eq!(table[0], 0x55c6_36e2);
        assert_eq!(table[1], 0x02be_0170);
        assert_eq!(table[2], 0x584b_71d4);
        assert_eq!(table[0x400], 0x193a_a698);
    }

    #[test]
    fn hashes_match_blacha_mpq_vectors() {
        assert_eq!(mpq_hash("hello world", MpqHashType::HashA), 0xef7b_1bdb);
        assert_eq!(mpq_hash("HELLO world", MpqHashType::HashA), 0xef7b_1bdb);
        assert_eq!(mpq_hash("hello world", MpqHashType::HashB), 0x4ba6_f4b8);
        assert_eq!(mpq_hash("hello world", MpqHashType::Table), 0x59c9_0699);
        assert_eq!(
            mpq_hash("hello world", MpqHashType::TableOffset),
            0xfeb4_1edd
        );
        assert_eq!(mpq_hash("(hash table)", MpqHashType::Table), 0xc3af_3770);
        assert_eq!(mpq_hash("(block table)", MpqHashType::Table), 0xec83_b3a3);
        assert_eq!(mpq_hash("(listfile)", MpqHashType::HashA), 0xfd65_7910);
        assert_eq!(mpq_hash("(listfile)", MpqHashType::HashB), 0x4e9b_98a7);
        assert_eq!(
            mpq_hash(
                "data\\local\\LNG\\ENG\\expansionstring.tbl",
                MpqHashType::HashA
            ),
            0xaaf4_7edd
        );
        assert_eq!(
            mpq_hash("data/local/LNG/ENG/expansionstring.tbl", MpqHashType::HashA),
            0xaaf4_7edd
        );
    }

    #[test]
    fn decryption_keys_use_file_name_only() {
        assert_eq!(
            mpq_decryption_key("data\\global\\excel\\monstats.bin"),
            0xdee9_de0b
        );
        assert_eq!(
            mpq_decryption_key("data\\global\\excel\\Armor.bin"),
            0xfd72_85c8
        );
        assert_eq!(
            mpq_decryption_key("data\\global\\excel\\armor.bin"),
            0xfd72_85c8
        );
    }

    #[test]
    fn decrypt_block_matches_blacha_table_vector_prefix() {
        let mut data = [
            0x5b, 0x63, 0x48, 0x3d, 0x34, 0xda, 0x08, 0xca, 0x8d, 0x55, 0x36, 0xf8, 0xe3, 0xb1,
            0x47, 0xe9, 0x7f, 0x1d, 0x3f, 0xb6, 0xa4, 0x87, 0xc5, 0x64, 0x36, 0x04, 0x7f, 0xe2,
            0xe6, 0x64, 0x0b, 0x3d,
        ];
        let expected = [
            0xd0, 0x04, 0x00, 0x00, 0x65, 0x02, 0x00, 0x00, 0x65, 0x02, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x81, 0x45, 0x07, 0x00, 0x00, 0x1f, 0x02, 0x00, 0x00, 0x1f, 0x02, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x81,
        ];

        decrypt_mpq_block(&mut data, 0xec83_b3a3).expect("decrypt pass");

        assert_eq!(data, expected);
    }

    #[test]
    fn parses_v1_header_and_table_entries() {
        let mut header = [0; 32];
        header[0..4].copy_from_slice(b"MPQ\x1a");
        header[4..8].copy_from_slice(&32u32.to_le_bytes());
        header[8..12].copy_from_slice(&59_335u32.to_le_bytes());
        header[12..14].copy_from_slice(&0u16.to_le_bytes());
        header[14..16].copy_from_slice(&3u16.to_le_bytes());
        header[16..20].copy_from_slice(&128u32.to_le_bytes());
        header[20..24].copy_from_slice(&256u32.to_le_bytes());
        header[24..28].copy_from_slice(&16u32.to_le_bytes());
        header[28..32].copy_from_slice(&4u32.to_le_bytes());

        let parsed = MpqHeader::parse(&header).expect("header should parse");
        assert_eq!(parsed.header_size, 32);
        assert_eq!(parsed.archive_size, 59_335);
        assert_eq!(parsed.format_version, MpqFormatVersion::Version1);
        assert_eq!(parsed.sector_size(), 4096);

        let hash = MpqHashEntry::parse(&[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 4, 0, 5, 0, 0, 0])
            .expect("hash entry should parse");
        assert_eq!(hash.hash_a, 1);
        assert_eq!(hash.hash_b, 2);
        assert_eq!(hash.locale, 3);
        assert_eq!(hash.platform, 4);
        assert_eq!(hash.block_table_index, 5);

        let block = MpqBlockEntry::parse(&[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 2, 0, 0])
            .expect("block entry should parse");
        assert_eq!(block.offset, 1);
        assert_eq!(block.archive_size, 2);
        assert_eq!(block.size, 3);
        assert!(block.has_flag(MpqFileFlags::COMPRESSED));
    }
}
