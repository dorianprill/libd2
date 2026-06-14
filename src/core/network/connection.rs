extern crate pnet;

use pnet::ipnetwork::IpNetwork;

//use self::pnet::packet::ethernet::Ethernet;
use self::pnet::datalink::{self, NetworkInterface};
use self::pnet::packet::Packet;
use self::pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use self::pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use self::pnet::packet::ipv4::Ipv4Packet;
use self::pnet::packet::ipv6::Ipv6Packet;
use self::pnet::packet::tcp::{TcpFlags, TcpPacket};
use self::pnet::packet::udp::UdpPacket;
use self::pnet::util::MacAddr;

//use std::env;
//use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::process;
use std::str;

//use std::collections::BinaryHeap;
//use connection::raw_packet::RawPacket;
use crate::core::game_state::GameState;
use crate::core::network::d2gs::{D2GSPacket, D2GSReader};
use crate::core::network::tcp_stream::{TcpReassemblyEvent, TcpStreamReassembler};
use crate::core::protocol::ServerMessage;
use crate::core::protocol::server_message::ServerMessageParseError;
use crate::core::update::Update;

const LEGACY_D2GS_PORT: u16 = 4000;
const D2R_BNET_PORT: u16 = 1119;
const MAX_BUFFERED_D2GS_BYTES: usize = 16 * 1024;
const D2GS_DIAGNOSTIC_PREFIX_BYTES: usize = 32;
const CAPTURE_READ_BUFFER_SIZE: usize = 4 * 1024 * 1024;

const ROUTE_PROBE_TARGETS: &[(Ipv4Addr, u16)] = &[
    (Ipv4Addr::new(1, 1, 1, 1), 443),
    (Ipv4Addr::new(8, 8, 8, 8), 443),
    (Ipv4Addr::new(9, 9, 9, 9), 443),
];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D2gsBufferSnapshot {
    pub packet_buffer_len: usize,
    pub compressed_buffer_len: usize,
    pub compression_enabled: bool,
    pub payload_prefix: Vec<u8>,
    pub packet_buffer_prefix: Vec<u8>,
    pub compressed_buffer_prefix: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureInterfaceSelectionReason {
    RouteProbe { local_ip: IpAddr },
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D2gsSessionResetReason {
    NewTcpStream,
    TcpReset,
    TcpFin,
}

#[derive(Debug, Clone)]
pub struct CaptureInterfaceSelection {
    pub interface: NetworkInterface,
    pub reason: CaptureInterfaceSelectionReason,
}

/// Non-packet diagnostic emitted by [`Connection`] during live capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionTransportWarning {
    /// The observed D2GS TCP session ended or a new one began, so accumulated
    /// game state was cleared.
    D2gsSessionReset { reason: D2gsSessionResetReason },
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
    /// A TCP gap stayed open past the passive-capture recovery window, so the
    /// D2GS reader was reset and capture resumed at a later sequence.
    TcpGapTimeoutReset {
        sequence: u32,
        len: usize,
        expected_sequence: u32,
        buffered_segments: usize,
        buffered_bytes: usize,
        elapsed_millis: u128,
    },
    /// A payload reached the D2GS reader but did not yet complete a D2GS packet.
    BufferedD2gsPayload {
        payload_len: usize,
        buffered_len: usize,
        snapshot: D2gsBufferSnapshot,
    },
    /// The D2GS byte-stream splitter accumulated too much unread data, so the
    /// buffered framing state was discarded and the latest TCP payload was
    /// retried from a clean boundary.
    D2gsFramingReset {
        payload_len: usize,
        discarded_len: usize,
        snapshot: D2gsBufferSnapshot,
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
            TcpReassemblyEvent::GapTimeoutReset {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
                elapsed_millis,
            } => Self::TcpGapTimeoutReset {
                sequence,
                len,
                expected_sequence,
                buffered_segments,
                buffered_bytes,
                elapsed_millis,
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
        message: ServerMessage,
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

pub fn select_capture_interface(
    interfaces: &[NetworkInterface],
) -> Option<CaptureInterfaceSelection> {
    select_capture_interface_for_route_ip(interfaces, route_probe_local_ip())
}

pub fn route_probe_local_ip() -> Option<IpAddr> {
    for &(target, port) in ROUTE_PROBE_TARGETS {
        let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
            continue;
        };
        if socket.connect((target, port)).is_err() {
            continue;
        }
        let Ok(local_addr) = socket.local_addr() else {
            continue;
        };
        let local_ip = local_addr.ip();
        if !local_ip.is_unspecified() {
            return Some(local_ip);
        }
    }
    None
}

fn select_capture_interface_for_route_ip(
    interfaces: &[NetworkInterface],
    route_ip: Option<IpAddr>,
) -> Option<CaptureInterfaceSelection> {
    if let Some(local_ip) = route_ip
        && let Some(interface) = interfaces
            .iter()
            .find(|interface| interface_has_ip(interface, local_ip) && !interface.is_loopback())
    {
        return Some(CaptureInterfaceSelection {
            interface: interface.clone(),
            reason: CaptureInterfaceSelectionReason::RouteProbe { local_ip },
        });
    }

    interfaces
        .iter()
        .find(|interface| fallback_capture_candidate(interface))
        .cloned()
        .map(|interface| CaptureInterfaceSelection {
            interface,
            reason: CaptureInterfaceSelectionReason::Fallback,
        })
}

fn interface_has_ip(interface: &NetworkInterface, ip: IpAddr) -> bool {
    interface
        .ips
        .iter()
        .any(|network| network.ip() == ip && usable_interface_ip(network))
}

fn fallback_capture_candidate(interface: &NetworkInterface) -> bool {
    if interface.is_loopback() || !interface.ips.iter().any(usable_interface_ip) {
        return false;
    }

    cfg!(target_os = "windows") || interface.is_up()
}

fn usable_interface_ip(network: &IpNetwork) -> bool {
    let ip = network.ip();
    !ip.is_unspecified() && !ip.is_loopback()
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
    interface: NetworkInterface,
    initialized: bool,
    //protocol_state: ProtocolState,
    d2gs_reader: D2GSReader,
    d2gs_tcp_stream: TcpStreamReassembler,
    d2gs_tcp_stream_key: Option<TcpStreamKey>,
    // BinaryHeap as a PriorityQueue
    //packet_queue:   BinaryHeap<RawPacket<'a>>
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            interface: datalink::interfaces().pop().unwrap(),
            initialized: false,
            d2gs_reader: D2GSReader::new(),
            d2gs_tcp_stream: TcpStreamReassembler::new(),
            d2gs_tcp_stream_key: None,
        }
    }

    pub fn init(&mut self) {
        let interfaces = datalink::interfaces();
        if let Some(selection) = select_capture_interface(&interfaces) {
            self.interface = selection.interface;
            match selection.reason {
                CaptureInterfaceSelectionReason::RouteProbe { local_ip } => println!(
                    "Identified network interface {} from routed local address {}",
                    self.interface, local_ip
                ),
                CaptureInterfaceSelectionReason::Fallback => {
                    println!("Identified fallback network interface {}", self.interface)
                }
            }
        } else {
            println!("No active network adapter found, aborting...");
            process::exit(1);
        }

        self.initialized = true;
    }
    // thread function start_hook() ?
    //pub fn get_packet() -> pnet::packet::TcpPacket {}
    //pub fn build_packet() -> pnet::packet::TcpPacket {}
    // add_packet allows external modules to add packages for injecting
    //pub fn add_packet() -> bool {}
    pub fn listen(&mut self, game_state: &mut GameState) {
        self.listen_with_events(game_state, |_, _| {});
    }

    /// Starts the blocking packet-capture loop and emits decoded D2GS events.
    ///
    /// This method is still blocking because libpnet's receiver waits for the
    /// next frame. UI applications should call it from a worker thread. The
    /// callback receives each parsed/failed packet plus the current game state
    /// after a successfully parsed message has been applied.
    pub fn listen_with_events<F>(&mut self, game_state: &mut GameState, mut on_event: F)
    where
        F: FnMut(ConnectionEvent, &GameState),
    {
        self.listen_with_mut_events(game_state, |event, game_state| {
            on_event(event, game_state);
        });
    }

    /// Starts the blocking packet-capture loop and emits decoded D2GS events.
    ///
    /// This variant gives the callback mutable access to the current game state
    /// after each applied message. UI tools use this for explicit operator
    /// actions such as clearing stale state without tying that behavior to TCP
    /// stream recovery.
    pub fn listen_with_mut_events<F>(&mut self, game_state: &mut GameState, mut on_event: F)
    where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        use self::pnet::datalink::Channel::Ethernet;
        let interface = self.interface.clone();
        if !self.initialized {
            println!("Connection: must init() before listen()");
            return;
        }
        let capture_config = datalink::Config {
            read_buffer_size: CAPTURE_READ_BUFFER_SIZE,
            // We only need packets addressed to/from the local Diablo II client.
            // Promiscuous mode is unnecessary for that use case and can fail on
            // some Linux wireless drivers with ENODEV during PACKET_ADD_MEMBERSHIP.
            promiscuous: false,
            ..Default::default()
        };

        // Create a channel to receive on
        let (_, mut rx_channel) = match datalink::channel(&self.interface, capture_config) {
            Ok(Ethernet(tx, rx_channel)) => (tx, rx_channel),
            Ok(_) => panic!("{}", "unhandled channel type: {}"),
            Err(e) => panic!("unable to create channel: {}", e),
        };

        loop {
            let mut buf: [u8; 1600] = [0u8; 1600];
            let mut fake_ethernet_frame = MutableEthernetPacket::new(&mut buf[..]).unwrap();
            match rx_channel.next() {
                Ok(packet) => {
                    if cfg!(target_os = "macos")
                        && !interface.is_broadcast()
                        && interface.is_point_to_point()
                    {
                        // Maybe is TUN interface
                        let Some(ipv4_packet) = Ipv4Packet::new(packet) else {
                            eprintln!("[{}]: Malformed point-to-point IP packet", interface.name);
                            continue;
                        };
                        let version = ipv4_packet.get_version();

                        fake_ethernet_frame.set_destination(MacAddr(0, 0, 0, 0, 0, 0));
                        fake_ethernet_frame.set_source(MacAddr(0, 0, 0, 0, 0, 0));
                        if version == 4 {
                            fake_ethernet_frame.set_ethertype(EtherTypes::Ipv4);
                            continue;
                        } else if version == 6 {
                            fake_ethernet_frame.set_ethertype(EtherTypes::Ipv6);
                            continue;
                        }
                        fake_ethernet_frame.set_payload(packet);
                        self.handle_ethernet_frame(
                            &interface,
                            &fake_ethernet_frame.to_immutable(),
                            game_state,
                            &mut on_event,
                        );
                    }
                    let Some(ethernet) = EthernetPacket::new(packet) else {
                        eprintln!("[{}]: Malformed Ethernet frame", interface.name);
                        continue;
                    };
                    self.handle_ethernet_frame(&interface, &ethernet, game_state, &mut on_event);
                }
                Err(e) => {
                    eprintln!("unable to receive packet: {}", e);
                    continue;
                }
            }
        }
    }

    fn handle_udp_packet<F>(
        &mut self,
        _source: IpAddr,
        _destination: IpAddr,
        packet: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let udp = UdpPacket::new(packet);

        if let Some(udp) = udp {
            // filter packet by ports used by d2, will continue for both sent & received
            //if !PORTS.contains(&udp.get_destination()) && !PORTS.contains(&udp.get_source()) {
            //    return
            //}
            match classify_transport(udp.get_source(), udp.get_destination()) {
                CapturedTransport::LegacyD2gsServerToClient => {
                    self.read_d2gs_payload(udp.payload(), game_state, on_event)
                }
                CapturedTransport::LegacyD2gsClientToServer => {}
                CapturedTransport::D2rEncryptedOrUnknown | CapturedTransport::Ignored => {}
            }
            // println!(
            //         "UDP {}:{} > {}:{}  len={:03}  {:x?}  {:?}",
            //         source,
            //         udp.get_source(),
            //         destination,
            //         udp.get_destination(),
            //         udp.payload().len(),
            //         udp.payload(),
            //         String::from_utf8_lossy(udp.payload()).into_owned()
            // );
        } else {
            println!("Malformed UDP Packet");
        }
    }

    fn handle_tcp_packet<F>(
        &mut self,
        source: IpAddr,
        destination: IpAddr,
        packet: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let tcp = TcpPacket::new(packet);
        if let Some(tcp) = tcp {
            // filter packet by ports used by d2, will continue for both sent & received
            //if !PORTS.contains(&tcp.get_destination()) {//&& !PORTS.contains(&tcp.get_source()) {
            //    return
            //}
            match classify_transport(tcp.get_source(), tcp.get_destination()) {
                CapturedTransport::LegacyD2gsServerToClient => {
                    let stream_key = TcpStreamKey::new(
                        source,
                        destination,
                        tcp.get_source(),
                        tcp.get_destination(),
                    );
                    self.read_legacy_d2gs_tcp_segment(
                        stream_key,
                        tcp.get_flags(),
                        tcp.get_sequence(),
                        tcp.payload(),
                        game_state,
                        on_event,
                    );
                }
                CapturedTransport::LegacyD2gsClientToServer => {}
                CapturedTransport::D2rEncryptedOrUnknown | CapturedTransport::Ignored => {}
            }
            // println!(
            //     "TCP {}:{} > {}:{}  len={:03}  {:x?}  {:?}",
            //     source,
            //     tcp.get_source(),
            //     destination,
            //     tcp.get_destination(),
            //     tcp.payload().len(),
            //     tcp.payload(),
            //     String::from_utf8_lossy(tcp.payload()).into_owned()
            // );
        } else {
            println!("Malformed TCP Packet");
        }
    }

    fn handle_transport_protocol<F>(
        &mut self,
        source: IpAddr,
        destination: IpAddr,
        protocol: IpNextHeaderProtocol,
        packet: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        match protocol {
            IpNextHeaderProtocols::Udp => {
                self.handle_udp_packet(source, destination, packet, game_state, on_event)
            }
            IpNextHeaderProtocols::Tcp => {
                self.handle_tcp_packet(source, destination, packet, game_state, on_event)
            }
            _ => (),
        }
    }

    fn handle_ipv4_packet<F>(
        &mut self,
        interface_name: &str,
        ethernet: &EthernetPacket,
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let header = Ipv4Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(
                IpAddr::V4(header.get_source()),
                IpAddr::V4(header.get_destination()),
                header.get_next_level_protocol(),
                header.payload(),
                game_state,
                on_event,
            );
        } else {
            println!("[{}]: Malformed IPv4 Packet", interface_name);
        }
    }

    fn handle_ipv6_packet<F>(
        &mut self,
        interface_name: &str,
        ethernet: &EthernetPacket,
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let header = Ipv6Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(
                IpAddr::V6(header.get_source()),
                IpAddr::V6(header.get_destination()),
                header.get_next_header(),
                header.payload(),
                game_state,
                on_event,
            );
        } else {
            println!("[{}]: Malformed IPv6 Packet", interface_name);
        }
    }

    fn handle_ethernet_frame<F>(
        &mut self,
        interface: &NetworkInterface,
        ethernet: &EthernetPacket,
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let interface_name = &interface.name[..];
        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => {
                self.handle_ipv4_packet(interface_name, ethernet, game_state, on_event)
            }
            EtherTypes::Ipv6 => {
                self.handle_ipv6_packet(interface_name, ethernet, game_state, on_event)
            }
            _ => (), // TODO make debug only print
                     // println!(
                     // "[{}]: Unknown packet: {} > {}; ethertype: {:?} length: {}",
                     // interface_name,
                     // ethernet.get_source(),
                     // ethernet.get_destination(),
                     // ethernet.get_ethertype(),
                     // ethernet.packet().len()
                     //),
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
        self.process_d2gs_payload_with_mut_events(payload, game_state, |event, game_state| {
            on_event(event, game_state);
        });
    }

    /// Processes a legacy D2GS payload and calls `on_event` for each decoded
    /// packet, allowing the callback to explicitly mutate the game state.
    pub fn process_d2gs_payload_with_mut_events<F>(
        &mut self,
        payload: &[u8],
        game_state: &mut GameState,
        mut on_event: F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        self.read_d2gs_payload(payload, game_state, &mut on_event);
    }

    fn read_legacy_d2gs_tcp_segment<F>(
        &mut self,
        stream_key: TcpStreamKey,
        tcp_flags: u8,
        sequence: u32,
        payload: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let new_stream = self.d2gs_tcp_stream_key.as_ref() != Some(&stream_key);
        let reset_reason = if new_stream {
            Some(D2gsSessionResetReason::NewTcpStream)
        } else if tcp_flags & TcpFlags::RST != 0 {
            Some(D2gsSessionResetReason::TcpReset)
        } else if tcp_flags & TcpFlags::SYN != 0 {
            Some(D2gsSessionResetReason::NewTcpStream)
        } else {
            None
        };

        if let Some(reason) = reset_reason {
            self.d2gs_tcp_stream.reset();
            self.d2gs_reader.reset();
            self.d2gs_tcp_stream_key = Some(stream_key);
            on_event(
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::D2gsSessionReset { reason },
                },
                game_state,
            );
        }

        if payload.is_empty() {
            if tcp_flags & TcpFlags::FIN != 0 {
                self.d2gs_tcp_stream.reset();
                self.d2gs_reader.reset();
                on_event(
                    ConnectionEvent::TransportWarning {
                        warning: ConnectionTransportWarning::D2gsSessionReset {
                            reason: D2gsSessionResetReason::TcpFin,
                        },
                    },
                    game_state,
                );
            }
            return;
        }

        let result = self.d2gs_tcp_stream.push(sequence, payload);
        if result.reset_required() {
            self.d2gs_reader.reset_framing();
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

        if tcp_flags & TcpFlags::FIN != 0 {
            self.d2gs_tcp_stream.reset();
            self.d2gs_reader.reset();
            on_event(
                ConnectionEvent::TransportWarning {
                    warning: ConnectionTransportWarning::D2gsSessionReset {
                        reason: D2gsSessionResetReason::TcpFin,
                    },
                },
                game_state,
            );
        }
    }

    fn read_d2gs_payload<F>(&mut self, payload: &[u8], game_state: &mut GameState, on_event: &mut F)
    where
        F: FnMut(ConnectionEvent, &mut GameState),
    {
        let buffered_before = self.d2gs_reader.buffered_len();
        let mut emitted_packet = false;

        self.d2gs_reader.read(payload);
        emitted_packet |= emit_buffered_packets(&mut self.d2gs_reader, game_state, on_event);

        let buffered_len = self.d2gs_reader.buffered_len();
        if !payload.is_empty() && !emitted_packet && buffered_len > buffered_before {
            let snapshot = self.d2gs_buffer_snapshot(payload);
            if buffered_len >= MAX_BUFFERED_D2GS_BYTES {
                self.d2gs_reader.reset_framing();
                on_event(
                    ConnectionEvent::TransportWarning {
                        warning: ConnectionTransportWarning::D2gsFramingReset {
                            payload_len: payload.len(),
                            discarded_len: buffered_len,
                            snapshot,
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
                        snapshot,
                    },
                },
                game_state,
            );
        }
    }

    fn d2gs_buffer_snapshot(&self, payload: &[u8]) -> D2gsBufferSnapshot {
        D2gsBufferSnapshot {
            packet_buffer_len: self.d2gs_reader.packet_stream_len(),
            compressed_buffer_len: self.d2gs_reader.compressed_stream_len(),
            compression_enabled: self.d2gs_reader.compression_enabled(),
            payload_prefix: payload
                .iter()
                .take(D2GS_DIAGNOSTIC_PREFIX_BYTES)
                .copied()
                .collect(),
            packet_buffer_prefix: self
                .d2gs_reader
                .packet_stream_prefix(D2GS_DIAGNOSTIC_PREFIX_BYTES),
            compressed_buffer_prefix: self
                .d2gs_reader
                .compressed_stream_prefix(D2GS_DIAGNOSTIC_PREFIX_BYTES),
        }
    }
}

fn emit_buffered_packets<F>(
    reader: &mut D2GSReader,
    game_state: &mut GameState,
    on_event: &mut F,
) -> bool
where
    F: FnMut(ConnectionEvent, &mut GameState),
{
    let mut emitted_packet = false;
    for packet in reader.by_ref() {
        emitted_packet = true;
        match ServerMessage::try_from(&packet) {
            Ok(message) => {
                let applied = game_state.update(message.clone());
                on_event(
                    ConnectionEvent::ServerMessage {
                        packet,
                        message,
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
        CaptureInterfaceSelectionReason, CapturedTransport, Connection, ConnectionEvent,
        ConnectionTransportWarning, D2R_BNET_PORT, D2gsSessionResetReason, LEGACY_D2GS_PORT,
        MAX_BUFFERED_D2GS_BYTES, classify_transport,
    };
    use crate::ServerMessage;
    use crate::core::game_state::GameState;
    use crate::core::protocol::server_message::ServerMessageParseError;
    use crate::core::update::Update;
    use pnet::datalink::NetworkInterface;
    use pnet::ipnetwork::IpNetwork;
    use pnet::packet::tcp::TcpFlags;
    use std::net::{IpAddr, Ipv4Addr};

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
    fn route_probe_selection_matches_routed_local_ip_to_interface() {
        let hyper_v_ip = Ipv4Addr::new(172, 29, 80, 1);
        let wifi_ip = Ipv4Addr::new(192, 168, 1, 106);
        let interfaces = vec![
            test_interface("Hyper-V Virtual Ethernet Adapter", 34, hyper_v_ip),
            test_interface("Intel(R) Wi-Fi 6E AX210 160MHz", 15, wifi_ip),
        ];

        let selection =
            super::select_capture_interface_for_route_ip(&interfaces, Some(IpAddr::V4(wifi_ip)))
                .expect("routed interface should be selected");

        assert_eq!(selection.interface.index, 15);
        assert_eq!(
            selection.reason,
            CaptureInterfaceSelectionReason::RouteProbe {
                local_ip: IpAddr::V4(wifi_ip)
            }
        );
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
                message: ServerMessage::AssignPlayer { unit_id, x, y, .. },
                applied,
            } => {
                assert_eq!(packet.packet_id(), 0x59);
                assert_eq!(*unit_id, 0x0102_0304);
                assert_eq!((*x, *y), (1234, 5678));
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
                message: ServerMessage::MapReveal { .. },
                applied: true,
                ..
            }
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
                    ..
                }
            }
        )));

        let payload = [0x07, 0x70, 0x04, 0x78, 0x03, 0x01].repeat(MAX_BUFFERED_D2GS_BYTES / 6 + 10);
        let recovery_events = connection.process_d2gs_payload(&payload, &mut state);

        assert!(recovery_events.iter().any(|event| matches!(
            event,
            ConnectionEvent::TransportWarning {
                warning: ConnectionTransportWarning::D2gsFramingReset {
                    payload_len,
                    discarded_len,
                    ..
                }
            } if *payload_len == payload.len() && *discarded_len >= MAX_BUFFERED_D2GS_BYTES
        )));
        assert!(recovery_events.iter().any(|event| matches!(
            event,
            ConnectionEvent::ServerMessage {
                message: ServerMessage::MapReveal { .. },
                applied: true,
                ..
            }
        )));
        assert!(!state.map().revealed_tiles.is_empty());
    }

    #[test]
    fn live_tcp_new_stream_preserves_game_state_and_emits_event() {
        let mut connection = Connection::new();
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 7,
            class: 1,
            szname: *b"Stale\0\0\0\0\0\0\0\0\0\0\0",
            x: 10,
            y: 20,
        }));
        let key = super::TcpStreamKey::new(
            "10.0.0.1".parse().unwrap(),
            "10.0.0.2".parse().unwrap(),
            LEGACY_D2GS_PORT,
            51_000,
        );
        let mut events = Vec::new();

        connection.read_legacy_d2gs_tcp_segment(
            key,
            TcpFlags::SYN,
            100,
            &[],
            &mut state,
            &mut |event, game_state| {
                assert!(game_state.player(7).is_some());
                events.push(event);
            },
        );

        assert!(state.player(7).is_some());
        assert!(events.iter().any(|event| matches!(
            event,
            ConnectionEvent::TransportWarning {
                warning: ConnectionTransportWarning::D2gsSessionReset {
                    reason: D2gsSessionResetReason::NewTcpStream
                }
            }
        )));
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
                        ..
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

    fn test_interface(description: &str, index: u32, ip: Ipv4Addr) -> NetworkInterface {
        NetworkInterface {
            name: format!(r"\Device\NPF_{{{index}}}"),
            description: description.to_owned(),
            index,
            mac: None,
            ips: vec![IpNetwork::new(IpAddr::V4(ip), 24).unwrap()],
            flags: 0,
        }
    }
}
