//! Minimal TCP byte-stream reconstruction for passive D2GS capture.
//!
//! `pnet` exposes individual TCP segments. Diablo II game-server packets sit
//! above TCP, so the D2GS reader must see bytes in TCP sequence order instead
//! of raw capture order. This helper provides the small subset needed by the
//! legacy port-4000 server-to-client stream: retransmission trimming,
//! out-of-order buffering, and a bounded reset path when capture misses a gap.

use std::collections::BTreeMap;

const DEFAULT_MAX_PENDING_SEGMENTS: usize = 32;
const DEFAULT_MAX_PENDING_BYTES: usize = 64 * 1024;

/// Diagnostic emitted while reconstructing a TCP payload stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpReassemblyEvent {
    /// A retransmitted segment was fully behind the already-consumed sequence.
    DuplicateSegment {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
    },
    /// A retransmitted segment overlapped the consumed sequence and had new
    /// trailing bytes that were forwarded.
    OverlapTrimmed {
        sequence: u32,
        skipped: usize,
        emitted: usize,
        expected_sequence: u32,
    },
    /// A segment arrived after the next expected sequence and is waiting for
    /// the missing earlier segment.
    OutOfOrderBuffered {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
        buffered_segments: usize,
        buffered_bytes: usize,
    },
    /// A buffered out-of-order segment became contiguous and was forwarded.
    BufferedSegmentReleased { sequence: u32, len: usize },
    /// Too many bytes or segments were waiting behind a gap; the caller should
    /// discard higher-level framing state and resume at the new segment.
    GapReset {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
        buffered_segments: usize,
        buffered_bytes: usize,
    },
}

/// Result of ingesting one captured TCP segment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TcpReassemblyResult {
    payloads: Vec<Vec<u8>>,
    events: Vec<TcpReassemblyEvent>,
    reset_required: bool,
}

impl TcpReassemblyResult {
    pub fn payloads(&self) -> &[Vec<u8>] {
        &self.payloads
    }

    pub fn events(&self) -> &[TcpReassemblyEvent] {
        &self.events
    }

    pub fn reset_required(&self) -> bool {
        self.reset_required
    }

    fn push_payload(&mut self, payload: Vec<u8>) {
        if !payload.is_empty() {
            self.payloads.push(payload);
        }
    }

    fn push_event(&mut self, event: TcpReassemblyEvent) {
        self.events.push(event);
    }
}

/// Stateful TCP reassembler for one unidirectional payload stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpStreamReassembler {
    next_sequence: Option<u32>,
    pending: BTreeMap<u32, Vec<u8>>,
    pending_bytes: usize,
    max_pending_segments: usize,
    max_pending_bytes: usize,
}

impl Default for TcpStreamReassembler {
    fn default() -> Self {
        Self {
            next_sequence: None,
            pending: BTreeMap::new(),
            pending_bytes: 0,
            max_pending_segments: DEFAULT_MAX_PENDING_SEGMENTS,
            max_pending_bytes: DEFAULT_MAX_PENDING_BYTES,
        }
    }
}

impl TcpStreamReassembler {
    /// Creates a reassembler with default buffering limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a reassembler with small limits for focused tests.
    #[cfg(test)]
    fn with_limits(max_pending_segments: usize, max_pending_bytes: usize) -> Self {
        Self {
            max_pending_segments,
            max_pending_bytes,
            ..Self::default()
        }
    }

    /// Clears all remembered TCP sequence state.
    pub fn reset(&mut self) {
        self.next_sequence = None;
        self.pending.clear();
        self.pending_bytes = 0;
    }

    /// Ingests one captured TCP segment and returns ordered payload chunks.
    ///
    /// This function deliberately reconstructs only a single direction of an
    /// already-identified connection. It assumes callers have filtered out ACKs
    /// without payload and call `reset` when the 4-tuple changes or a new TCP
    /// handshake starts.
    pub fn push(&mut self, sequence: u32, payload: &[u8]) -> TcpReassemblyResult {
        let mut result = TcpReassemblyResult::default();
        if payload.is_empty() {
            return result;
        }

        let Some(expected_sequence) = self.next_sequence else {
            self.next_sequence = Some(sequence);
            self.accept_contiguous(sequence, payload.to_vec(), &mut result);
            return result;
        };

        if sequence == expected_sequence {
            self.accept_contiguous(sequence, payload.to_vec(), &mut result);
            self.release_pending(&mut result);
            return result;
        }

        if sequence < expected_sequence {
            let skipped = (expected_sequence - sequence) as usize;
            if skipped >= payload.len() {
                result.push_event(TcpReassemblyEvent::DuplicateSegment {
                    sequence,
                    len: payload.len(),
                    expected_sequence,
                });
                return result;
            }

            let trimmed = payload[skipped..].to_vec();
            result.push_event(TcpReassemblyEvent::OverlapTrimmed {
                sequence,
                skipped,
                emitted: trimmed.len(),
                expected_sequence,
            });
            self.accept_contiguous(expected_sequence, trimmed, &mut result);
            self.release_pending(&mut result);
            return result;
        }

        self.buffer_gap(sequence, payload, expected_sequence, &mut result);
        result
    }

    fn accept_contiguous(
        &mut self,
        sequence: u32,
        payload: Vec<u8>,
        result: &mut TcpReassemblyResult,
    ) {
        self.next_sequence = Some(sequence.wrapping_add(payload.len() as u32));
        result.push_payload(payload);
    }

    fn buffer_gap(
        &mut self,
        sequence: u32,
        payload: &[u8],
        expected_sequence: u32,
        result: &mut TcpReassemblyResult,
    ) {
        if !self.pending.contains_key(&sequence) {
            self.pending_bytes = self.pending_bytes.saturating_add(payload.len());
            self.pending.insert(sequence, payload.to_vec());
        }

        if self.pending.len() > self.max_pending_segments
            || self.pending_bytes > self.max_pending_bytes
        {
            let buffered_segments = self.pending.len();
            let buffered_bytes = self.pending_bytes;
            self.reset();
            self.next_sequence = Some(sequence);
            result.reset_required = true;
            result.push_event(TcpReassemblyEvent::GapReset {
                sequence,
                len: payload.len(),
                expected_sequence,
                buffered_segments,
                buffered_bytes,
            });
            self.accept_contiguous(sequence, payload.to_vec(), result);
            return;
        }

        result.push_event(TcpReassemblyEvent::OutOfOrderBuffered {
            sequence,
            len: payload.len(),
            expected_sequence,
            buffered_segments: self.pending.len(),
            buffered_bytes: self.pending_bytes,
        });
    }

    fn release_pending(&mut self, result: &mut TcpReassemblyResult) {
        while let Some(expected_sequence) = self.next_sequence {
            let Some(payload) = self.pending.remove(&expected_sequence) else {
                break;
            };
            self.pending_bytes = self.pending_bytes.saturating_sub(payload.len());
            result.push_event(TcpReassemblyEvent::BufferedSegmentReleased {
                sequence: expected_sequence,
                len: payload.len(),
            });
            self.accept_contiguous(expected_sequence, payload, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TcpReassemblyEvent, TcpStreamReassembler};

    #[test]
    fn reassembler_emits_in_order_segments() {
        let mut stream = TcpStreamReassembler::new();

        let first = stream.push(100, b"abc");
        let second = stream.push(103, b"def");

        assert_eq!(first.payloads(), &[b"abc".to_vec()]);
        assert!(first.events().is_empty());
        assert_eq!(second.payloads(), &[b"def".to_vec()]);
        assert!(second.events().is_empty());
    }

    #[test]
    fn reassembler_ignores_duplicate_retransmission() {
        let mut stream = TcpStreamReassembler::new();

        stream.push(100, b"abcdef");
        let duplicate = stream.push(100, b"abcdef");

        assert!(duplicate.payloads().is_empty());
        assert_eq!(
            duplicate.events(),
            &[TcpReassemblyEvent::DuplicateSegment {
                sequence: 100,
                len: 6,
                expected_sequence: 106,
            }]
        );
    }

    #[test]
    fn reassembler_trims_overlapping_retransmission() {
        let mut stream = TcpStreamReassembler::new();

        stream.push(100, b"abcdef");
        let overlap = stream.push(103, b"defghi");

        assert_eq!(overlap.payloads(), &[b"ghi".to_vec()]);
        assert_eq!(
            overlap.events(),
            &[TcpReassemblyEvent::OverlapTrimmed {
                sequence: 103,
                skipped: 3,
                emitted: 3,
                expected_sequence: 106,
            }]
        );
    }

    #[test]
    fn reassembler_buffers_and_releases_out_of_order_segment() {
        let mut stream = TcpStreamReassembler::new();

        stream.push(100, b"abc");
        let gap = stream.push(106, b"ghi");
        let fill = stream.push(103, b"def");

        assert!(gap.payloads().is_empty());
        assert_eq!(
            gap.events(),
            &[TcpReassemblyEvent::OutOfOrderBuffered {
                sequence: 106,
                len: 3,
                expected_sequence: 103,
                buffered_segments: 1,
                buffered_bytes: 3,
            }]
        );
        assert_eq!(fill.payloads(), &[b"def".to_vec(), b"ghi".to_vec()]);
        assert_eq!(
            fill.events(),
            &[TcpReassemblyEvent::BufferedSegmentReleased {
                sequence: 106,
                len: 3,
            }]
        );
    }

    #[test]
    fn reassembler_resets_after_bounded_gap_overflow() {
        let mut stream = TcpStreamReassembler::with_limits(1, 8);

        stream.push(100, b"abc");
        stream.push(106, b"ghi");
        let reset = stream.push(109, b"jkl");

        assert!(reset.reset_required());
        assert_eq!(reset.payloads(), &[b"jkl".to_vec()]);
        assert_eq!(
            reset.events(),
            &[TcpReassemblyEvent::GapReset {
                sequence: 109,
                len: 3,
                expected_sequence: 103,
                buffered_segments: 2,
                buffered_bytes: 6,
            }]
        );
    }
}
