# A Diablo II Core and Client Library

A Diablo II library for passive game-state reconstruction, save/static-data
parsing, and helper-tool foundations, written in Rust for performance, safety,
and re-usability without modifying the running game client.

The current focus is legacy Classic/Lord of Destruction 1.14-era packet capture
and state reconstruction, plus read-only foundations for character files, MPQ
archives, static data, and generated maps. The companion overlay/debug UI is
[d2helper](https://github.com/dorianprill/d2helper), which consumes this crate
for live LoD packet capture and automap-style visualization.

This effort is still WIP and is not available on crates.io yet.

If you are interested in Diablo II and/or Rust, this might be fun!

## Long Term Goal

In the long term, the library aims to make it possible to build high-quality
external Diablo II tooling: packet-derived live game-state visualization,
map/collision/pathing helpers, event notifications, character-file inspection
and editing, and eventually more complete client/protocol workflows.

The immediate consumer is
[d2helper](https://github.com/dorianprill/d2helper), an egui-based helper and
debug overlay for LoD 1.14 that displays packet-derived players, monsters,
objects, items, static-data names, and generated-map collision when available.

## Feature List

This list describes the current code, not the final project goal.

1. Network and protocol support
   - [x] Passive packet capture through `Client`/`Connection` using `pnet`; legacy Classic/LoD plaintext D2GS server traffic from source port `4000` is routed to the D2GS reader. Client-to-server packets with destination port `4000` are ignored for now because they use the separate client packet space. Capture opens the datalink channel without promiscuous mode because local client/server traffic is sufficient and promiscuous membership can fail on some wireless interfaces.
   - [x] Live server-to-client TCP payloads are reconstructed in sequence order before D2GS parsing. Duplicate retransmissions are ignored, overlapping retransmissions are trimmed, out-of-order segments are buffered, and bounded gap resets are reported as transport warnings.
   - [x] D2R/modern Battle.net traffic on port `1119` is classified as encrypted/unknown transport and is no longer fed into the legacy D2GS parser.
   - [x] Plain D2GS TCP payloads are split into individual `D2GSPacket`s before parsing, including live-observed concatenated map-reveal and `0x9C` item bursts.
   - [x] Compressed D2GS/Huffman framing tracks `0xAF` compression mode, supports one-byte and two-byte chunk headers, buffers compressed chunks split across TCP payloads, and is covered by Blacha-derived Huffman fixtures. More captured live compressed fixtures are still needed before claiming broad compressed-traffic compatibility.
   - [x] Parsed server packet IDs: `0x00..0x11`, `0x15`, `0x18`, `0x19..0x20`, `0x23`, `0x28`, `0x3E`, `0x47`, `0x48`, `0x4C`, `0x4D`, `0x51`, `0x53`, `0x59`, `0x5A`, `0x5B`, `0x5C`, `0x67..0x69`, `0x6B..0x6D`, `0x75`, `0x76`, `0x77`, `0x7D`, `0x8F`, `0x90`, `0x94`, `0x95`, `0x96`, `0x9C`, `0x9D`, `0xA9`, `0xAB`, `0xAC`, `0xAF`, and `0xB0`.
   - [x] Local-player HP/mana/stamina bitstreams from `0x18`, `0x95`, and `0x96` are decoded into raw packet-unit vitals, regeneration counters where present, and movement verification coordinates.
   - [ ] Missing high-priority packet parsers include party/relationship packets beyond the parsed `0x75` level update (`0x7F`, `0x8B..0x8D`), mercenary/summon updates (`0x4E`, `0x81`, `0x9E..0xA2`), chat/event streams, quest streams, and full item stat-list interpretation.
   - [ ] BNCS and MCP/Realm protocol support are not implemented; `realm_connection` currently contains declarative status/message sketches only.
2. Runtime game-state reconstruction
   - [x] Tracks game type, difficulty, locale, expansion/ladder/hardcore flags, local player id, and active act/map metadata.
   - [x] Tracks players for assignment, join/leave, movement, player-map updates, `0x75` party-info level updates, `0x94` base skill levels by global `Skills.txt` id, simple local stats, experience updates, and local-player HP/mana/stamina/movement verification.
   - [x] Tracks NPCs/monsters for assignment, movement/action/attack/stop, state, life percent, heal, and death/removal.
   - [x] Tracks world objects for assignment/removal, level-warp entrance markers from `0x09`, and object-state metadata from `0x0E` for known objects.
   - [x] Tracks map reveal/hide tiles from packets.
   - [x] Tracks item unit ids, world/unit ownership, raw item action bitstreams, typed action/category/container ids, flags, item-data version, destination/placement, item code, gold amount, used/open sockets, item level, quality, graphic/color ids, quality-specific ids, runeword metadata, armor defense, and durability from `0x9C`/`0x9D` where the packet bitstream contains those fields.
   - [x] Tracks latest raw `0x7D` item-state flags by item GUID.
   - [x] Preserves raw `0x3E` item-stat update bitstreams in arrival order, including the 1.14d padded 34-byte framing form. The packet envelope does not expose a stable item GUID, so these are not yet merged into individual `Item` records.
   - [ ] Full item stat lists, resolved item names/properties, complete ground/inventory/stash/cube/belt semantics, missiles, mercenaries, party/hostility, buffs/states, quests, and derived event notifications are not complete.
3. Character files and inventory profiles
   - [x] `.d2s` loading/parsing/saving is raw-preserving and validates magic, file size, and checksum; saving repairs size and checksum.
   - [x] Recognized save-version values are `0x47` pre-LoD, `0x57` LoD 1.07/1.08, `0x59` Classic 1.08, `0x5c` 1.09, `0x60` legacy 1.10+, and `>=0x61` D2R/modern.
   - [x] Header layouts are dispatched as legacy, D2R legacy (`0x61..=0x68`), and D2R v105+ (`>=0x69`, decimal 105).
   - [x] Edition detection covers Classic, Lord of Destruction, Resurrected, and Reign of the Warlock. Classic/LoD are distinguished by the expansion status flag; RotW is detected for D2R-encoded saves with Warlock class id `7`.
   - [x] Inventory profiles are modeled for Classic (`10x4` inventory, `6x4` stash), LoD (`10x4`, `6x8` stash), D2R (`10x4`, `10x10` personal stash, 3 shared pages), and RotW (`10x4`, `10x8` personal stash, 3 shared pages).
   - [x] Parsed save sections include header fields, D2R v105 progression and mercenary header fields, bit-packed `gf` character stats, 30-byte `if` skills, and item-related marker metadata for `JM`, `jf`, `kf`, and `lf`.
   - [x] Legacy Classic/LoD export can write a standalone LoD 1.10+ style `.d2s` from local-player `GameState`: header/status/class/level/map seed, bit-packed `gf` stats, reconstructed `0x94` skills projected into the 30-byte `if` class table, empty player item and corpse lists, and empty expansion merc/golem markers. Callers can override the skill table when a capture lacks `0x94`.
   - [x] Legacy template overlay can rewrite header, stats, and skills in an existing Classic/LoD save while preserving later save-only sections and repairing size/checksum.
   - [ ] Item records, personal/shared stash pages, semantic quest/waypoint/NPC introduction parsers, corpse payloads, detailed Iron Golem payloads, follower payload contents, and semantic save editing are not implemented.
4. MPQ and static game data
   - [x] MPQ primitives include header parsing, hash-table/block-table entry parsing, format/compression enums, path hashing, decryption-key derivation, encryption-table generation, and in-place block decryption.
   - [x] Read-only MPQ v1 archive lookup/extraction supports known logical paths, encrypted hash/block tables, sector tables, encrypted sectors, single-unit files, uncompressed sectors, PKWARE implode, zlib, and bzip2 compression masks.
   - [x] MPQ extraction is covered by captured fixture archives for both legacy implode and PKWARE compression-mask framing.
   - [x] Classic/LoD static-data loading resolves MPQ precedence (`patch_d2.mpq`, `d2exp.mpq`, `d2data.mpq`) and parses `.tbl`, `MonStats.bin`, `MonStats2.bin`, `Objects.bin`, `Levels.bin`, `Weapons.bin`, `Armor.bin`, and `Misc.bin`.
   - [ ] TXT decoders, DS1/DT1 map asset ingestion, skill/stat/property tables, and D2R/RotW CASC-style static-data loading are not implemented.
5. Maps and pathing
   - [x] Map support currently models generated output: map seeds, validated generator requests for seed/difficulty/act/area, generated-map objects, RLE collision grids, row expansion, point collision queries, level-id-to-act lookup, important-exit classification, and generated-map JSON normalization for single-level and wrapped generator responses.
   - [ ] Native map generation from seed, MPQ-backed asset loading, and DS1/DT1/excel integration are not implemented.
   - [ ] Pathfinding over static collision plus dynamic game-state overlays is not implemented.
6. Client API
   - [x] Blocking shadow-client facade with packet listener and state update loop.
   - [x] Callback-oriented capture API via `Client::start_with_events`, `Connection::listen_with_events`, and fixture/replay helpers that emit `ConnectionEvent`s with the current `GameState`.
   - [x] Packet parse errors and transport warnings are surfaced as events, which lets UI consumers keep running while showing diagnostics for unsupported packets, TCP gaps, duplicate segments, and partial D2GS buffers.
   - [ ] Active client/protocol state machine, decoded-packet iterator, and fully non-blocking stream API are not implemented.

## How to Build

Building on windows requires some extra steps, otherwise it should be smooth sailing.  
At this early stage I haven't created any bindings, but Python/TypeScript would be useful to many people, I guess.

### Linux

Tested with Diablo 2 (Legacy) and WINE
`cargo build --release`

### Mac Os

`cargo build --release` (not tested yet)

### Windows

You will need to install `ncap` or the `WinPcap Developers Pack` as per the [libpnet](https://github.com/libpnet/libpnet) build instructions for Windows (I tested the latter). Then point your user environment variable `LIB` (create if nonexistent) to the folder where to find Packet.lib i.e. `WpdPack/Lib/x64/` from the WinPcap Developers Pack you just downloaded. Then `cargo build --release`
This will get the project building.  
Currently, in order to find the internet-connected network interface, it is necessary to disable disconnected-but-enabled interfaces (such as virtual adaperts for VPN).

## Usage

One simple use case that is supported now is launching a shadow client to sniff
legacy LoD D2GS packets on port `4000`. Put the following code in your `main.rs`
and run it. Then start Diablo II LoD 1.14, join a game, and let the library keep
`GameState` updated from parsed packets.

```Rust
use libd2r::Client;

fn main() {
    let mut client = Client::new();
    client.start()
}
```

For UI tools, run the blocking listener on a worker thread and forward compact
snapshots or events to the UI thread:

```Rust
use libd2r::{Client, ConnectionEvent};

fn main() {
    let mut client = Client::new();
    client.start_with_events(|event, state| {
        if let ConnectionEvent::ServerMessage { applied: true, .. } = event {
            println!(
                "players={} npcs={} items={}",
                state.players().len(),
                state.npcs().len(),
                state.items().len()
            );
        }
    });
}
```

D2R/modern Battle.net traffic on port `1119` is classified as encrypted/unknown
transport and is not fed into the legacy D2GS parser.

For a concrete application, see
[d2helper](https://github.com/dorianprill/d2helper). It uses
`Client::start_with_events` on a worker thread, loads Classic/LoD MPQ static
data for names, and renders a packet-derived automap/debug view in egui.

## History

The first commit was on 2022-02-23. The initial version was a Rust network/TCP
sniffing layer for Diablo II game-server packets, followed by Rust translations
of the legacy Huffman decoder and packet parsers from D2BS, RedVex, and
OmegaBot-era resources. That early code already fixed up several packet layouts
for LoD 1.14 and had a simple `GameState` plus basic Diablo II data structures.

Development resumed with a broader scope: keep the packet-only legacy LoD path
useful for external tools, add version-aware save parsing for Classic, LoD, D2R,
and Reign of the Warlock, port enough MPQ/static-data support for names and map
metadata, and expose a stable library API that d2helper can build on.

## Contributing

This is quite the challenge so any help is appreciated!  
There is quite a bit of awesome code out there, but scattered across various sources.  
> Update: We now have AI to translate and consolidate all the awesome code out there into this library, so the need for help is not as big as it was before. But if you want to contribute, feel free to reach out!

## Disclaimer and Credits

This project is a reverse-engineered compatibility effort for external tooling.
It is not affiliated with Blizzard. The crate is useful today for passive LoD
1.14-era packet-derived state reconstruction, read-only MPQ/static-data loading,
generated-map JSON ingestion, and conservative character-file parsing, but many
protocol and save-editing areas are intentionally incomplete.

Here are some great resources on the original game, thanks to everyone who has been working on reverse engineering and botting for this game over the years, without you this would not be possible:

- [client-less C# bot by dkuwahara](https://github.com/dkuwahara/OmegaBot)
- [a blog post by Eric Carmichael](http://www.ericcarmichael.com/my-diablo-2-botting-phase.html)  
- [D2BS](https://github.com/noah-/d2bs)
- Another good resource is the [diablo 2 protocol js library](https://github.com/MephisTools/diablo2-protocol).
- https://github.com/blizzhackers/kolbot (Data structures and game mechanics)
- https://github.com/blizzhackers/kolbot-SoloPlay (Solo play strategy  implementation)
- [Blizzhackers/Diablo2PacketsData](https://github.com/blizzhackers/Diablo2PacketsData)
- https://github.com/blacha/diablo2  (Network traffic interception and parsing and visualization of game state)
- https://github.com/OpenDiablo2/OpenDiablo2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED)
- https://github.com/eezstreet/OpenD2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED?)
- https://github.com/nokka/d2s (character file parsing for slashdiablo's armory)
- https://github.com/krisives/d2s-format (D2 character file format specification)
- https://github.com/dschu012/D2SLib Savefile support for 1.10 (LoD) to 1.15 (d2r)
- https://github.com/emmericp/diablo2-maps (Map generation from game seed and reverse engineering of seed from map)
- https://github.com/Vitalick/go-d2editor (Character file parsing and generation, savegame load/save support, tests)
- https://github.com/crabsmadethis/d2r-horadric-tools (character loading/saving, character generation from descriptions (we can use this for testing?), MCP functionality (we dont need mcp) but we can use the character file parsing and generation)
