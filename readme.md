# Diablo II Resurrected Library

A Diablo II Resurrected library for core and simple client functionality, written in Rust for performance, safety and re-usability.

This effort is very much WIP, so it is not available on crates.io yet.  

If you are interested in Diablo II and/or Rust, this might be fun!

## Goals

- cross platform without runtime/VM requirement  
- provide game internal data structures and methods
- savegame load/save support (for e.g. character editors, armory-style webpages)
- implement the network protocol as fully supported as possible (i.e. however much is known publically)
- reproduce game state as accurately as possible/needed

## Milestones

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
At this early stage I haven't created any bindings, but Python/JS would be useful to many people I guess.
### Linux 
Tested with Diablo 2 (Vanilla) and WINE, not tested with D2R yet because I don't have a vulkan capable machine
`cargo build --release`
### Mac Os
`cargo build --release` (not tested yet)
### Windows
You will need to install npcap and additionally download the WinPcap Developers Pack as per the [libpnet](https://github.com/libpnet/libpnet) build instructions for Windows. Then point your user environment variable `LIB` (create if nonexistent) to the folder where to find Packet.lib i.e. `WpdPack/Lib/x64/` from the WinPcap Developers Pack you just downloaded. Then `cargo build --release`
This will get the project building but the executable panics while querying the available network interfaces.  
Turns out windows is very fickle and I might need to pull in the `windows` crate just to find the active interface.

## Usage

One simple use case that is supported now is launching a listening client/connection to sniff d2gs packets:

```Rust
use libd2r::Client;

fn main() {
    let mut client = Client::new();
    client.start()
}
```

Please note that it does not fill any internal game data structures as of now (state update handling is still WIP). It will just filter, decode and print packets.

## Contributing

This is quite the challenge so any help is appreciated!  
There is quite a bit of awesome code out there, but scattered across various sources.  
Maybe you want to help translating this [d2r savegame utility from go](https://github.com/Vitalick/go-d2editor)?  
Or help with the map generation from game seed? It exists in several projects (d2bs, opendiablo, opend2, ...) and shouldn't have changed for D2R.

## Disclaimer

Little of this works yet and most probably never will. Unless you feel like contributing, which is welcome, it is mostly an exercise.  
Here are some great resources on the original game:

- [client-less C# bot by dkuwahara](https://github.com/dkuwahara/OmegaBot)
- a [blog post by Eric Carmichael](http://www.ericcarmichael.com/my-diablo-2-botting-phase.html)  
- and, of course, [D2BS](https://github.com/noah-/d2bs)
- Another good resource is the [diablo 2 protocol js library](https://github.com/MephisTools/diablo2-protocol).
