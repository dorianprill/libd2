extern crate pnet;

#[cfg(target_os = "windows")]
use pnet::ipnetwork::IpNetwork;

//use self::pnet::packet::ethernet::Ethernet;
use self::pnet::datalink::{self, NetworkInterface};
use self::pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use self::pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use self::pnet::packet::ipv4::Ipv4Packet;
use self::pnet::packet::ipv6::Ipv6Packet;
use self::pnet::packet::tcp::TcpPacket;
use self::pnet::packet::udp::UdpPacket;
use self::pnet::packet::Packet;
use self::pnet::util::MacAddr;

//use std::env;
//use std::io::{self, Write};
use std::net::IpAddr;
#[cfg(target_os = "windows")]
use std::net::Ipv4Addr;
use std::process;
use std::str;

//use std::collections::BinaryHeap;
//use connection::raw_packet::RawPacket;
use crate::core::game_state::GameState;
use crate::core::network::d2gs::{D2GSPacket, D2GSReader};
use crate::core::protocol::server_message::ServerMessageParseError;
use crate::core::protocol::ServerMessage;
use crate::core::update::Update;

const LEGACY_D2GS_PORT: u16 = 4000;
const D2R_BNET_PORT: u16 = 1119;

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

/// Event emitted by [`Connection`] when a legacy D2GS payload produces a packet.
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
}

impl ConnectionEvent {
    pub fn packet(&self) -> &D2GSPacket {
        match self {
            Self::ServerMessage { packet, .. } | Self::ParseError { packet, .. } => packet,
        }
    }

    pub fn packet_id(&self) -> u8 {
        self.packet().packet_id()
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
    interface: NetworkInterface,
    initialized: bool,
    //protocol_state: ProtocolState,
    d2gs_reader: D2GSReader,
    // BinaryHeap as a PriorityQueue
    //packet_queue:   BinaryHeap<RawPacket<'a>>
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            interface: datalink::interfaces().pop().unwrap(),
            initialized: false,
            d2gs_reader: D2GSReader::new(),
        }
    }

    pub fn init(&mut self) {
        // Find the first network interface connected to the internet
        // FIXME this only works on linux, for windows discerning whether an
        // interface has an internet connection is not possible with libpnet
        // maybe use
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Networking/Connectivity/struct.ConnectionProfile.html#method.GetNetworkConnectivityLevel
        let interfaces = datalink::interfaces();
        // linux/MacOs: should be easy to find an internet connected interface.
        // TODO how to ensure it is the default iprouted one?
        #[cfg(not(target_os = "windows"))]
        let some_if: Option<NetworkInterface> = Some(
            interfaces
                .into_iter()
                .find(|ref ifx| ifx.is_up() && !ifx.is_loopback() && !ifx.ips.is_empty())
                .unwrap(),
        );
        // windows: libpnet is not really helpful on windows as is_up() is always false.
        // additionally, there is no way to tell between a regular interface and a
        // disconnected interface with an ip (e.g. virtual adapter for VPN)
        // for use on windows, you should disable all devices that are not in use even if they are not connected.
        #[cfg(target_os = "windows")]
        let some_if: Option<NetworkInterface> = Some(
            interfaces
                .into_iter()
                .find(|ref ifx| {
                    *(ifx.ips.first().unwrap())
                        != IpNetwork::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0).unwrap()
                })
                .unwrap(),
        );

        if some_if != None {
            self.interface = some_if.unwrap();
            println!("Identified network interface {}", self.interface);
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
        use self::pnet::datalink::Channel::Ethernet;
        let interface = self.interface.clone();
        if !self.initialized {
            println!("Connection: must init() before listen()");
            return;
        }
        let capture_config = datalink::Config {
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
                        let version = Ipv4Packet::new(&packet).unwrap().get_version();

                        fake_ethernet_frame.set_destination(MacAddr(0, 0, 0, 0, 0, 0));
                        fake_ethernet_frame.set_source(MacAddr(0, 0, 0, 0, 0, 0));
                        if version == 4 {
                            fake_ethernet_frame.set_ethertype(EtherTypes::Ipv4);
                            continue;
                        } else if version == 6 {
                            fake_ethernet_frame.set_ethertype(EtherTypes::Ipv6);
                            continue;
                        }
                        fake_ethernet_frame.set_payload(&packet);
                        self.handle_ethernet_frame(
                            &interface,
                            &fake_ethernet_frame.to_immutable(),
                            game_state,
                            &mut on_event,
                        );
                    }
                    self.handle_ethernet_frame(
                        &interface,
                        &EthernetPacket::new(packet).unwrap(),
                        game_state,
                        &mut on_event,
                    );
                }
                Err(e) => panic!("unable to receive packet: {}", e),
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
        F: FnMut(ConnectionEvent, &GameState),
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
        _source: IpAddr,
        _destination: IpAddr,
        packet: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
    {
        let tcp = TcpPacket::new(packet);
        if let Some(tcp) = tcp {
            // filter packet by ports used by d2, will continue for both sent & received
            //if !PORTS.contains(&tcp.get_destination()) {//&& !PORTS.contains(&tcp.get_source()) {
            //    return
            //}
            match classify_transport(tcp.get_source(), tcp.get_destination()) {
                CapturedTransport::LegacyD2gsServerToClient => {
                    self.read_d2gs_payload(tcp.payload(), game_state, on_event)
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
        _interface_name: &str,
        source: IpAddr,
        destination: IpAddr,
        protocol: IpNextHeaderProtocol,
        packet: &[u8],
        game_state: &mut GameState,
        on_event: &mut F,
    ) where
        F: FnMut(ConnectionEvent, &GameState),
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
        F: FnMut(ConnectionEvent, &GameState),
    {
        let header = Ipv4Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(
                interface_name,
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
        F: FnMut(ConnectionEvent, &GameState),
    {
        let header = Ipv6Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(
                interface_name,
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
        F: FnMut(ConnectionEvent, &GameState),
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
        self.read_d2gs_payload(payload, game_state, &mut on_event);
    }

    fn read_d2gs_payload<F>(&mut self, payload: &[u8], game_state: &mut GameState, on_event: &mut F)
    where
        F: FnMut(ConnectionEvent, &GameState),
    {
        self.d2gs_reader.read(payload);
        while let Some(packet) = self.d2gs_reader.next() {
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
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_transport, CapturedTransport, Connection, ConnectionEvent, D2R_BNET_PORT,
        LEGACY_D2GS_PORT,
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
        assert_eq!(state.local_player_id(), Some(0x0102_0304));
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
}
