use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
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

impl fmt::Display for MpqCompressionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Huffman => formatter.write_str("huffman"),
            Self::Zlib => formatter.write_str("zlib"),
            Self::PkWare => formatter.write_str("pkware"),
            Self::BZip2 => formatter.write_str("bzip2"),
            Self::Unknown(raw) => write!(formatter, "unknown({raw:#04x})"),
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

/// Read-only MPQ v1 archive reader.
///
/// Diablo II Classic and Lord of Destruction store most static game data in
/// MPQ archives. File names are not stored in the hash table, so callers must
/// already know the logical archive path, for example
/// `data/global/excel/monstats.bin`. The reader resolves that path through the
/// encrypted hash table, decrypts sector tables and sectors when required, and
/// returns the decompressed file bytes.
///
/// The implementation is intentionally read-only. It never mutates the source
/// archive and is limited to the MPQ v1 format used by the legacy Diablo II
/// archives. Newer D2R/RotW asset containers need a separate loader.
#[derive(Debug, Clone)]
pub struct MpqArchive {
    name: String,
    source: MpqArchiveSource,
    header: MpqHeader,
    hash_table: HashMap<(u32, u32), MpqHashEntry>,
    block_table: Vec<MpqBlockEntry>,
}

#[derive(Debug, Clone)]
enum MpqArchiveSource {
    Path(PathBuf),
    Memory(Vec<u8>),
}

impl MpqArchive {
    /// Opens an MPQ archive from disk and eagerly reads only its header and
    /// encrypted hash/block tables.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MpqError> {
        let path = path.as_ref().to_path_buf();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mpq")
            .to_owned();
        Self::load(name, MpqArchiveSource::Path(path))
    }

    /// Loads an MPQ archive from in-memory bytes.
    ///
    /// This is mainly useful for small test fixtures and callers that already
    /// have archive bytes from another storage layer.
    pub fn from_bytes(name: impl Into<String>, bytes: Vec<u8>) -> Result<Self, MpqError> {
        Self::load(name.into(), MpqArchiveSource::Memory(bytes))
    }

    /// Human-readable archive name, usually the file name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Parsed MPQ header.
    pub fn header(&self) -> MpqHeader {
        self.header
    }

    /// Returns `true` when the archive hash table contains the given logical
    /// file path.
    pub fn contains_file(&self, path: &str) -> bool {
        self.file_entry(path).is_some()
    }

    /// Reads and decompresses a file from the archive.
    ///
    /// The return value is `Ok(None)` when the path is not present in the MPQ
    /// hash table. Corrupt table data, unsupported compression, and impossible
    /// sector layouts are reported as errors.
    pub fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>, MpqError> {
        let Some(hash_entry) = self.file_entry(path) else {
            return Ok(None);
        };
        let block_index = usize::try_from(hash_entry.block_table_index).map_err(|_| {
            MpqError::BlockIndexOutOfRange {
                index: hash_entry.block_table_index,
                len: self.block_table.len(),
            }
        })?;
        let block_entry =
            self.block_table
                .get(block_index)
                .copied()
                .ok_or(MpqError::BlockIndexOutOfRange {
                    index: hash_entry.block_table_index,
                    len: self.block_table.len(),
                })?;

        if !block_entry.has_flag(MpqFileFlags::EXISTS) {
            return Ok(None);
        }
        if block_entry.archive_size == 0 {
            return Ok(Some(Vec::new()));
        }

        self.extract_block(path, block_entry).map(Some)
    }

    fn load(name: String, source: MpqArchiveSource) -> Result<Self, MpqError> {
        let header_raw = source.read_range(0, HEADER_LEN)?;
        let header = MpqHeader::parse(&header_raw)?;
        if header.format_version != MpqFormatVersion::Version1 {
            return Err(MpqError::UnsupportedFormatVersion(header.format_version));
        }

        let hash_table_size = checked_table_size(header.hash_table_entries, "hash")?;
        let mut hash_table_raw =
            source.read_range(header.hash_table_offset as u64, hash_table_size)?;
        decrypt_mpq_block(
            &mut hash_table_raw,
            mpq_hash("(hash table)", MpqHashType::Table),
        )?;

        let block_table_size = checked_table_size(header.block_table_entries, "block")?;
        let mut block_table_raw =
            source.read_range(header.block_table_offset as u64, block_table_size)?;
        decrypt_mpq_block(
            &mut block_table_raw,
            mpq_hash("(block table)", MpqHashType::Table),
        )?;

        let mut hash_table = HashMap::new();
        for chunk in hash_table_raw.chunks_exact(TABLE_ENTRY_LEN) {
            let entry = MpqHashEntry::parse(chunk)?;
            if entry.block_table_index != u32::MAX && entry.block_table_index != u32::MAX - 1 {
                hash_table.insert((entry.hash_b, entry.hash_a), entry);
            }
        }

        let mut block_table = Vec::with_capacity(header.block_table_entries as usize);
        for chunk in block_table_raw.chunks_exact(TABLE_ENTRY_LEN) {
            block_table.push(MpqBlockEntry::parse(chunk)?);
        }

        Ok(Self {
            name,
            source,
            header,
            hash_table,
            block_table,
        })
    }

    fn file_entry(&self, path: &str) -> Option<MpqHashEntry> {
        let hash_a = mpq_hash(path, MpqHashType::HashA);
        let hash_b = mpq_hash(path, MpqHashType::HashB);
        self.hash_table.get(&(hash_b, hash_a)).copied()
    }

    fn extract_block(&self, path: &str, block_entry: MpqBlockEntry) -> Result<Vec<u8>, MpqError> {
        if block_entry.has_flag(MpqFileFlags::CRC) {
            return Err(MpqError::UnsupportedFlag {
                path: path.to_owned(),
                flag: "crc",
            });
        }

        let mut file_data = self
            .source
            .read_range(block_entry.offset as u64, block_entry.archive_size as usize)?;
        let is_encrypted = block_entry.has_flag(MpqFileFlags::ENCRYPTED);
        let key = self.file_decryption_key(path, block_entry);

        if block_entry.has_flag(MpqFileFlags::SINGLE_UNIT) {
            if is_encrypted {
                decrypt_mpq_block(&mut file_data, key)?;
            }
            let decoded = decode_mpq_sector(&file_data, block_entry.flags, block_entry.size)?;
            ensure_decoded_size(path, &decoded, block_entry.size)?;
            return Ok(decoded);
        }

        self.extract_sectored_file(path, block_entry, &mut file_data, is_encrypted, key)
    }

    fn extract_sectored_file(
        &self,
        path: &str,
        block_entry: MpqBlockEntry,
        file_data: &mut [u8],
        is_encrypted: bool,
        key: u32,
    ) -> Result<Vec<u8>, MpqError> {
        let sector_size = self.header.sector_size() as usize;
        let file_size = block_entry.size as usize;
        let sectors = file_size.div_ceil(sector_size);
        let sector_table_len = (sectors + 1) * 4;
        if file_data.len() < sector_table_len {
            return Err(MpqError::InvalidSectorTable {
                path: path.to_owned(),
                reason: "sector table is shorter than expected",
            });
        }

        if is_encrypted {
            decrypt_mpq_block_range(file_data, key.wrapping_sub(1), 0, sector_table_len)?;
        }

        let mut offsets = Vec::with_capacity(sectors + 1);
        for index in 0..=sectors {
            offsets.push(read_u32_le(file_data, index * 4)? as usize);
        }

        let mut output = vec![0; file_size];
        let mut output_offset = 0;
        for sector_index in 0..sectors {
            let current_offset = offsets[sector_index];
            let next_offset = offsets[sector_index + 1];
            if next_offset < current_offset
                || next_offset > file_data.len()
                || current_offset < sector_table_len
            {
                return Err(MpqError::InvalidSectorTable {
                    path: path.to_owned(),
                    reason: "sector offsets point outside the archived file data",
                });
            }

            let expected_len = sector_size.min(file_size - output_offset);
            let mut sector = file_data[current_offset..next_offset].to_vec();
            if is_encrypted {
                decrypt_mpq_block(&mut sector, key.wrapping_add(sector_index as u32))?;
            }

            let decoded = if sector.len() == expected_len {
                sector
            } else {
                decode_mpq_sector(&sector, block_entry.flags, expected_len as u32)?
            };

            if decoded.len() != expected_len {
                return Err(MpqError::SizeMismatch {
                    path: path.to_owned(),
                    expected: expected_len,
                    actual: decoded.len(),
                });
            }

            output[output_offset..output_offset + expected_len].copy_from_slice(&decoded);
            output_offset += expected_len;
        }

        Ok(output)
    }

    fn file_decryption_key(&self, path: &str, block_entry: MpqBlockEntry) -> u32 {
        let mut key = mpq_decryption_key(path);
        if block_entry.has_flag(MpqFileFlags::ENCRYPTION_FIX) {
            key = key.wrapping_add(block_entry.offset) ^ block_entry.size;
        }
        key
    }
}

impl MpqArchiveSource {
    fn read_range(&self, offset: u64, byte_count: usize) -> Result<Vec<u8>, MpqError> {
        match self {
            Self::Path(path) => {
                let mut file = File::open(path).map_err(|error| MpqError::Io {
                    path: path.display().to_string(),
                    error: error.to_string(),
                })?;
                file.seek(SeekFrom::Start(offset))
                    .map_err(|error| MpqError::Io {
                        path: path.display().to_string(),
                        error: error.to_string(),
                    })?;
                let mut output = vec![0; byte_count];
                file.read_exact(&mut output).map_err(|error| MpqError::Io {
                    path: path.display().to_string(),
                    error: error.to_string(),
                })?;
                Ok(output)
            }
            Self::Memory(bytes) => {
                let offset =
                    usize::try_from(offset).map_err(|_| MpqError::InvalidArchiveRange {
                        offset,
                        size: byte_count,
                    })?;
                let end = offset
                    .checked_add(byte_count)
                    .filter(|end| *end <= bytes.len())
                    .ok_or(MpqError::InvalidArchiveRange {
                        offset: offset as u64,
                        size: byte_count,
                    })?;
                Ok(bytes[offset..end].to_vec())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpqError {
    TooSmall {
        len: usize,
        required: usize,
    },
    BadMagic {
        found: [u8; 4],
    },
    InvalidDecryptRange {
        offset: usize,
        size: usize,
    },
    Io {
        path: String,
        error: String,
    },
    UnsupportedFormatVersion(MpqFormatVersion),
    TableTooLarge {
        table: &'static str,
        entries: u32,
    },
    InvalidArchiveRange {
        offset: u64,
        size: usize,
    },
    BlockIndexOutOfRange {
        index: u32,
        len: usize,
    },
    UnsupportedFlag {
        path: String,
        flag: &'static str,
    },
    InvalidSectorTable {
        path: String,
        reason: &'static str,
    },
    UnsupportedCompression {
        compression: MpqCompressionType,
        mask: u8,
    },
    StackedCompression {
        mask: u8,
    },
    DecompressionFailed {
        compression: MpqCompressionType,
        error: String,
    },
    SizeMismatch {
        path: String,
        expected: usize,
        actual: usize,
    },
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
            Self::Io { path, error } => write!(formatter, "failed to read MPQ {path}: {error}"),
            Self::UnsupportedFormatVersion(version) => {
                write!(formatter, "unsupported MPQ format version {:?}", version)
            }
            Self::TableTooLarge { table, entries } => {
                write!(
                    formatter,
                    "MPQ {table} table has too many entries: {entries}"
                )
            }
            Self::InvalidArchiveRange { offset, size } => write!(
                formatter,
                "invalid MPQ archive read range offset={} size={}",
                offset, size
            ),
            Self::BlockIndexOutOfRange { index, len } => write!(
                formatter,
                "MPQ hash entry references block table index {} but table length is {}",
                index, len
            ),
            Self::UnsupportedFlag { path, flag } => {
                write!(formatter, "MPQ file {path} uses unsupported {flag} flag")
            }
            Self::InvalidSectorTable { path, reason } => {
                write!(formatter, "invalid MPQ sector table for {path}: {reason}")
            }
            Self::UnsupportedCompression { compression, mask } => write!(
                formatter,
                "unsupported MPQ compression {compression} from mask {mask:#04x}"
            ),
            Self::StackedCompression { mask } => {
                write!(
                    formatter,
                    "stacked MPQ compression mask {mask:#04x} is unsupported"
                )
            }
            Self::DecompressionFailed { compression, error } => {
                write!(
                    formatter,
                    "failed to decompress MPQ {compression} sector: {error}"
                )
            }
            Self::SizeMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "decoded MPQ file {path} has {} bytes, expected {}",
                actual, expected
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
        || !size.is_multiple_of(4)
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

fn checked_table_size(entries: u32, table: &'static str) -> Result<usize, MpqError> {
    usize::try_from(entries)
        .ok()
        .and_then(|entries| entries.checked_mul(TABLE_ENTRY_LEN))
        .ok_or(MpqError::TableTooLarge { table, entries })
}

fn decode_mpq_sector(data: &[u8], flags: u32, expected_size: u32) -> Result<Vec<u8>, MpqError> {
    if flags & MpqFileFlags::IMPLODE == MpqFileFlags::IMPLODE {
        return pklib::explode_bytes(data).map_err(|error| MpqError::DecompressionFailed {
            compression: MpqCompressionType::PkWare,
            error: error.to_string(),
        });
    }

    if flags & MpqFileFlags::COMPRESSED != MpqFileFlags::COMPRESSED {
        return Ok(data.to_vec());
    }

    let Some((&mask, payload)) = data.split_first() else {
        return Ok(Vec::with_capacity(expected_size as usize));
    };
    if mask.count_ones() > 1 {
        return Err(MpqError::StackedCompression { mask });
    }

    match MpqCompressionType::from_raw(mask) {
        MpqCompressionType::PkWare => {
            pklib::explode_bytes(payload).map_err(|error| MpqError::DecompressionFailed {
                compression: MpqCompressionType::PkWare,
                error: error.to_string(),
            })
        }
        MpqCompressionType::Zlib => {
            let mut decoder = flate2::read::ZlibDecoder::new(payload);
            let mut output = Vec::with_capacity(expected_size as usize);
            decoder
                .read_to_end(&mut output)
                .map_err(|error| MpqError::DecompressionFailed {
                    compression: MpqCompressionType::Zlib,
                    error: error.to_string(),
                })?;
            Ok(output)
        }
        MpqCompressionType::BZip2 => {
            let mut decoder = bzip2::read::BzDecoder::new(payload);
            let mut output = Vec::with_capacity(expected_size as usize);
            decoder
                .read_to_end(&mut output)
                .map_err(|error| MpqError::DecompressionFailed {
                    compression: MpqCompressionType::BZip2,
                    error: error.to_string(),
                })?;
            Ok(output)
        }
        compression => Err(MpqError::UnsupportedCompression { compression, mask }),
    }
}

fn ensure_decoded_size(path: &str, decoded: &[u8], expected: u32) -> Result<(), MpqError> {
    let expected = expected as usize;
    if decoded.len() == expected {
        Ok(())
    } else {
        Err(MpqError::SizeMismatch {
            path: path.to_owned(),
            expected,
            actual: decoded.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MpqArchive, MpqBlockEntry, MpqFileFlags, MpqFormatVersion, MpqHashEntry, MpqHashType,
        MpqHeader, decrypt_mpq_block, encryption_table, mpq_decryption_key, mpq_hash,
    };
    use sha2::{Digest, Sha256};

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

    #[test]
    fn extracts_imploded_file_from_mpq_fixture() {
        let archive =
            MpqArchive::open("tests/fixtures/mpq/test-implode.mpq").expect("fixture archive");

        assert_eq!(archive.header().archive_size, 59_335);
        assert!(archive.contains_file("strings/pd2/patchstring.tbl"));
        let data = archive
            .read_file("strings/pd2/patchstring.tbl")
            .expect("extract")
            .expect("file exists");

        assert_eq!(data.len(), 61_650);
        assert_eq!(
            Sha256::digest(&data)
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>(),
            "3867b6f6bc92c4c2895d9cec970e6e499c2d3a9e647cc43cae4449e695b1a2c8"
        );
    }

    #[test]
    fn extracts_pkware_compressed_file_from_mpq_fixture() {
        let archive =
            MpqArchive::open("tests/fixtures/mpq/test-pkware.mpq").expect("fixture archive");

        assert_eq!(archive.header().archive_size, 59_364);
        assert!(archive.contains_file("strings/pd2/patchstring.tbl"));
        let data = archive
            .read_file("strings/pd2/patchstring.tbl")
            .expect("extract")
            .expect("file exists");

        assert_eq!(data.len(), 61_650);
        assert_eq!(
            Sha256::digest(&data)
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>(),
            "3867b6f6bc92c4c2895d9cec970e6e499c2d3a9e647cc43cae4449e695b1a2c8"
        );
    }
}
