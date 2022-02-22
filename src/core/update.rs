use crate::core::network::d2gs::D2GSPacket;

/// Update trait for state update of all data structures that can be updated with a received game packet.
/// Doesn't need to bee and Entity (e.g. GameState is no Entity but can still be updated through the trait).
/// Usually contains match arms on packetId (packet[0]) and may delegate to further update handlers
/// e.g. GameState.update() -> Player.update() -> Belt.update()
pub trait Update {
    fn update(&self, p: D2GSPacket) -> bool;
}
