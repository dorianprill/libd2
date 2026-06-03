# libd2r Architecture

This document captures the current architecture and intended direction of the
library. It should be updated whenever modules are added, packet flow changes,
or state ownership changes.

## Current Goal

The first phase is a passive, external Diablo II state reconstruction library.
It should observe network traffic between an original client and server, decode
game-server packets, parse them into typed messages, and apply those messages to
an in-memory game state. The library must remain useful to external tools
without requiring client memory access.

## Current Module Layout

```text
src/lib.rs
  public crate exports

src/client/
  Client facade for starting the shadow client

src/core/
  act.rs, area.rs, character_class.rs, coordinate.rs, unit_stat.rs
    shared Diablo II domain enums and values

  network/
    connection.rs
      interface selection and packet capture through pnet
    d2gs/
      d2gs_reader.rs
        D2GS frame/chunk handling, decompression dispatch, packet queue
      d2gs_packet.rs
        validated decompressed packet wrapper
    huffman.rs
      Diablo II Huffman decode tables and decoder helpers
    raw_packet.rs
      planned priority-queue raw packet representation

  protocol/
    client_message.rs
      declarative client-to-server D2GS message IDs
    server_message.rs
      declarative server-to-client D2GS message IDs

  game_state.rs
    indexed in-memory game-state container, map state, metadata enums,
    packet-to-state update logic

  version.rs
    runtime game-edition, save-version, and character-status detection

  inventory.rs
    edition-specific inventory/stash/cube profiles and save item-placement
    enums

  character_file.rs
    conservative .d2s header parser, checksum validator, stats/skills parser,
    item-related section-header scanner, D2R v105 header handling, and
    raw-preserving loader/saver

  mpq.rs
    MPQ v1 header/table structs, hashing, decryption-key derivation,
    encryption-table generation, and in-place block decryption primitives

  entity/
    Entity trait and current player, NPC, mercenary, missile shells

  object/
    item shell and item buffer IDs

  map/
    generated-map data model, RLE collision rows, seed validation, act lookup,
    and important-exit classification
```

## Data Flow

```text
Network interface
    |
    v
pnet datalink channel
    |
    v
Connection::listen(&mut GameState)
    |
    v
Ethernet -> IPv4/IPv6 -> TCP/UDP filtering
    |
    v
D2GSReader::read(raw payload)
    |
    v
D2GS chunk framing and Huffman decompression
    |
    v
D2GSPacket queue
    |
    v
ServerMessage::parse
    |
    v
GameState::update(ServerMessage)
    |
    v
External tools query state or subscribe to derived events
```

The live path now applies successfully parsed packets to `GameState` during
capture. Unsupported packet IDs are ignored by the live connection path until
parsers are implemented for them.

## Control Flow

```text
Client::new()
    creates default GameState and Connection

Client::start()
    Connection::init()
        selects a non-loopback network interface
    Connection::listen(&mut GameState)
        blocks in a packet receive loop
        dispatches Diablo II game-server traffic to D2GSReader
        drains decoded packets and applies parsed messages to GameState
```

`Client` is currently a blocking facade. Future integration points should avoid
forcing every consumer into this exact loop. A practical next step is to split
packet capture, packet decoding, and state application into independently
testable components.

## Component Responsibilities

### Network Capture

`Connection` owns packet capture concerns:

- choose the network interface
- read Ethernet frames from `pnet`
- unwrap IPv4/IPv6 and TCP/UDP payloads
- filter likely D2GS ports, currently `4000` and `1119`
- pass payload bytes to `D2GSReader`
- drain decoded packets and apply successfully parsed packets to `GameState`

This component still couples capture to state application. A future callback or
stream API should let consumers choose whether they want raw packets, parsed
messages, state mutation, or all three.

### D2GS Framing and Decompression

`D2GSReader` owns conversion from captured game-server payload bytes to
decompressed `D2GSPacket`s:

- plain packet detection
- compressed chunk length parsing
- Huffman decompression
- packet-size calculation for decompressed packet streams
- packet queueing

Packets remain queued for callers. `Connection` currently drains that queue and
applies successfully parsed messages to `GameState`.

### Protocol Messages

`ClientMessage` describes known client-to-server packet IDs. `ServerMessage`
now has an initial binary parser for a fixed-size, state-initialization-oriented
packet subset.

The intended direction is:

```text
D2GSPacket bytes -> ServerMessage::parse -> GameState update
```

Parsing should prefer structured binary parsing and explicit little-endian field
reads over ad hoc indexing. Variable-size packets such as chat, item stat
streams, and bit-packed HP/MP updates should be isolated behind focused parser
helpers with fixtures.

The current parser uses a small native Rust cursor instead of `deku` or another
parser framework. That keeps fixed little-endian packet parsing allocation-free,
explicit, and easy to test. If variable bitstreams become large enough to justify
a parser library, that decision should be revisited with benchmarks and fixture
coverage.

### Game State

`GameState` currently stores indexed runtime memory:

- players by unit id
- NPCs by unit id
- world objects by unit id
- items by unit id
- local player id
- game type, difficulty, locale, ladder/expansion/hardcore flags
- active map metadata and revealed map tiles

It implements the `Update` trait with `&mut self` and mutates state for the
currently parsed packet subset: game flags, act load/unload, map reveal/hide,
player assignment/movement/join/left, world object assignment/removal, NPC
assignment/movement/state/heal/death, and simple local player stat/experience
updates.

### Entities and Objects

`Entity` provides the shared ID/location surface for player, NPC, mercenary, and
missile shells. Items are represented separately under `object`.

Future state updates should preserve Diablo II's unit identity model while still
presenting ergonomic Rust APIs to consumers.

### Versions and Inventory Profiles

Edition-specific behavior is modeled at runtime rather than through cargo
features. External tools should be able to load traffic or save files, let the
library detect the relevant version, and call the matching implementation
without rebuilding the crate.

The central model is:

```text
.d2s header / runtime metadata
    |
    v
SaveVersion + CharacterStatus
    |
    v
GameEdition
    |
    v
InventoryProfile
```

`InventoryProfile` captures the storage dimensions and encoding choice that
callers need before detailed item parsing. Built-in profiles currently cover:

- Classic: 10x4 inventory, 6x4 stash, 3x4 cube, no shared stash.
- Lord of Destruction: 10x4 inventory, 6x8 stash, 3x4 cube, no shared stash.
- Resurrected: 10x4 inventory, 10x10 personal stash, 3 shared stash pages,
  3x4 cube, and D2R item encoding.
- Reign of the Warlock: D2R v105/Warlock-class profile with 10x4 inventory,
  10x8 personal stash, 3 shared stash pages, 3x4 cube, and D2R item encoding.

This keeps low-friction auto-detection for consumers while still leaving room
for custom profiles if modded editions diverge from the built-ins.

### Character Files

`CharacterFile` loads `.d2s` bytes, validates the stable header fields, detects
the edition, selects an inventory profile, and preserves the original byte
buffer. Saving currently writes back the preserved bytes after recalculating the
file-size and checksum fields.

The parser intentionally starts with the stable fields shared by the public
resources: magic, version, file size, checksum, name, status flags, class,
level, D2R v105 progression, D2R v105 mercenary header fields, the bit-packed
`gf` character-stat section, the 30-byte `if` skills section, and item-related
section headers (`JM`, `jf`, `kf`, `lf`). These literal save markers are exposed
through `SaveSectionMarker` so public code can use descriptive names while the
parser remains tied to the actual on-disk bytes.

Header handling is version-dispatched. Legacy and older D2R-style layouts still
use the offsets from the existing resources, while D2R v105 uses the Horadric
Tools layout: status at `0x14`, progression at `0x15`, class at `0x18`, level
at `0x1b`, mercenary header fields at `0xa3..0xae`, and character name at
`0x12b`.

Quest, waypoint, NPC-introduction, corpse, item, iron-golem, follower, personal
stash, and shared stash sections should be added as separate parsers behind
`SaveVersion` and `InventoryProfile`, because item encoding differs between
legacy/LoD saves and Diablo II: Resurrected.

The current item-related scanner deliberately stops at marker/count metadata.
It exposes parent item-list counts, the optional corpse marker, the Iron Golem
active flag, and follower payload length validation, but it does not yet infer
variable item record boundaries.

### MPQ Archives and Data Files

`mpq.rs` is the first layer for reading Diablo II static game data from MPQ
archives. It is intentionally limited to low-level, dependency-free primitives:

- MPQ v1 header parsing
- hash-table and block-table entry parsing
- MPQ path hashing and decryption-key derivation
- encryption-table generation
- in-place block/table decryption
- file flag and compression-type enums

The integration model should be layered:

```text
MPQ bytes or file
    |
    v
MpqHeader + decrypted hash/block tables
    |
    v
read-only file lookup/extract
    |
    v
TXT/TBL/bin decoders
    |
    v
typed game-data tables for names, stats, items, maps, and skills
```

Compression and full archive extraction are deliberately not mixed into the
primitive module. The next MPQ layer should provide a reader abstraction over
files and memory buffers, then add sector extraction and compression support
behind small functions. Diablo II classic MPQs need PKWARE implode support;
later support can add zlib/BZip2 if fixtures require it.

### Maps and Pathing

The map module now defines the render-independent contract used by generated
maps:

- `MapSeed` and unsigned seed validation
- `GeneratedMap`, `MapObject`, `MapPoint`, and `MapSize`
- `CollisionGrid` over Blaine's alternating filled/open run-length rows
- row expansion and point collision queries
- level-id to act lookup
- "good exit" classification for common high-value exits

This mirrors the JSON shape emitted by `packages/map` without importing the
Node server, canvas renderer, process pool, or Wine/C map-generation binary.
Those parts are integration tools, not core Rust library logic.

Future map work should be separated into:

- seed and difficulty/act inputs
- generated static layout
- dynamic collision overlays from game state
- pathfinding queries over collision data
- optional external map-generator integration for classic clients until a
  native Rust generator exists

## Public API Boundary

The crate currently re-exports the main domain types from `src/lib.rs`.
Consumers can construct `Client` and access type definitions, but there is not
yet a stable state-query API.

Near-term public API should focus on:

- decoded packet iterator or callback
- typed server-message parser
- state snapshot/query accessors
- event stream for notable derived facts such as item drops

## Testing Strategy

The project needs tests at three levels:

- unit tests for framing, Huffman decompression, packet-size calculation, and
  individual binary parsers
- fixture tests from captured D2GS payloads with expected message sequences
- state-transition tests that apply message sequences and assert `GameState`
  contents

Tests should not require live packet capture. Live sniffing is integration
behavior and should remain optional.

## Known Architectural Gaps

- `ServerMessage::parse` covers a first fixed-size subset plus variable-length
  NPC assignment and player join packets.
- `GameState::update` only handles the parsed state-relevant subset so far.
- `Client::start` blocks and has no callback/stream API.
- `Connection` directly owns a `D2GSReader`, which couples capture to decoding.
- Live capture currently ignores parse errors for unsupported packet IDs.
- Parser and state-transition unit tests exist for the initial subset, but
  captured packet fixtures are still needed.
- Native seed-to-layout map generation and pathfinding are not implemented.
- Map data currently models generated output and collision queries; it does not
  invoke or embed a Diablo II map-generation engine.
- Character-file support exists but does not yet parse every section.
- MPQ support exists only at the primitive header/hash/decrypt layer; archive
  file extraction, compression, and TXT/TBL/bin decoding still need to be built.
