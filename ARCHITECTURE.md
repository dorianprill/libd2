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
Connection::listen_with_events(&mut GameState, callback)
    |
    v
Ethernet -> IPv4/IPv6 -> TCP/UDP filtering
    |
    v
D2GSReader::read(raw payload)
    |
    v
D2GS chunk framing and optional Huffman decompression
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
ConnectionEvent + current GameState callback
    |
    v
External tools forward snapshots/events to UI or storage
```

The live path now applies successfully parsed packets to `GameState` during
capture. The event path reports unsupported packet IDs and parse errors as
`ConnectionEvent::ParseError` so overlay tools can keep running while also
counting missing packet coverage.

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

Client::start_with_events(callback)
    Connection::init()
    Connection::listen_with_events(&mut GameState, callback)
        performs the same blocking capture loop
        calls callback(ConnectionEvent, &GameState) after each decoded packet
```

`Client` remains a blocking facade, so UI tools should run it on a worker
thread. `ConnectionEvent` is the current integration point for an overlay: the
callback can forward parsed packet events or compact snapshots of `GameState` to
the UI thread. `Connection::process_d2gs_payload` provides the same decode/parse
state-update path for fixtures and replay without live packet capture.

## Component Responsibilities

### Network Capture

`Connection` owns packet capture concerns:

- choose the network interface
- read Ethernet frames from `pnet`
- unwrap IPv4/IPv6 and TCP/UDP payloads
- classify likely Diablo II transport ports
- pass legacy plaintext D2GS payload bytes from port `4000` to `D2GSReader`
- ignore D2R/modern Battle.net port `1119` for D2GS parsing because it is
  protected transport, not legacy D2GS framing
- drain decoded packets and apply successfully parsed packets to `GameState`
- emit `ConnectionEvent` values for parsed messages and parse errors

This component still owns capture, decode, parse, and state mutation together.
The callback API is enough for a first overlay, while a future stream API can
split those stages more cleanly.

### D2GS Framing and Decompression

`D2GSReader` owns conversion from captured game-server payload bytes to
`D2GSPacket`s:

- plain packet detection
- plain TCP payload stream splitting
- compressed chunk length parsing
- Huffman decompression
- packet-size calculation for plain and decompressed packet streams
- packet queueing

Packets remain queued for callers. `Connection` currently drains that queue and
applies successfully parsed messages to `GameState`.

The plain-packet stream path is covered by unit tests, including live-observed
concatenated map-reveal and item-action bursts. The compressed-packet path has
chunk parsing and Huffman tables/decoder code, but it still needs captured
fixture tests and audit before it should be considered reliable input support.

### Protocol Messages

`ClientMessage` describes known client-to-server packet IDs. `ServerMessage`
now has an initial binary parser for a state-initialization-oriented packet
subset:

```text
0x00..0x11,
0x15,
0x18,
0x19..0x20,
0x3E,
0x51,
0x53,
0x59,
0x5A,
0x5B,
0x5C,
0x67..0x69,
0x6B..0x6D,
0x77,
0x7D,
0x8F,
0x90,
0x95,
0x96,
0x9C,
0x9D,
0xA9,
0xAB,
0xAC,
0xAF,
0xB0
```

These IDs cover game/load lifecycle packets, map reveal/hide, level warps,
object removal/handshake, movement/state basics, local HP/MP/stamina bitstreams,
simple stat and experience
updates, world objects, darkness/event envelopes, player assignment/join/left
and player-map updates, NPC movement/state/heal, trade/pong/status envelopes,
fixed item-state flag updates, variable-length monster assignment, variable
item stat-update envelopes, world/owned item action envelopes, state ending, and
compression/termination signals.

Item action packet envelopes are parsed by `ServerMessage`, but the item
bitstream itself is owned by `core::object::item`. That module currently decodes
the stable packet-time fields that do not require MPQ stat tables: flags,
version, destination/placement, item code, gold amount, used/open sockets,
level, quality, graphic/color ids, quality-specific ids, runeword metadata,
armor defense, and durability. The final item stat lists remain raw until
`ItemStatCost.txt` and related static data are available.
Server packet `0x3E` is preserved separately in `GameState` as declared-size
item-stat bitstreams, including the 1.14d padded 34-byte form, because the
packet envelope does not expose a stable item GUID to update.

The intended direction is:

```text
D2GSPacket bytes -> ServerMessage::parse -> GameState update
```

Parsing should prefer structured binary parsing and explicit little-endian field
reads over ad hoc indexing. Variable-size packets such as chat and item stat
streams should be isolated behind focused parser helpers with fixtures; the
local-player HP/MP/stamina bitstreams already follow that pattern.

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
- items by unit id, including latest owner, raw item action bits, typed
  action/category/container ids, decoded item code/quality/socket/durability
  fields, generic destination/placement fields, and latest raw `0x7D`
  item-state flags
- raw `0x3E` item-stat update bitstreams in arrival order
- local player id
- game type, difficulty, locale, ladder/expansion/hardcore flags
- active map metadata and revealed map tiles

It implements the `Update` trait with `&mut self` and mutates state for the
currently parsed packet subset: game flags, act load/unload, map reveal/hide,
player assignment/movement/join/left, local-player HP/mana/stamina/movement
verification, world object assignment/removal/state metadata, NPC
assignment/movement/state/heal/death, simple local player stat/experience
updates, and raw item-stat update preservation. Item action packets currently
upsert item owner and decoded packet-time item state; fixed `0x7D` packets
update the matching item's raw state flags by GUID.

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

`mpq.rs` is the archive layer for reading Diablo II Classic/LoD static game data
from MPQ v1 archives. It keeps all archive-specific hashing, decryption, sector,
and decompression rules behind a read-only API:

- MPQ v1 header parsing
- hash-table and block-table entry parsing
- MPQ path hashing and decryption-key derivation
- encryption-table generation
- in-place block/table decryption
- file flag and compression-type enums
- read-only archive opening from a path or in-memory fixture bytes
- logical path lookup through decrypted hash tables
- sector table decoding, encrypted sector handling, single-unit files, and
  uncompressed file copies
- PKWARE implode, zlib, and bzip2 decompression for non-stacked MPQ compression
  masks

`data.rs` sits above this and provides typed Classic/LoD game-data tables. Its
loader opens `patch_d2.mpq`, `d2exp.mpq`, and `d2data.mpq` in game precedence
order and parses only the files needed by current packet/UI state:

- `string.tbl`, `expansionstring.tbl`, and `patchstring.tbl`
- `MonStats.bin` and `MonStats2.bin`
- `Objects.bin` and `Levels.bin`
- `Weapons.bin`, `Armor.bin`, and `Misc.bin`

The integration model is:

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
TBL/bin decoders
    |
    v
GameData lookups for monster, object, item, and level names
```

This does not yet cover TXT parsing, DS1/DT1 map assets, skill/stat/property
tables, or D2R/RotW asset packaging. Packet item stat decoding should continue
to preserve raw stat streams until `ItemStatCost.txt` and related tables exist.

### Maps and Pathing

The map module now defines the render-independent contract used by generated
maps:

- `MapSeed` and unsigned seed validation
- `MapGenerationRequest`, which binds seed, difficulty, act, and area into a
  validated generator-facing request
- `GeneratedMap`, `MapObject`, `MapPoint`, and `MapSize`
- generated-map JSON normalization for both single-level output and wrapped
  generator responses containing `seed`, `difficulty`, `act`, and `levels`
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
- native Rust generator work for one area family at a time

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
  NPC assignment, player join, item stat-update envelopes, item action packets,
  and live-observed plain D2GS packet bursts.
- Compressed D2GS/Huffman decoding still lacks captured fixture validation.
- `GameState::update` only handles the parsed state-relevant subset so far.
- D2R/modern Battle.net port `1119` is intentionally classified as
  encrypted/unknown and is not decoded as D2GS.
- Item action state now decodes packet-time item fields, and `0x3E` item stat
  streams are preserved, but full item stat-list parsing and resolved item
  semantics still need game-data table integration.
- `Client::start` still blocks; UI tools must run it on a worker thread or use
  fixture/replay helpers outside the UI loop.
- `Connection` directly owns a `D2GSReader`, which couples capture to decoding.
- The callback API reports parse errors, but there is not yet a first-class
  non-blocking iterator/stream abstraction.
- Parser and state-transition unit tests exist for the initial subset, but
  captured packet fixtures are still needed.
- Native seed-to-layout map generation and pathfinding are not implemented.
- Map data currently models generated output and collision queries; it does not
  invoke or embed a Diablo II map-generation engine.
- Character-file support exists but does not yet parse every section.
- MPQ support exists only at the primitive header/hash/decrypt layer; archive
  file extraction, compression, and TXT/TBL/bin decoding still need to be built.
