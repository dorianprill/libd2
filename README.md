# A Diablo II Core and Client Library 

A Diablo II library for core and simple client functionality, written in Rust for performance, safety and re-usability without any runtime requirements.


This effort is very much WIP, so it is not available on crates.io yet.  

If you are interested in Diablo II and/or Rust, this might be fun!

## Long Term Goal

In the long term, the library aims to make it possible to write a headless client that can connect to the game servers and interact with the game world.
An immediate use case is a game helper tool that visualizes the game state in real time (e.g. a map overlay, item and buff tracking, character reading for inspection, editing and download from bnet, etc. ) along with some QoL-functionality.


## Feature List

1. network protocol support
   - [x] D2GS (plain)  
   - [x] D2GS (compressed)  
   - [ ] BNCS  (It if is used in D2R at all)
   - [ ] MCP Realm Logon
2. Game State Data Structures.  
   - [ ] Items and Buffers (Ground, Inventory, Stash, Cube, Belt)
   - [x] Players (WIP)
   - [ ] Players Quest Progression  
   - [x] NPCs (WIP)
   - [ ] Party/Hostile
   - [ ] Game Quest Progression
   - [ ] Maps (generate from game seed, take from d2bs)
   - [ ] Pathing(?)
3. Client object
   - [x] shadow client (packet listener w/ game update loop WIP)
   - [ ] active client / protocol state machine

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

One simple use case that is supported now is launching a shadow client to sniff d2gs packets.  
Put the following code in your main.rs and run it. Then start up your D2 or D2R game client, join a game and watch the game packets flow.

```Rust
use libd2r::Client;

fn main() {
    let mut client = Client::new();
    client.start()
}
```

Please note that currently it does not fill any internal game data structures (state update handling is still WIP). It will just filter, decode and print packets. Also, make sure to not have multiple game clients running as currently their packages will be indistinguishable in the output.

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
