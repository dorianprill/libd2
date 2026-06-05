# libd2r Report

Last updated: 2026-06-05

## Current Status

The repository contains an early Rust implementation of a passive Diablo II
state reconstruction library. Network capture, legacy D2GS payload routing,
packet framing, typed server-message parsing for a first subset, `GameState`
mutation, and callback events are now wired together through the `Client` and
`Connection` facades. The plain legacy D2GS packet stream path is covered by tests;
compressed D2GS/Huffman support has tables and code scaffolding but still needs
captured fixture tests before it should be treated as production-supported.
D2R/modern Battle.net port `1119` is classified as encrypted/unknown transport
and is not routed into the legacy D2GS reader.

The crate also has read-only/static-data foundations: a raw-preserving `.d2s`
loader/saver with legacy, D2R, and Reign of the Warlock detection paths; MPQ
header/hash/decrypt primitives; and generated-map/collision data structures.
Native seed-to-layout map generation, pathfinding, full MPQ extraction, item
record parsing, and save editing remain future work.

The crate now resolves dependencies and passes:

```text
cargo test
```

The current suite has 72 unit tests.

## Work Completed

- Parsed the repository structure and current code paths.
- Added `ARCHITECTURE.md` with current module responsibilities, packet/state
  data flow, control flow, testing strategy, and known gaps.
- Added this cumulative `REPORT.md` for continuity between sessions.
- Removed `deku` after comparing the immediate parser need against the protocol
  shape. The first parser layer only needs explicit little-endian scalar reads,
  so a small native Rust cursor is simpler and avoids unused dependencies.
- Removed the obsolete crate-level `#![feature(arbitrary_enum_discriminant)]`;
  the feature has been stable on Rust since 1.66 and stable Rust rejects the
  feature gate.
- Cleaned low-risk warnings from unused imports, unreachable enum display
  fallback arms, intentionally unused placeholder parameters, and dormant fields
  or tables.
- Verified `cargo check` succeeds with Rust 1.85.1.
- Applied default `rustfmt` formatting across the repository.
- Added `ServerMessage::parse` plus `TryFrom<&D2GSPacket>` for an initial
  fixed-size packet subset: game/load lifecycle packets, map reveal/hide,
  level warps, object removal/handshake, movement/state basics, simple
  stat/experience updates, assign player, and player left.
- Added 7 unit tests covering direct parser behavior, little-endian field
  decoding, decoded plain-packet handoff from `D2GSReader`, truncated packet
  rejection, unsupported packet IDs, and empty input.
- Checked blacha/diablo2, Kolbot/D2BS, and OpenD2 for game memory/state
  structure. The first Rust state model follows blacha's packet-derived live
  state shape most closely: indexed maps for players, NPC units, objects, and
  items, with map metadata and local player tracking.
- Changed `Update` to take `&mut self` and implemented real `GameState` updates
  for the parsed packet subset.
- Added indexed in-memory state for players, NPCs, world objects, items, map
  metadata, revealed map tiles, local player id, mode flags, and player stats.
- Extended `ServerMessage::parse` for world objects, player join, NPC
  movement/state/action/attack/stop/heal, and variable-length NPC assignment.
- Wired `Connection::listen(&mut GameState)` so live D2GS packets are decoded,
  parsed, and applied to state.
- Changed `D2GSReader` to leave packets queued for callers instead of draining
  and printing them internally.
- Fixed D2GS packet-size detection for variable-length `0x5B` packets to read a
  little-endian short.
- Fixed live LoD D2GS TCP payload handling so one captured payload can contain
  several back-to-back server packets. This removes false oversized-packet
  errors for map reveal bursts, vendor/item bursts, player assignment bursts,
  and common NPC/player movement bursts.
- Corrected the 1.14d packet-size table entry for `0x01` game flags from 9 to
  8 bytes and added stream-splitting tests from live capture byte sequences.
- Removed noisy debug output from the D2GS/Huffman path and made Huffman decode
  append to a vector instead of attempting to copy into a zero-length slice.
- Added parsers for common live-observed server packets `0x53`, `0x5A`,
  `0x77`, `0x8F`, `0x90`, `0x95`, `0x96`, `0xA9`, `0xAF`, and `0xB0`.
- Wired `0x90 PlayerMapUpdate` into `GameState` for known player coordinates.
- Added state-transition tests; the suite now covers packet bytes ->
  `ServerMessage` -> `GameState` for player assignment, world objects, and NPC
  assignment, plus direct state updates for movement, death/removal, flags,
  stats, and XP.
- Compared dschu012/d2s, Vitalick/go-d2editor, krisives/d2s-format, and the
  D2R item-format notes from d07RiV for save-header, checksum, inventory,
  stash, and item-placement structures.
- Added runtime edition modeling with `GameEdition`, `SaveVersion`,
  `CharacterStatus`, and `detect_edition` instead of edition feature gates.
- Added `InventoryProfile` and item-placement enums for Classic, Lord of
  Destruction, Resurrected, and a placeholder Reign of the Warlock profile.
- Added a conservative raw-preserving `CharacterFile` `.d2s` loader/saver. It
  validates magic, file size, checksum, name, status flags, class, level, and
  selects the matching inventory profile. Saving recalculates file size and
  checksum.
- Added tests for edition detection, inventory profiles, D2S header parsing,
  D2R name-offset handling, checksum failure, and writeback checksum repair.
- Reviewed `crabsmadethis/d2r-horadric-tools` at commit `87a100d`. It is MIT
  licensed, D2R-focused, has public format docs and tests, and is worth adding
  to the project resource list.
- Ported the D2R v105 header layout from Horadric Tools: status/progression,
  class/level, mercenary header fields, and character name offsets.
- Added `CharacterHeaderLayout`, `CharacterProgression`, `MercenaryHeader`,
  `CharacterStats`, `CharacterStat`, and `CharacterSkills`.
- Added bit-packed `gf` character-stat parsing and 30-byte `if` skills parsing.
- Added read-only item-related section header parsing for `JM` item-list parent
  counts, `jf` corpse marker, `kf` Iron Golem flag, and `lf` follower block
  count/payload length.
- Added `SaveSectionMarker` as a descriptive Rust API over the literal `.d2s`
  section marker bytes: `gf`, `if`, `JM`, `jf`, `kf`, and `lf`.
- Added Warlock class id `7` and D2R v105 Warlock detection as
  `GameEdition::ReignOfTheWarlock`.
- Updated the Reign of the Warlock inventory profile to the Horadric Tools
  documented 10x8 personal stash while retaining D2R item encoding.
- Reviewed `blacha/diablo2/packages/mpq` at commit `45c91b3`. It is MIT
  licensed and provides a focused TypeScript MPQ v1 reader with hash/block
  table parsing, MPQ hashing, decryption, sector extraction, and PKWARE
  decompression.
- Added `core::mpq` with Rust MPQ primitives: header/hash/block entry parsers,
  format/compression/hash enums, file flag constants, encryption-table
  generation, MPQ path hashing, decryption-key derivation, and in-place block
  decryption.
- Added MPQ tests using Blaine's hash vectors, decryption-key vectors,
  encryption-table prefix, and a direct decrypt fixture prefix from the MPQ
  package tests.
- Changed live packet capture to open pnet datalink channels without
  promiscuous mode. The library only needs local client/server traffic for
  Diablo II helpers, and promiscuous membership can fail with `ENODEV` on some
  Linux wireless drivers even when the interface exists.
- Fixed live LoD capture direction handling so only server-to-client packets
  with source port `4000` enter the D2GS server-message reader. Client-to-server
  packets with destination port `4000` use a different packet space and were
  causing false server packet length mismatches when parsed as D2GS messages.
- Applied `0x09 AssignLevelWarp` packets to `GameState` as world objects so UI
  consumers can display packet-observed entrance/exit markers.
- Reviewed `blacha/diablo2/packages/map` at commit `45c91b3`. The package is
  MIT licensed and mostly wraps a C/Wine Diablo II 1.13c map-generation binary
  with a Node server, cache, renderer, and MPQ-backed name enrichment.
- Replaced the old map sketch with a concrete `core::map` module: generated-map
  structs, map objects, RLE collision grid, row expansion, point collision
  queries, map seed validation, level-id to act lookup, and good-exit
  classification based on Blaine's generator rules.
- Added map tests for seed range, act lookup, RLE collision decoding, grid
  bounds, and good-exit classification.
- Added variable-length parsing for server item packets `0x3E`, `0x9C`, and
  `0x9D`, including D2GS packet-size detection for `0x3E`.
- Added `ItemOwner`, typed item action/category/container enums, `ItemFlags`,
  `ItemDestination`, `ItemPlacement`, and `ItemPacketData` for item action
  bitstreams: flags, item-data version, destination/placement, item code, gold
  amount, used/open sockets, item level, quality, graphic/color ids,
  quality-specific ids, runeword metadata, armor defense, and durability where
  those fields are present.
- Wired `GameState` item updates for world and owned item action packets,
  preserving raw item bits for later stat-list parsing.
- Added item parser fixture tests using captured-style packet bytes and
  state-transition tests.
- Added generated-map JSON ingestion for the external generator output contract
  used by `@diablo2/map`, including generated-map import tests.
- Added explicit network transport classification so legacy port `4000` is
  parsed as D2GS while D2R/modern port `1119` is treated as protected transport
  and ignored by the D2GS reader.
- Added `MapGenerationRequest` and map-generation normalization errors for the
  seed/difficulty/act/area boundary. The request API exposes the numeric values
  expected by external generators and normalizes either a single generated level
  JSON object or a wrapped `seed`/`difficulty`/`act`/`levels` response into the
  requested `GeneratedMap`.
- Added `ConnectionEvent`, `Connection::listen_with_events`,
  `Connection::process_d2gs_payload`, `Client::start_with_events`, and
  `Client::process_d2gs_payload`. These APIs let overlay tools run blocking
  capture on a worker thread and forward parsed packet events or compact
  `GameState` snapshots to a UI thread.
- Added fixture-style tests for successful callback/event state mutation,
  parse-error reporting, and the client facade payload replay path.

## Current Architecture Summary

The live network runtime path is:

```text
Client::start / Client::start_with_events
  -> Connection::init
  -> Connection::listen(_with_events)(&mut GameState)
  -> Ethernet/IP/TCP/UDP filtering
  -> D2GSReader::read
  -> D2GSPacket queue
  -> ServerMessage::parse
  -> GameState::update
  -> optional ConnectionEvent callback
```

The character-file path is:

```text
.d2s bytes
  -> CharacterFile::parse
  -> stable header validation
  -> SaveVersion + CharacterStatus
  -> GameEdition
  -> InventoryProfile
  -> optional gf stats + if skills
  -> optional SaveSectionMarker-backed section headers
  -> raw-preserving save with repaired size/checksum
```

The static-data path now starts as:

```text
MPQ bytes
  -> MpqHeader::parse
  -> encrypted table bytes
  -> mpq_hash / mpq_decryption_key / decrypt_mpq_block
  -> MpqHashEntry / MpqBlockEntry
  -> future read-only archive extraction
```

## Challenges and Risks

- `ServerMessage::parse` covers only a first subset. Many variable-length
  packets and bit-packed packets still need dedicated parsers.
- D2R/modern live Battle.net traffic is not passive-D2GS-decodable in this
  crate. Support should come from offline files, static data, generated maps,
  or already-decoded plaintext fixtures.
- Item action packet support decodes packet-time fields, but full stat-list
  interpretation still needs game-data tables such as `ItemStatCost.txt`.
- The compressed D2GS/Huffman path has implementation scaffolding but lacks
  captured fixture tests and should be audited before any support claim.
- `CharacterFile` currently parses the stable header, D2R v105 progression and
  mercenary header fields, character stats, and skills. Quests, waypoints,
  NPC-introduction bytes, item records, detailed iron-golem payloads, detailed
  follower payloads, and stash pages still need section parsers.
- `core::map` models generated map output and collision queries, but does not
  generate maps from seed natively. Blaine's package relies on the original
  game DLLs through a C/Wine helper for that hard part.
- MPQ support currently stops at archive primitives. Full file extraction still
  needs a reader abstraction, hash-table lookup, sector table handling,
  compression dispatch, and fixtures against known MPQ files.
- D2R item codes are Huffman-coded and use a different bit layout than legacy
  1.10+ saves; item parsing must dispatch through `SaveVersion` rather than a
  single shared bit layout.
- Reign of the Warlock storage dimensions and item encoding still need primary
  validation. The current profile is an explicit placeholder, not a final
  compatibility claim.
- `Connection` currently couples capture, decode, parse, and state application.
  The callback API is sufficient for a first overlay, but a future stream
  boundary should expose each stage independently.
- Several packets are variable length or bit-packed and need focused parsing
  helpers and fixtures.
- Live packet capture depends on host networking and privileges; tests should
  use byte fixtures instead.
- Native map generation and pathfinding are still missing.
- Current README/repository naming still refers to `libd2r` and Diablo II:
  Resurrected in places, while `AGENTS.md` describes broader Classic, Lord of
  Destruction, Resurrected, and Reign of the Warlock support.
- The working tree already contained case-sensitive README changes
  (`readme.md` deleted and `README.md` untracked) plus untracked `AGENTS.md`;
  those were not reverted.

## Recommended Next Steps

1. Add captured fixture tests for compressed D2GS/Huffman decompression before
   treating compressed legacy traffic as supported.
2. Expand item stat-list interpretation after game-data table loading exists,
   especially `ItemStatCost.txt` bit widths and parameter rules.
3. Expand `ServerMessage::parse` with the next state-relevant variable-length
   packets after checking each layout against multiple resources.
4. Add state support for missiles, party/relationship data, mercenaries, richer
   item semantics, and event derivation.
5. Create a small LoD 1.14 `d2helper` prototype using the callback API, with an
   egui worker-thread/channel boundary and an automap-style isometric debug
   renderer over `GameState`.
6. Create captured-payload fixtures and broader state-transition tests.
7. Build read-only MPQ archive extraction on top of `core::mpq`: file/memory
   reader abstraction, decrypted hash/block tables, path lookup, sector
   extraction, and uncompressed file support.
8. Add PKWARE implode decompression for classic Diablo II MPQs, preferably
   behind a focused module with fixture tests from Blaine's package.
9. Add TXT/TBL/bin data integration for resolving monster, object, item, skill,
   and string names from extracted game files.
10. Add pathfinding over `CollisionGrid` plus dynamic overlays from live
   `GameState` units and objects.
11. Implement `.d2s` quest and waypoint section parsers next; these are
   marker-delimited and lower risk than item rewriting.
12. Add a version-dispatched item bitstream reader for legacy/LoD versus D2R
   item encoding.
13. Port D2R item-list navigation from Horadric Tools in read-only form before
   attempting any item write support.
14. Add scanner-style validation helpers for D2R save invariants: checksum,
   size, stat terminator, item counts, follower count/payload length, and
   Warlock follower payload size.
15. Start native map generation with a narrow area family after fixture coverage
   exists for external generated-map imports.
16. Keep `ARCHITECTURE.md` and `REPORT.md` updated as each component becomes
   real implementation.

## Resource Comparison

- Blizzhackers `Diablo2PacketsData` is useful for packet sizes and legacy field
  names, especially fixed structures like `D2GS_GAMEFLAGS`, `D2GS_LOADACT`, and
  `D2GS_PLAYERMOVE`.
- MephisTools `diablo2-protocol` is useful as a machine-readable schema and is
  MIT licensed. Its protocol data agrees with Blizzhackers for the initial
  fixed-size packet subset, though it sometimes splits fields differently, such
  as `D2GS_GAMEFLAGS`.
- blacha/diablo2 is the best fit for live network-derived memory state. Its
  `Diablo2State` keeps maps of players, units, objects, and items and updates
  them from parsed packet events.
- blacha/diablo2 `packages/mpq` is also a strong fit for classic static-data
  access. The MPQ package is compact and idiomatic for its TypeScript context:
  version-1 archive headers, hash/block table decryption, path hashing with
  `/` and `\` normalization, file-name-only decryption keys, sector extraction,
  uncompressed file support, encrypted file support, and PKWARE implode
  decompression. The Rust port should preserve its vectors and layering while
  avoiding a direct dependency on Node-style buffers or its `implode-decoder`
  package.
- blacha/diablo2 `packages/map` is most useful as a generated-map contract and
  operational reference, not as code to port wholesale. The reusable pieces are
  the JSON level shape, alternating filled/open collision RLE, unsigned seed
  handling, act-from-level ranges, object/exit/npc metadata, MPQ-backed name
  enrichment, and good-exit rules. The C client hooks, Wine process management,
  Express server, LRU process cache, and canvas rendering should stay outside
  this Rust core crate.
- OpenD2 is useful as an independent packet-layout cross-check, especially for
  NPC movement and assignment comments in `Shared/D2Packets.hpp`.
- Kolbot/D2BS is useful for unit semantics, item events, stat meanings, and
  automation behavior, but its internal memory model is tied to the injected
  scripting runtime and is less directly portable for this external packet-only
  library.
- For now, native Rust parsing is a better fit than importing a parser library:
  the packets implemented here are fixed-size, little-endian, and can be parsed
  without allocation or macro-generated code.
- dschu012/d2s is the most useful save-file implementation to port
  structurally. It clearly separates header, item, and stash sections, handles
  D2R-specific name/item/stash differences, and uses the same checksum
  algorithm cross-checked by d2s-format.
- Vitalick/go-d2editor is a compact independent check for core header offsets,
  status flags, checksum repair, and write flow.
- krisives/d2s-format is useful documentation for item placement values and
  field names, but the TypeScript parser from dschu012/d2s is a better
  implementation source for Rust porting because it covers more modern D2R
  branching.
- crabsmadethis/d2r-horadric-tools is now the best D2R/RotW-specific resource.
  It documents save version 105, D2R header offsets, mercenary header overlap,
  `gf` stats, `if` skills, JM item lists, Iron Golem blocks, Warlock follower
  blocks, scanner safety rules, and D2R item encoding. Its code is Python and
  includes generated game-data modules, so Rust ports should stay selective:
  prefer stable binary layout, scanners, and small parser/writer algorithms
  over importing its whole data-mod pipeline.

## Verification

```text
cargo test
```

Result: passed. 62 tests.

```text
cargo fmt --check
```

Result: passed.

```text
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Result: passed.

```text
git diff --check
```

Result: passed.
