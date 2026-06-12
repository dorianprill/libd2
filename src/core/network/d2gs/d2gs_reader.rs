use crate::core::network::d2gs::d2gs_packet::D2GSPacket;
use crate::core::network::huffman;

use std::collections::VecDeque;

const PACKET_SIZES: [i32; 177] = [
    // 1, 8, 1, 12, 1, 1, 1, 6, 6, 11, 6, 6, 9, 13, 12, 16,
    // 16, 8, 26, 14, 18, 11, 0, 0, 15, 2, 2, 3, 5, 3, 4, 6,
    // 10, 12, 12, 13, 90, 90, 0, 40, 103,97, 15, 0, 8, 0, 0, 0,
    // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34, 8,
    // 13, 0, 6, 0, 0, 13, 0, 11, 11, 0, 0, 0, 16, 17, 7, 1,
    // 15, 14, 42, 10, 3, 0, 0, 14, 7, 26, 40, 0, 5, 6, 38, 5,
    // 7, 2, 7, 21, 0, 7, 7, 16, 21, 12, 12, 16, 16, 10, 1, 1,
    // 1, 1, 1, 32, 10, 13, 6, 2, 21, 6, 13, 8, 6, 18, 5, 10,
    // 4, 20, 29, 0, 0, 0, 0, 0, 0, 2, 6, 6, 11, 7, 10, 33,
    // 13, 26, 6, 8, 0, 13, 9, 1, 7, 16, 17, 7, 0, 0, 7, 8,
    // 10, 7, 8, 24, 3, 8, 0, 7, 0, 7, 0, 7, 0, 0, 0, 0,
    // 1 ];
    1, 8, 1, 12, 1, 1, 1, 6, 6, 11, 6, 6, 9, 13, 12, 16, /* 1 */ 16, 8, 26, 14, 18, 11, -1, -1,
    15, 2, 2, 3, 5, 3, 4, 6, /* 2 */ 10, 12, 12, 13, 90, 90, -1, 40, 103, 97, 15, 0, 8, 0, 0,
    0, /* 3 */ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34, 8, /* 4 */ 13, 0, 6, 0, 0,
    13, 0, 11, 11, 0, 0, 0, 16, 17, 7, 1, /* 5 */ 15, 14, 42, 10, 3, 0, 0, 14, 7, 26, 40, -1,
    5, 6, 38, 5, /* 6 */ 7, 2, 7, 21, 0, 7, 7, 16, 21, 12, 12, 16, 16, 10, 1, 1,
    /* 7 */ 1, 1, 1, 32, 10, 13, 6, 2, 21, 6, 13, 8, 6, 18, 5, 10, /* 8 */ 4, 20, 29, 0,
    0, 0, 0, 0, 0, 2, 6, 6, 11, 7, 10, 33, /* 9 */ 13, 26, 6, 8, -1, 13, 9, 1, 7, 16, 17, 7,
    -1, -1, 7, 8, /* A */ 10, 7, 8, 24, 3, 8, -1, 7, -1, 7, -1, 7, -1, 0, -1, 0,
    /* B */ 1,
];

pub struct D2GSReader {
    // used as a single ended queue here
    packets: VecDeque<D2GSPacket>,
    packet_stream: Vec<u8>,
    compressed_stream: Vec<u8>,
    compression_enabled: bool,
}

impl Default for D2GSReader {
    fn default() -> Self {
        Self::new()
    }
}

impl D2GSReader {
    pub fn new() -> Self {
        D2GSReader {
            packets: VecDeque::with_capacity(128),
            packet_stream: Vec::with_capacity(1600),
            compressed_stream: Vec::with_capacity(1600),
            compression_enabled: false,
        }
    }

    pub fn next(&mut self) -> Option<D2GSPacket> {
        self.packets.pop_front()
    }

    /// Clears queued packets and any partial D2GS packet bytes.
    ///
    /// Live capture uses this when the lower TCP stream detects that bytes were
    /// missed and has to resume at a later sequence. Keeping the partial D2GS
    /// buffer in that situation would make the next valid packet look like a
    /// continuation of stale data.
    pub fn reset(&mut self) {
        self.packets.clear();
        self.packet_stream.clear();
        self.compressed_stream.clear();
        self.compression_enabled = false;
    }

    /// Returns the number of D2GS bytes waiting for more data.
    ///
    /// This includes decompressed/plain packet bytes and raw compressed chunk
    /// bytes. A non-zero value is normal when TCP splits a D2GS packet or a
    /// Huffman chunk across multiple segments.
    pub fn buffered_len(&self) -> usize {
        self.packet_stream.len() + self.compressed_stream.len()
    }

    pub fn packet_stream_len(&self) -> usize {
        self.packet_stream.len()
    }

    pub fn compressed_stream_len(&self) -> usize {
        self.compressed_stream.len()
    }

    pub fn packet_stream_prefix(&self, limit: usize) -> Vec<u8> {
        self.packet_stream.iter().take(limit).copied().collect()
    }

    pub fn compressed_stream_prefix(&self, limit: usize) -> Vec<u8> {
        self.compressed_stream.iter().take(limit).copied().collect()
    }

    /// Normalizes a captured legacy D2GS payload into individual game packets.
    ///
    /// Live packet capture sees TCP payloads, not semantic D2GS messages. One
    /// payload may contain a single packet, several back-to-back packets, or the
    /// first part of a packet completed by a later payload. Classic/LoD commonly
    /// leaves game traffic uncompressed after the `0xAF` compression-mode packet,
    /// so an uncompressed payload starts directly with a D2GS packet id such as
    /// `0x07` (map reveal) or `0x9C` (world item action). This reader splits that
    /// byte stream using the 1.14d server packet-size table before callers parse
    /// protocol messages.
    ///
    /// Compressed payloads use Diablo II's Huffman framing. A compressed chunk
    /// length can be encoded in one byte or two bytes; the encoded length includes
    /// the framing header. A chunk can decompress to several game packets, which
    /// are fed through the same stream splitter.
    pub fn read(&mut self, raw: &[u8]) {
        if raw.is_empty() {
            return;
        }

        if self.compression_enabled || !self.compressed_stream.is_empty() || raw[0] >= 0xF0 {
            self.queue_compressed_stream(raw);
            return;
        }

        self.queue_packet_stream(raw, PacketStreamSource::PlainTcp);

        // Packets remain queued for the caller to parse and apply to game state.
    }

    pub fn handle_all(&mut self) {
        while let Some(p) = self.packets.pop_front() {
            println!("{}", p);
        }
    }

    fn queue_compressed_stream(&mut self, stream: &[u8]) {
        self.compressed_stream.extend_from_slice(stream);

        let mut start = 0;
        while start < self.compressed_stream.len() {
            let Some((header_size, data_size)) =
                compressed_chunk_lengths(&self.compressed_stream[start..])
            else {
                break;
            };
            let data_start = start + header_size;
            let data_end = data_start + data_size;
            if data_end > self.compressed_stream.len() {
                break;
            }

            let mut decompressed_chunk = Vec::with_capacity(data_size.saturating_mul(2));
            huffman::decode(
                &self.compressed_stream[data_start..data_end],
                &mut decompressed_chunk,
            );
            self.queue_packet_stream(&decompressed_chunk, PacketStreamSource::Decompressed);
            start = data_end;
        }

        if start > 0 {
            self.compressed_stream.drain(0..start);
        }
    }

    fn queue_packet_stream(&mut self, stream: &[u8], source: PacketStreamSource) {
        self.packet_stream.extend_from_slice(stream);

        while !self.packet_stream.is_empty() {
            match packet_size_status(&self.packet_stream) {
                PacketSizeStatus::Complete(size) if self.packet_stream.len() >= size => {
                    let data: Vec<u8> = self.packet_stream.drain(0..size).collect();
                    self.observe_packet_mode(&data, source);
                    self.packets.push_back(D2GSPacket { data });
                    if self.compression_enabled
                        && source == PacketStreamSource::PlainTcp
                        && !self.packet_stream.is_empty()
                    {
                        let compressed_tail = std::mem::take(&mut self.packet_stream);
                        self.queue_compressed_stream(&compressed_tail);
                        break;
                    }
                }
                PacketSizeStatus::Complete(_) | PacketSizeStatus::NeedMore => break,
                PacketSizeStatus::Unknown => {
                    let data = self.packet_stream.drain(..).collect();
                    self.packets.push_back(D2GSPacket { data });
                }
            }
        }
    }

    fn observe_packet_mode(&mut self, data: &[u8], _source: PacketStreamSource) {
        if data.len() == 2 && data[0] == 0xAF {
            self.compression_enabled = data[1] != 0;
            if !self.compression_enabled {
                self.compressed_stream.clear();
            }
        }
    }
} // impl D2GSReader

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PacketStreamSource {
    PlainTcp,
    Decompressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PacketSizeStatus {
    Complete(usize),
    NeedMore,
    Unknown,
}

fn compressed_chunk_lengths(raw: &[u8]) -> Option<(usize, usize)> {
    let first = *raw.first()?;
    if first < 0xF0 {
        let packet_size = first as usize;
        return packet_size.checked_sub(1).map(|data_size| (1, data_size));
    }

    let second = *raw.get(1)?;
    let packet_size = (((first & 0x0F) as usize) << 8) | second as usize;
    packet_size.checked_sub(2).map(|data_size| (2, data_size))
}

// translated from OmegaBot
pub fn get_chat_packet_size(input: &[u8], result: &mut i32) -> bool {
    if input.len() < 12 {
        return false;
    }

    const INITIAL_OFFSET: i32 = 10;
    let Some(name_position) = input.iter().position(|&x| (x as i32) == INITIAL_OFFSET) else {
        return false;
    };
    let mut name_offset = name_position as i32;
    name_offset -= INITIAL_OFFSET;

    let Some(message_position) = input
        .iter()
        .position(|&x| (x as i32) == (INITIAL_OFFSET + name_offset + 1))
    else {
        return false;
    };
    let mut message_offset = message_position as i32;

    message_offset = message_offset - INITIAL_OFFSET - name_offset - 1;
    *result = INITIAL_OFFSET + name_offset + 1 + message_offset + 1;

    true
}

// This was taken from Redvex according to qqbot source and corrected for
// Diablo II 1.14d packet streams where one TCP payload may contain many packets.
pub fn get_packet_size(input: &[u8], result: &mut i32) -> bool {
    match packet_size_status(input) {
        PacketSizeStatus::Complete(size) => {
            *result = size as i32;
            true
        }
        PacketSizeStatus::NeedMore | PacketSizeStatus::Unknown => {
            *result = 0;
            false
        }
    }
}

fn packet_size_status(input: &[u8]) -> PacketSizeStatus {
    if input.is_empty() {
        return PacketSizeStatus::NeedMore;
    }

    let identifier = input[0];
    let size = input.len() as i32;

    match identifier {
        0x26 => {
            let mut result = 0;
            if get_chat_packet_size(input, &mut result) {
                return positive_size(result);
            }
            PacketSizeStatus::NeedMore
        }
        0x5B => {
            if size >= 3 {
                return positive_size(((input[2] as i32) << 8) | input[1] as i32);
            }
            PacketSizeStatus::NeedMore
        }
        0x94 => {
            if size >= 2 {
                return positive_size(input[1] as i32 * 3 + 6);
            }
            PacketSizeStatus::NeedMore
        }
        0xA8 | 0xAA => {
            if size >= 7 {
                return positive_size(input[6] as i32);
            }
            PacketSizeStatus::NeedMore
        }
        0xAC => {
            if size >= 13 {
                return positive_size(input[12] as i32);
            }
            PacketSizeStatus::NeedMore
        }
        0xAE => {
            if size >= 3 {
                return positive_size(3 + (((input[1] as i32) << 8) | input[2] as i32));
            }
            PacketSizeStatus::NeedMore
        }
        0x3E => {
            if size >= 2 {
                let declared_size = input[1] as usize;
                if (2..=34).contains(&declared_size)
                    && input.len() >= 34
                    && input
                        .get(declared_size..34)
                        .is_some_and(|padding| padding.iter().all(|&byte| byte == 0))
                {
                    return PacketSizeStatus::Complete(34);
                }
                return positive_size(declared_size as i32);
            }
            PacketSizeStatus::NeedMore
        }
        0x9C | 0x9D => {
            if size >= 3 {
                return positive_size(input[2] as i32);
            }
            PacketSizeStatus::NeedMore
        }
        0xAF => PacketSizeStatus::Complete(2),
        0xFF => PacketSizeStatus::Complete(1),
        _ => {
            if (identifier as usize) < PACKET_SIZES.len() {
                return positive_size(PACKET_SIZES[identifier as usize]);
            }
            PacketSizeStatus::Unknown
        }
    }
}

fn positive_size(size: i32) -> PacketSizeStatus {
    if size > 0 {
        PacketSizeStatus::Complete(size as usize)
    } else {
        PacketSizeStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{D2GSReader, get_packet_size};

    fn drain_packet_lengths(reader: &mut D2GSReader) -> Vec<usize> {
        let mut lengths = Vec::new();
        while let Some(packet) = reader.next() {
            lengths.push(packet.data.len());
        }
        lengths
    }

    fn drain_packets(reader: &mut D2GSReader) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();
        while let Some(packet) = reader.next() {
            packets.push(packet.data);
        }
        packets
    }

    #[test]
    fn packet_size_table_matches_1_14d_game_flags_length() {
        let mut len = 0;

        assert!(get_packet_size(&[0x01], &mut len));
        assert_eq!(len, 8);
    }

    #[test]
    fn lod_1_14d_assumed_plain_payload_can_contain_multiple_map_reveals() {
        let mut reader = D2GSReader::new();
        // Live-shaped bytes captured before fixture metadata existed; assume LoD 1.14d.
        let payload = [
            0x07, 0x70, 0x04, 0x78, 0x03, 0x01, 0x07, 0x78, 0x04, 0x78, 0x03, 0x01, 0x07, 0x80,
            0x04, 0x78, 0x03, 0x01,
        ];

        reader.read(&payload);

        assert_eq!(drain_packet_lengths(&mut reader), vec![6, 6, 6]);
    }

    #[test]
    fn lod_1_14d_assumed_plain_payload_can_contain_multiple_variable_item_packets() {
        let mut reader = D2GSReader::new();
        // Live-shaped bytes captured before fixture metadata existed; assume LoD 1.14d.
        let payload = [
            0x9C, 0x0E, 0x14, 0x10, 0xEB, 0xAA, 0xCC, 0x81, 0x10, 0x00, 0xA2, 0x00, 0x65, 0x08,
            0x02, 0x80, 0x06, 0x17, 0x03, 0x02, 0x9C, 0x0E, 0x14, 0x10, 0x75, 0x35, 0xE6, 0xD0,
            0x10, 0x00, 0xA2, 0x00, 0x65, 0x08, 0x04, 0x80, 0x06, 0x17, 0x03, 0x02,
        ];

        reader.read(&payload);

        assert_eq!(drain_packet_lengths(&mut reader), vec![20, 20]);
    }

    #[test]
    fn padded_1_14d_item_stat_packet_is_not_split_at_declared_size() {
        let mut reader = D2GSReader::new();
        let mut payload = vec![0x3E, 0x05, 0x10, 0x20, 0x30];
        payload.resize(34, 0);
        payload.extend_from_slice(&[0xAF, 0x00]);

        reader.read(&payload);

        assert_eq!(drain_packet_lengths(&mut reader), vec![34, 2]);
    }

    #[test]
    fn split_payload_buffers_incomplete_trailing_packet() {
        let mut reader = D2GSReader::new();

        reader.read(&[0x07, 0x70, 0x04]);
        assert!(reader.next().is_none());

        reader.read(&[0x78, 0x03, 0x01]);
        let packet = reader.next().expect("packet should complete");
        assert_eq!(packet.data, vec![0x07, 0x70, 0x04, 0x78, 0x03, 0x01]);
        assert!(reader.next().is_none());
    }

    #[test]
    fn compressed_mode_allows_one_byte_huffman_chunk_header() {
        let mut reader = D2GSReader::new();
        // Blacha huffman fixture: a one-byte length header followed by encoded
        // bytes that decompress to GameFlags (8 bytes) and GameLoading (1 byte).
        reader.read(&[0xAF, 0x01]);
        assert_eq!(
            reader.next().expect("compression mode packet").data,
            [0xAF, 0x01]
        );

        reader.read(&[0x06, 0x7A, 0x04, 0x64, 0xBB, 0xBC]);

        assert_eq!(
            drain_packets(&mut reader),
            vec![
                vec![0x01, 0x00, 0x04, 0x08, 0x30, 0x00, 0x01, 0x01],
                vec![0x00]
            ]
        );
    }

    #[test]
    fn compressed_chunk_split_across_tcp_payloads_is_buffered() {
        let mut reader = D2GSReader::new();
        reader.read(&[0xAF, 0x01]);
        assert!(reader.next().is_some());

        reader.read(&[0x06, 0x7A, 0x04]);
        assert!(reader.next().is_none());
        assert_eq!(reader.buffered_len(), 3);

        reader.read(&[0x64, 0xBB, 0xBC]);

        assert_eq!(
            drain_packets(&mut reader),
            vec![
                vec![0x01, 0x00, 0x04, 0x08, 0x30, 0x00, 0x01, 0x01],
                vec![0x00]
            ]
        );
        assert_eq!(reader.buffered_len(), 0);
    }
}
