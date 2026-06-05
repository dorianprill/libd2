# A Diablo II Core and Client Library 

A Diablo II library for core and simple client functionality, written in Rust for performance, safety and re-usability without any runtime requirements.


This effort is very much WIP, so it is not available on crates.io yet.  

If you are interested in Diablo II and/or Rust, this might be fun!

## Long Term Goal

In the long term, the library aims to make it possible to write a headless client that can connect to the game servers and interact with the game world.
An immediate use case is a game helper tool that visualizes the game state in real time (e.g. a map overlay, item and buff tracking, character reading for inspection, editing and download from bnet, etc. ) along with some QoL-functionality.


## Feature List

This list describes the current code, not the final project goal.

1. Network and protocol support
   - [x] Passive packet capture through `Client`/`Connection` using `pnet`; legacy Classic/LoD plaintext D2GS server traffic from source port `4000` is routed to the D2GS reader. Client-to-server packets with destination port `4000` are ignored for now because they use the separate client packet space. Capture opens the datalink channel without promiscuous mode because local client/server traffic is sufficient and promiscuous membership can fail on some wireless interfaces.
   - [x] D2R/modern Battle.net traffic on port `1119` is classified as encrypted/unknown transport and is no longer fed into the legacy D2GS parser.
   - [x] Plain D2GS TCP payloads are split into individual `D2GSPacket`s before parsing, including live-observed concatenated map-reveal and `0x9C` item bursts.
   - [ ] Compressed D2GS packets have corrected chunk-size handling, a packet-size table, and Huffman decoder scaffolding, but still need captured fixture tests before they should be treated as supported.
   - [x] Parsed server packet IDs: `0x00..0x11`, `0x15`, `0x18`, `0x19..0x20`, `0x3E`, `0x51`, `0x53`, `0x59`, `0x5A`, `0x5B`, `0x5C`, `0x67..0x69`, `0x6B..0x6D`, `0x77`, `0x7D`, `0x8F`, `0x90`, `0x95`, `0x96`, `0x9C`, `0x9D`, `0xA9`, `0xAB`, `0xAC`, `0xAF`, and `0xB0`.
   - [x] Local-player HP/mana/stamina bitstreams from `0x18`, `0x95`, and `0x96` are decoded into raw packet-unit vitals, regeneration counters where present, and movement verification coordinates.
   - [ ] Missing high-priority packet parsers include party/relationship packets (`0x75`, `0x7F`, `0x8B..0x8D`), mercenary/summon updates (`0x4E`, `0x81`, `0x9E..0xA2`), chat/event streams, quest streams, and full item stat-list interpretation.
   - [ ] BNCS and MCP/Realm protocol support are not implemented; `realm_connection` currently contains declarative status/message sketches only.
2. Runtime game-state reconstruction
   - [x] Tracks game type, difficulty, locale, expansion/ladder/hardcore flags, local player id, and active act/map metadata.
   - [x] Tracks players for assignment, join/leave, movement, player-map updates, level, simple local stats, experience updates, and local-player HP/mana/stamina/movement verification.
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
   - [ ] Item records, personal/shared stash pages, quests, waypoints, NPC introductions, corpse payloads, detailed Iron Golem payloads, follower payload contents, and semantic save editing are not implemented.
4. MPQ and static game data
   - [x] MPQ primitives include header parsing, hash-table/block-table entry parsing, format/compression enums, path hashing, decryption-key derivation, encryption-table generation, and in-place block decryption.
   - [ ] Full archive lookup/extraction, sector table handling, compression/decompression dispatch, and file fixtures are not implemented.
   - [ ] TXT/TBL/bin decoders and typed game-data tables are not implemented.
5. Maps and pathing
   - [x] Map support currently models generated output: map seeds, validated generator requests for seed/difficulty/act/area, generated-map objects, RLE collision grids, row expansion, point collision queries, level-id-to-act lookup, important-exit classification, and generated-map JSON normalization for single-level and wrapped generator responses.
   - [ ] Native map generation from seed, MPQ-backed asset loading, and DS1/DT1/excel integration are not implemented.
   - [ ] Pathfinding over static collision plus dynamic game-state overlays is not implemented.
6. Client API
   - [x] Blocking shadow-client facade with packet listener and state update loop.
   - [x] Callback-oriented capture API via `Client::start_with_events`, `Connection::listen_with_events`, and fixture/replay helpers that emit `ConnectionEvent`s with the current `GameState`.
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

## Contributing

This is quite the challenge so any help is appreciated!  
There is quite a bit of awesome code out there, but scattered across various sources.  
> Update: We now have AI to translate and consolidate all the awesome code out there into this library, so the need for help is not as big as it was before. But if you want to contribute, feel free to reach out!

## Disclaimer and Credits

Little of this works yet and probably never will as i haven't started  working on the tool that will use this library as a dependency (see section `Long term Goal`).

Here are some great resources on the original game, thanks to everyone who has been working on reverse engineering and botting for this game over the years, without you this would not be possible:

- [client-less C# bot by dkuwahara](https://github.com/dkuwahara/OmegaBot)
- a [blog post by Eric Carmichael](http://www.ericcarmichael.com/my-diablo-2-botting-phase.html)  
- and, of course, [D2BS](https://github.com/noah-/d2bs)
- Another good resource is the [diablo 2 protocol js library](https://github.com/MephisTools/diablo2-protocol).
- - https://github.com/blizzhackers/kolbot (Data structures and game mechanics)
- https://github.com/blizzhackers/kolbot-SoloPlay (Solo play strategy  implementation)
- https://github.com/blacha/diablo2  (Network traffic interception and parsing and visualization of game state)
- https://github.com/OpenDiablo2/OpenDiablo2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED)
- https://github.com/eezstreet/OpenD2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED?)
- https://github.com/nokka/d2s (character file parsing for slashdiablo's armory)
- https://github.com/krisives/d2s-format (D2 character file format specification)
- https://github.com/dschu012/D2SLib Savefile support for 1.10 (LoD) to 1.15 (d2r)
- https://github.com/emmericp/diablo2-maps (Map generation from game seed and reverse engineering of seed from map)
- https://github.com/Vitalick/go-d2editor (Character file parsing and generation, savegame load/save support, tests)
- https://github.com/crabsmadethis/d2r-horadric-tools (character loading/saving, character generation from descriptions (we can use this for testing?), MCP functionality (we dont need mcp) but we can use the character file parsing and generation)
