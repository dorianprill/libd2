use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use socket2::{Domain, Protocol, Socket, Type};
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawSocket;

use std::io::Read;
use std::net::IpAddr;
use std::process;

use crate::core::game_state::GameState;
use crate::core::network::d2gs::{D2GSPacket, D2GSReader};
use crate::core::network::tcp_stream::{TcpReassemblyEvent, TcpStreamReassembler};
use crate::core::protocol::server_message::ServerMessageParseError;
use crate::core::protocol::ServerMessage;
use crate::core::update::Update;

const LEGACY_D2GS_PORT: u16 = 4000;
const D2R_BNET_PORT: u16 = 1119;
const MAX_BUFFERED_D2GS_BYTES: usize = 1024;

mod tcp_flags {
    pub const FIN: u16 = 0x01;
    pub const SYN: u16 = 0x02;
    pub const RST: u16 = 0x04;
    // pub const PSH: u16 = 0x08;
    // pub const ACK: u16 = 0x10;
    // pub const URG: u16 = 0x20;
}

/// Transport classification for captured Diablo II traffic.
///
/// Classic/LoD D2GS traffic on port 4000 uses the legacy plaintext game-server
/// framing handled by [`D2GSReader`]. D2R/modern Battle.net traffic observed on
/// port 1119 is a protected Battle.net transport, not raw D2GS framing, so it
/// must not be passed to the legacy reader unless a future caller supplies
/// already-decoded plaintext fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturedTransport {
    LegacyD2gsServerToClient,
    LegacyD2gsClientToServer,
    D2rEncryptedOrUnknown,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TcpStreamKey {
    source: IpAddr,
    destination: IpAddr,
    source_port: u16,
    destination_port: u16,
}

impl TcpStreamKey {
    fn new(source: IpAddr, destination: IpAddr, source_port: u16, destination_port: u16) -> Self {
        Self {
            source,
            destination,
            source_port,
            destination_port,
        }
    }
}

fn classify_transport(source_port: u16, destination_port: u16) -> CapturedTransport {
    if source_port == LEGACY_D2GS_PORT {
        CapturedTransport::LegacyD2gsServerToClient
    } else if destination_port == LEGACY_D2GS_PORT {
        CapturedTransport::LegacyD2gsClientToServer
    } else if source_port == D2R_BNET_PORT || destination_port == D2R_BNET_PORT {
        CapturedTransport::D2rEncryptedOrUnknown
    } else {
        CapturedTransport::Ignored
    }
}

/// Non-packet diagnostic emitted by [`Connection`] during live capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionTransportWarning {
    /// A TCP retransmission was fully behind the already-consumed byte stream.
    DuplicateTcpSegment {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
    },
    /// A TCP retransmission overlapped the consumed sequence and only its new
    /// trailing bytes were forwarded to the D2GS reader.
    OverlappingTcpSegment {
        sequence: u32,
        skipped: usize,
        emitted: usize,
        expected_sequence: u32,
    },
    /// A TCP segment arrived after a gap and is buffered until the missing
    /// earlier bytes arrive.
    OutOfOrderTcpSegment {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
        buffered_segments: usize,
        buffered_bytes: usize,
    },
    /// A previously buffered segment became contiguous and was forwarded.
    BufferedTcpSegmentReleased { sequence: u32, len: usize },
    /// Too much data accumulated behind a missing TCP segment, so the D2GS
    /// reader was reset and capture resumed at a later sequence.
    TcpGapReset {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
        buffered_segments: usize,
        buffered_bytes: usize,
    },
    /// A payload reached the D2GS reader but did not yet complete a D2GS packet.
    BufferedD2gsPayload {
        payload_len: usize,
        buffered_len: usize,
    },
    /// The D2GS byte-stream splitter accumulated too much unread data, so the
    /// buffered framing state was discarded and the latest TCP payload was
    /// retried from a clean boundary.
    D2gsFramingReset {
        payload_len: usize,
        discarded_len: usize,
    },
}

impl From<TcpReassemblyEvent> for ConnectionTransportWarning {
    fn from(event: TcpReassemblyEvent) -> Self {
        match event {
            TcpReassemblyEvent::DuplicateSegment {
                sequence,
                len,
                expected_sequence,
            } => Self::DuplicateTcpSegment {
                sequence,
                len,
                expected_sequence,
            },
            TcpReassemblyEvent::OverlapTrimmed {
                sequence,
                skipped,
                emitted,
                expected_sequence,
            } => Self::OverlappingTcpSegment {
                sequence,
                skipped,
                emitted,
                expected_sequence,
            },
            TcpReassemblyEvent::OutOfOrderBuffered {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
            } => Self::OutOfOrderTcpSegment {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
            },
            TcpReassemblyEvent::BufferedSegmentReleased { sequence, len } => {
                Self::BufferedTcpSegmentReleased { sequence, len }
            }
            TcpReassemblyEvent::GapReset {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
            } => Self::TcpGapReset {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
            },
        }
    }
}

/// Event emitted by [`Connection`] when legacy D2GS capture progresses.
///
/// Overlay and visualization tools should treat this as the non-blocking bridge
/// out of packet capture: run `Connection::listen_with_events` or
/// `Client::start_with_events` on a worker thread, then forward these events or
/// small `GameState` snapshots to the UI thread. Parse errors are emitted rather
/// than silently swallowed so callers can track missing packet coverage while
/// still keeping the capture loop alive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    ServerMessage {
        packet: D2GSPacket,
        message: Box<ServerMessage>,
        applied: bool,
    },
    ParseError {
        packet: D2GSPacket,
        error: ServerMessageParseError,
    },
    TransportWarning {
        warning: ConnectionTransportWarning,
    },
}

impl ConnectionEvent {
    pub fn packet(&self) -> Option<&D2GSPacket> {
        match self {
            Self::ServerMessage { packet, .. } | Self::ParseError { packet, .. } => Some(packet),
            Self::TransportWarning { .. } => None,
        }
    }

    pub fn packet_id(&self) -> Option<u8> {
        self.packet().map(D2GSPacket::packet_id)
    }
}

// There are three different connections involved:
// BNCS: the battle.net chat server
// Realm: (also called MCP)
//   Ports:
//     6112 UDP for D2Vanilla
//     6120 UDP for D2R (PC)
// D2GS:  The diablo2 game server protocol
//   Ports:
//     source 4000 TCP for legacy Classic/LoD plaintext D2GS server messages
//     destination 4000 TCP for client action messages, not parsed as D2GS server messages yet
//     1119 TCP for D2R/modern Battle.net transport; not legacy D2GS framing

pub struct Connection {
    interface: netdev::Interface,
    initialized: bool,
    //protocol_state: ProtocolState,
    d2gs_reader: D2GSReader,
    d2gs_tcp_stream: TcpStreamReassembler,
    d2gs_tcp_stream_key: Option<TcpStreamKey>,
    // BinaryHeap as a PriorityQueue
    //packet_queue:   BinaryHeap<RawPacket<'a>>
}

impl Connection {
    pub fn new() -> Self {
        let interfaces = netdev::get_interfaces();
        Connection {
            interface: interfaces.into_iter().next().unwrap(),
            initialized: false,
            d2gs_reader: D2GSReader::new(),
            d2gs_tcp_stream: TcpStreamReassembler::new(),
            d2gs_tcp_stream_key: None,
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn init(&mut self) {
        // Find the first network interface connected to the internet
        let interfaces = netdev::get_interfaces();
        let some_if = interfaces.into_iter().find(|ifx| {
            ifx.is_up() && !ifx.is_loopback() && !ifx.ipv4.is_empty()
        });

        if let Some(itf) = some_if {
            self.interface = itf;
            println!("Identified network interface {}", self.interface.name);
        } else {
            println!("No active network adapter found, aborting...");
            process::exit(1);
        }

        self.initialized = true;
    }
    pub fn listen(&mut self, game_state: &mut GameState) {
        self.listen_with_events(game_state, |_, _| {});
    }

    /// Starts the blocking packet-capture loop and emits decoded D2GS events.
    ///
    /// This method is blocking because the underlying raw socket waits for the
    /// next packet. UI applications should call it from a worker thread.
    pub fn listen_with_events<F>(&mut self, game_state: &mut GameState, mut on_event: F)
    where
        F: FnMut(ConnectionEvent, &GameState),
    {
        if !self.initialized {
            println!("Connection: must init() before listen()");
            return;
        }

        let mut socket = self.create_raw_socket();
        let mut buf = [0u8; 2048];

        loop {
            match socket.read(&mut buf) {

                Ok(n) => {
                    self.handle_raw_packet(&buf[..n], game_state, &mut on_event);
                }
                Err(e) => {
                    eprintln!("unable to receive packet: {}", e);
                    continue;
                }
            }
        }
    }

    fn create_raw_socket(&self) -> Socket {
        #[cfg(target_os = "windows")]
        {
            // Protocol 0 is IPPROTO_IP
            let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::from(0))).expect("Failed to create Windows raw socket");
            
            // On Windows we must bind to a local interface IP to use SIO_RCVALL
            if let Some(ip) = self.interface.ipv4.first() {
                socket.bind(&std::net::SocketAddrV4::new(ip.addr(), 0).into()).expect("Failed to bind raw socket");
            }

            let rcval_on: u32 = 1; // SIO_RCVALL_ON
            let mut bytes_returned: u32 = 0;
            unsafe {
                let r = windows_sys::Win32::Networking::WinSock::WSAIoctl(
                    socket.as_raw_socket() as _,
                    0x98000001, // SIO_RCVALL
                    &rcval_on as *const _ as _,
                    std::mem::size_of::<u32>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                    None,
                );
                if r != 0 {
                    eprintln!("Warning: WSAIoctl SIO_RCVALL failed ({}). Ensure running as Admin.", std::io::Error::last_os_error());
                }
            }
            socket
        }

        #[cfg(target_os = "linux")]
        {
            // ETH_P_ALL = 0x0003
            Socket::new(Domain::PACKET, Type::RAW, Some(Protocol::from(0x0003))).expect("Failed to create Linux raw socket")
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            panic!("Unsupported platform for driverless raw sockets. Implementation for BPF/macOS needed.");
        }
    }

    fn handle_raw_packet<F>(
        &mut self,
        data: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
    {
        // Try parsing as Ethernet (Linux/macOS)
        if let Ok(packet) = SlicedPacket::from_ethernet(data) {
            self.handle_sliced_packet(packet, game_state, on_event);
            return;
        }

        // Fallback to IP (Windows SIO_RCVALL or TUN/TAP)
        if let Ok(packet) = SlicedPacket::from_ip(data) {
            self.handle_sliced_packet(packet, game_state, on_event);
        }
    }

    fn handle_sliced_packet<F>(
        &mut self,
        packet: SlicedPacket,
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
    {
        let (source_ip, dest_ip) = match &packet.net {
            Some(NetSlice::Ipv4(ipv4)) => (
                IpAddr::V4(ipv4.header().source_addr()),
                IpAddr::V4(ipv4.header().destination_addr()),
            ),
            Some(NetSlice::Ipv6(ipv6)) => (
                IpAddr::V6(ipv6.header().source_addr()),
                IpAddr::V6(ipv6.header().destination_addr()),
            ),
            _ => return,
        };

        if let Some(TransportSlice::Tcp(tcp)) = &packet.transport {
            if classify_transport(tcp.source_port(), tcp.destination_port())
                == CapturedTransport::LegacyD2gsServerToClient
            {
                let stream_key = TcpStreamKey::new(
                    source_ip,
                    dest_ip,
                    tcp.source_port(),
                    tcp.destination_port(),
                );
                let mut flags = 0u16;
                if tcp.fin() {
                    flags |= tcp_flags::FIN;
                }
                if tcp.syn() {
                    flags |= tcp_flags::SYN;
                }
                if tcp.rst() {
                    flags |= tcp_flags::RST;
                }
                self.read_legacy_d2gs_tcp_segment(
                    stream_key,
                    flags,
                    tcp.sequence_number(),
                    tcp.payload(),
                    game_state,
                    on_event,
                );
            }
        } else if let Some(TransportSlice::Udp(udp)) = &packet.transport {
            if classify_transport(udp.source_port(), udp.destination_port())
                == CapturedTransport::LegacyD2gsServerToClient
            {
                self.read_d2gs_payload(udp.payload(), game_state, on_event);
            }
        }
    }

    /// Processes a legacy D2GS payload without using live packet capture.
    ///
    /// This is intended for fixture replay, tests, and callers that receive raw
    /// D2GS payload bytes from some other source. For live sniffing, use
    /// [`Connection::listen_with_events`].
    pub fn process_d2gs_payload(
        &mut self,
        payload: &[u8],
        game_state: &mut GameState,
    ) -> Vec<ConnectionEvent> {
        let mut events = Vec::new();
        self.process_d2gs_payload_with_events(payload, game_state, |event, _| {
            events.push(event);
        });
        events
    }

    /// Processes a legacy D2GS payload and calls `on_event` for each decoded
    /// packet.
    pub fn process_d2gs_payload_with_events<F>(
        &mut self,
        payload: &[u8],
        game_state: &mut GameState,
        mut on_event: F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
    {
        self.read_d2gs_payload(payload, game_state, &mut on_event);
    }

    fn read_legacy_d2gs_tcp_segment<F>(
        &mut self,
        stream_key: TcpStreamKey,
        tcp_flags: u16,
        sequence: u32,
        payload: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
    {
        if self.d2gs_tcp_stream_key.as_ref() != Some(&stream_key)
            || tcp_flags & (tcp_flags::SYN | tcp_flags::RST) != 0
        {
            self.d2gs_tcp_stream.reset();
            self.d2gs_reader.reset();
            self.d2gs_tcp_stream_key = Some(stream_key);
        }

        if payload.is_empty() {
            if tcp_flags & tcp_flags::FIN != 0 {
                self.d2gs_tcp_stream.reset();
                self.d2gs_reader.reset();
            }
            return;
        }

        let result = self.d2gs_tcp_stream.push(sequence, payload);
        if result.reset_required() {
            self.d2gs_reader.reset();
        }

        for event in result.events() {
            on_event(
                ConnectionEvent::TransportWarning {
                    warning: (*event).into(),
                },
                game_state,
            );
        }

        for payload in result.payloads() {
            self.read_d2gs_payload(payload, game_state, on_event);
        }

        if tcp_flags & tcp_flags::FIN != 0 {
            self.d2gs_tcp_stream.reset();
            self.d2gs_reader.reset();
        }
    }

    fn read_d2gs_payload<F>(&mut self, payload: &[u8], game_state: &mut GameState, on_event: &mut F)
    where
        F: FnMut(ConnectionEvent, &GameState),
    {
        let buffered_before = self.d2gs_reader.buffered_len();
        let mut emitted_packet = false;

        self.d2gs_reader.read(payload);
        emitted_packet |= emit_buffered_packets(&mut self.d2gs_reader, game_state, on_event);

        let buffered_len = self.d2gs_reader.buffered_len();
        if !payload.is_empty() && !emitted_packet && buffered_len > buffered_before {
            if buffered_len >= MAX_BUFFERED_D2GS_BYTES {
                self.d2gs_reader.reset();
                on_event(
                    ConnectionEvent::TransportWarning {
                        warning: ConnectionTransportWarning::D2gsFramingReset {
                            payload_len: payload.len(),
                            discarded_len: buffered_len,
                        },
                    },
                    game_state,
                );

                self.d2gs_reader.read(payload);
                emit_buffered_packets(&mut self.d2gs_reader, game_state, on_event);
                return;
            }
            on_event(
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::BufferedD2gsPayload {
                        payload_len: payload.len(),
                        buffered_len,
                    },
                },
                game_state,
            );
        }
    }
}

fn emit_buffered_packets<F>(
    reader: &mut D2GSReader,
    game_state: &mut GameState,
    on_event: &mut F,
) -> bool
where
    F: FnMut(ConnectionEvent, &GameState),
{
    let mut emitted_packet = false;
    while let Some(packet) = reader.next() {
        emitted_packet = true;
        match ServerMessage::try_from(&packet) {
            Ok(message) => {
                let applied = game_state.update(message.clone());
                on_event(
                    ConnectionEvent::ServerMessage {
                        packet,
                        message: Box::new(message),
                        applied,
                    },
                    game_state,
                );
            }
            Err(error) => {
                on_event(ConnectionEvent::ParseError { packet, error }, game_state);
            }
        }
    }
    emitted_packet
}

#[cfg(test)]
mod tests {
    use super::{
        classify_transport, CapturedTransport, Connection, ConnectionEvent,
        ConnectionTransportWarning, D2R_BNET_PORT, LEGACY_D2GS_PORT, MAX_BUFFERED_D2GS_BYTES,
    };
    use crate::core::game_state::GameState;
    use crate::core::protocol::server_message::ServerMessageParseError;
    use crate::ServerMessage;

    #[test]
    fn classifies_legacy_d2gs_server_messages_by_source_port() {
        assert_eq!(
            classify_transport(LEGACY_D2GS_PORT, 51_000),
            CapturedTransport::LegacyD2gsServerToClient
        );
    }

    #[test]
    fn classifies_legacy_d2gs_client_messages_without_server_parsing() {
        assert_eq!(
            classify_transport(51_000, LEGACY_D2GS_PORT),
            CapturedTransport::LegacyD2gsClientToServer
        );
    }

    #[test]
    fn classifies_d2r_battle_net_transport_without_d2gs_parsing() {
        assert_eq!(
            classify_transport(D2R_BNET_PORT, 51_000),
            CapturedTransport::D2rEncryptedOrUnknown
        );
        assert_eq!(
            classify_transport(51_000, D2R_BNET_PORT),
            CapturedTransport::D2rEncryptedOrUnknown
        );
    }

    #[test]
    fn ignores_unrelated_ports() {
        assert_eq!(classify_transport(443, 51_000), CapturedTransport::Ignored);
    }

    #[test]
    fn process_d2gs_payload_emits_event_and_updates_state() {
        let mut connection = Connection::new();
        let mut state = GameState::default();
        let mut packet = vec![0x59, 0x04, 0x03, 0x02, 0x01, 0x03];
        packet.extend_from_slice(b"Rusty\0\0\0\0\0\0\0\0\0\0\0");
        packet.extend_from_slice(&1234u16.to_le_bytes());
        packet.extend_from_slice(&5678u16.to_le_bytes());

        let events = connection.process_d2gs_payload(&packet, &mut state);

        assert_eq!(events.len(), 1);
        assert!(state.player(0x0102_0304).is_some());
        assert_eq!(state.local_player_id(), None);
        match &events[0] {
            ConnectionEvent::ServerMessage {
                packet,
                message,
                applied,
            } => {
                assert_eq!(packet.packet_id(), 0x59);
                if let ServerMessage::AssignPlayer { unit_id, x, y, .. } = **message {
                    assert_eq!(unit_id, 0x0102_0304);
                    assert_eq!((x, y), (1234, 5678));
                } else {
                    panic!("unexpected message variant");
                }
                assert!(*applied);
            }
            other => panic!("unexpected connection event: {:?}", other),
        }
    }

    #[test]
    fn process_d2gs_payload_splits_concatenated_server_packets() {
        let mut connection = Connection::new();
        let mut state = GameState::default();
        let payload = [
            0x07, 0x70, 0x04, 0x78, 0x03, 0x01, 0x07, 0x78, 0x04, 0x78, 0x03, 0x01, 0x07, 0x80,
            0x04, 0x78, 0x03, 0x01,
        ];

        let events = connection.process_d2gs_payload(&payload, &mut state);

        assert_eq!(events.len(), 3);
        assert_eq!(state.map().revealed_tiles.len(), 3);
        assert!(events.iter().all(|event| matches!(
            event,
            ConnectionEvent::ServerMessage {
                message,
                applied: true,
                ..
            } if matches!(**message, ServerMessage::MapReveal { .. })
        )));
    }

    #[test]
    fn process_d2gs_payload_reports_parse_errors() {
        let mut connection = Connection::new();
        let mut state = GameState::default();

        let events = connection.process_d2gs_payload(&[0xB1, 0x00], &mut state);

        assert_eq!(events.len(), 1);
        match &events[0] {
            ConnectionEvent::ParseError { packet, error } => {
                assert_eq!(packet.packet_id(), 0xB1);
                assert_eq!(error, &ServerMessageParseError::UnsupportedPacketId(0xB1));
            }
            other => panic!("unexpected connection event: {:?}", other),
        }
    }

    #[test]
    fn process_d2gs_payload_recovers_after_framing_desync() {
        let mut connection = Connection::new();
        let mut state = GameState::default();

        let poison_events = connection.process_d2gs_payload(&[0xAE, 0xFF, 0x7F], &mut state);
        assert!(poison_events.iter().any(|event| matches!(
            event,
            ConnectionEvent::TransportWarning {
                warning: ConnectionTransportWarning::BufferedD2gsPayload {
                    payload_len: 3,
                    buffered_len: 3,
                }
            }
        )));

        let payload = [0x07, 0x70, 0x04, 0x78, 0x03, 0x01].repeat(200);
        let recovery_events = connection.process_d2gs_payload(&payload, &mut state);

        assert!(recovery_events.iter().any(|event| matches!(
            event,
            ConnectionEvent::TransportWarning {
                warning: ConnectionTransportWarning::D2gsFramingReset {
                    payload_len,
                    discarded_len,
                }
            } if *payload_len == payload.len() && *discarded_len >= MAX_BUFFERED_D2GS_BYTES
        )));
        assert!(recovery_events.iter().any(|event| matches!(
            event,
            ConnectionEvent::ServerMessage {
                message,
                applied: true,
                ..
            } if matches!(**message, ServerMessage::MapReveal { .. })
        )));
        assert!(!state.map().revealed_tiles.is_empty());
    }

    #[test]
    fn live_tcp_reassembly_buffers_out_of_order_segments_before_d2gs_decode() {
        let mut connection = Connection::new();
        let mut state = GameState::default();
        let key = super::TcpStreamKey::new(
            "10.0.0.1".parse().unwrap(),
            "10.0.0.2".parse().unwrap(),
            LEGACY_D2GS_PORT,
            51_000,
        );
        let mut events = Vec::new();

        connection.read_legacy_d2gs_tcp_segment(
            key.clone(),
            0,
            100,
            &[0x07, 0x70, 0x04],
            &mut state,
            &mut |event, _| events.push(event),
        );
        connection.read_legacy_d2gs_tcp_segment(
            key.clone(),
            0,
            106,
            &[0x07, 0x78, 0x04, 0x78, 0x03, 0x01],
            &mut state,
            &mut |event, _| events.push(event),
        );

        assert!(events.iter().any(|event| {
            matches!(
                event,
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::BufferedD2gsPayload {
                        payload_len: 3,
                        buffered_len: 3,
                    }
                }
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::OutOfOrderTcpSegment {
                        sequence: 106,
                        expected_sequence: 103,
                        ..
                    }
                }
            )
        }));
        assert!(state.map().revealed_tiles.is_empty());

        connection.read_legacy_d2gs_tcp_segment(
            key,
            0,
            103,
            &[0x78, 0x03, 0x01],
            &mut state,
            &mut |event, _| events.push(event),
        );

        let parsed_packets = events
            .iter()
            .filter_map(ConnectionEvent::packet_id)
            .collect::<Vec<_>>();
        assert_eq!(parsed_packets, vec![0x07, 0x07]);
        assert_eq!(state.map().revealed_tiles.len(), 2);
    }

    #[test]
    fn live_tcp_reassembly_ignores_duplicate_segment_without_replaying_state() {
        let mut connection = Connection::new();
        let mut state = GameState::default();
        let key = super::TcpStreamKey::new(
            "10.0.0.1".parse().unwrap(),
            "10.0.0.2".parse().unwrap(),
            LEGACY_D2GS_PORT,
            51_000,
        );
        let payload = [0x07, 0x70, 0x04, 0x78, 0x03, 0x01];
        let mut events = Vec::new();

        connection.read_legacy_d2gs_tcp_segment(
            key.clone(),
            0,
            100,
            &payload,
            &mut state,
            &mut |event, _| events.push(event),
        );
        connection.read_legacy_d2gs_tcp_segment(
            key,
            0,
            100,
            &payload,
            &mut state,
            &mut |event, _| events.push(event),
        );

        let parsed_packets = events
            .iter()
            .filter_map(ConnectionEvent::packet_id)
            .collect::<Vec<_>>();
        assert_eq!(parsed_packets, vec![0x07]);
        assert_eq!(state.map().revealed_tiles.len(), 1);
        assert!(events.iter().any(|event| {
            matches!(
                event,
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::DuplicateTcpSegment {
                        sequence: 100,
                        expected_sequence: 106,
                        ..
                    }
                }
            )
        }));
    }
}
