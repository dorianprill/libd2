NEVER MODIFY THIS FILE

# A Diablo 2 library for all game modes: Classic, Lord of Destruction, Resurrected, and Reign of the Warlock 


This library provides a set of tools and utilities for interacting with the Diablo 2 game server.  
The immediate goal is to be able to initialize the entire diablo 2 game state from network traffic between the original client and server and use this implement an external quality-of-life tool (no interference with the client's memory) with the following functionality:

- Map generation from game seed (for later overlay visualization in another tool that uses this library)
- Downloading of battle.net character files for offline play
- Offline character file editing (stats, inventory, etc.)
- Notifications for in-game events (i.e. off-screen item drops)
- Pathfinding resolution on the in game map (for later use in an overlay visualization tool)
- Personalization of automation of in-game actions (i.e. auto-picking up items, auto-selling items, auto-crafting etc.)

## Resources

As the base for this game is quite old, there have been numerous efforts to reverse engineer the game and its network protocol. Below are some of the resources that have been helpful in understanding the game and its network traffic:

- https://github.com/blizzhackers/kolbot (Data structures and game mechanics)
- https://github.com/blizzhackers/kolbot-SoloPlay (Solo play strategy  implementation)
- https://github.com/noah-/d2bs the diablo 2 botting system (awesome!)
- https://github.com/blacha/diablo2  (Network traffic interception and parsing and visualization of game state)
- https://github.com/OpenDiablo2/OpenDiablo2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED)
- https://github.com/eezstreet/OpenD2 (Reverse engineering of game mechanics and data structures, as well as implementation of a custom game client, ARCHIVED?)
- https://github.com/nokka/d2s (character file parsing for slashdiablo's armory)
- https://github.com/krisives/d2s-format (D2 character file format specification)
- https://github.com/dschu012/D2SLib Savefile support for 1.10 (LoD) to 1.15 (d2r)
- https://github.com/emmericp/diablo2-maps (Map generation from game seed and reverse engineering of seed from map)
- https://github.com/Vitalick/go-d2editor (Character file parsing and generation, savegame load/save support, tests)
- https://github.com/crabsmadethis/d2r-horadric-tools (character loading/saving, character generation from descriptions (we can use this for testing?), MCP functionality (we dont need mcp) but we can use the character file parsing and generation)


## Planning and Architecture

In order to manage such a big project in your context window, you are to write and maintain a file ARCHITECTURE.md that outlines the overall architecture of the library, including the main components and their interactions. This file should be updated regularly as the project progresses and new components are added or existing ones are modified. It should also include diagrams and explanations of the data flow and control flow within the library, as well as any design patterns or architectural principles that are being followed. This will help ensure that the development process is organized and that the overall structure of the library is clear and well-documented. It will also serve as a reference for future development and maintenance of the library, as well as for any other developers who may work on the project in the future.


## Implementation 

The library must be written in up-to-date and idiomatic Rust and should be designed with modularity and extensibility in mind. It should be structured in a way that allows for easy integration with other tools and libraries in the future. The library should also be well-documented, with clear explanations of the functionality of each component and how to use it.

For the first phase of the project the focus will be on the implementation of the core functionality, which includes network traffic interception and game state management. This will involve reverse engineering the network protocol used by the Diablo 2 client and server, as well as designing and implementing a system for maintaining an up-to-date representation of the game state based on the intercepted traffic. This functionality will to a very large part already be present in the resources mentioned above, but it will need to be adapted and extended to fit the specific requirements of this library. 
It doesnt need to binary-compatible with the original client or the resources, but it should keep performance on modern 64 bit cpus in mind, while writing idiotmatic, reusable and maintainable rust code.

The implementation of this library is structured around the following components:
- **Network Traffic Interception**: This component is responsible for intercepting and parsing the network traffic between the Diablo 2 client and server. It will extract relevant information such as game state, player actions, and item drops. Note: the package decoding is mostly done and works (tested) but the parsing of packets is still a work in progress. The current implementation is focused on the classic version of the game, but it will be extended to support Lord of Destruction, Resurrected, and Reign of the Warlock in the future.
- **Game State Management**: This component will maintain an up-to-date representation of the game state based on the intercepted network traffic. It will handle the initialization of the game state and update it in real-time as new information is received.
- **Quality-of-Life Tools**: This component will provide various tools and utilities for enhancing the player's experience. This includes functions that will later be needed to implement map visualization, character file management, and event notifications in another tool/repository that will use this library as a dependency.
- **Testing and Validation**: This component will ensure that the library functions correctly and reliably. It will include unit tests for individual functions and integration tests to validate the overall functionality of the library. Extensive testing is paramount to ensure that the library works correctly and reliably.

## Process and Reporting

You are to work on this project in a structured and organized manner, following best practices for software development. You should provide regular updates on your progress, including any challenges or obstacles you encounter and how you plan to address them. You should also document your code thoroughly, including comments and explanations for complex logic and algorithms. 
To follow along between independent coding sessions, you are to write a `REPORT.md` file yourself that summarizes the work done up to now (cumulative), the challenges faced, and the next steps for the project. You may also do online search for relevant resources that i missed and add these to the report. This will help maintain continuity and provide a clear record of the development process.