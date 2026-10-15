# Spec inventory: game classes without a spec (ranked)

Status: **planning document**, clean-room input. It lists the simulator, manager and layer classes of the original
program that `docs/specs/` does not yet describe. It ranks them by how much a replacement or the harness needs them.
It holds no behaviour. Each row names the class, its class ID (CLSID), its role, a size estimate and what depends on
it. Written 2026-10-15 by the re-specs12 lane.

**How the size is measured.** The size is the byte count of the class's exported pseudocode file in the analysis
workspace. It is a rough guide to the reading effort (about 1 KB for a trivial class, 100 KB or more for a large one).
It is not the size of a port. "Code" means the summed function body bytes from the analysis export, where no
pseudocode file carries the class name.

**What already has a spec.** All 53 saved city components (foundations §3.6) have a spec except those named below,
as do the demolition utility, the dispatch manager and the region deal flow (neighbors §9, foundations §11).
save-records.md §3.2 lists the saved classes that are still original: the automata script system, the controller
manager, the network manager, the occupant manager, the terrain, the four automata managers and the tutorial.

## 1. Ranking

| Rank | Class | CLSID | Role | Size | What depends on it | Spec |
|---|---|---|---|---|---|---|
| 1 | `cSC4DisasterLayer` and its seven managers (fire, riot, earthquake, pipe burst, toxic spill, volcano, effect disasters) | layer `0x61F6ABF5`; managers in disasters.md §1 | Starts, runs and ends disasters. Any active disaster puts the simulator in an **emergency pause** and **disables saving**. Fire is the one that occurs by itself. | layer 40 KB; fire 81 + 12 + fire occupant 25 KB; riot 35 + 7 KB; earthquake 28 KB; pipe burst 22 + 7 KB; toxic 8 + 6 KB; volcano 16 + 4 KB (+ lava 59 KB); effect 3 + 8 KB | Flammability (spontaneous fires), crime (arson), aura (riots), plumbing (pipe bursts), budget (earthquakes), advisors (dispatch effectiveness, disaster cost), MySims, the fire truck, history (`0x49D15B74`, `0xAA8C87C7`), the 3D view (`GoToNextDisaster`). The harness: a spontaneous fire pauses the simulator in an unattended run. | **[disasters.md](disasters.md)** (this lane) |
| 2 | Automata layer: `cSC4AutomataManager`, `cSC4AutomataControllerManager`, `cSC4AutomataScriptSystem`, `cSC4AutomataGenerator`, `cSC4AutomataController`, `cSC4AutomataAttractor`, `cSC4Automaton`, and the vehicle, pedestrian, aircraft and watercraft managers | script system `0x4A1B664E`; controller manager `0xCA14CA03`; generator `0x4A1628C5`; attractor `0x6A1628BA`; vehicle / pedestrian / aircraft / watercraft managers `0xA98F9D3B` / `0xC98F9D17` / `0xC98F9D4A` / `0xA98F9D28` | Cars, people, planes and boats. They are decoration plus a few game hooks: riot leaders, fire crews, MySim cars, U-Drive-It, the "fire_occupant" controller. | manager 71 KB; controller manager 90 KB; script system 156 KB; generator 29 KB; controller 49 KB; attractor 8 KB; automaton 86 KB; vehicle 112 KB; watercraft 127 + 102 KB; aircraft 6 + 8 KB; pedestrian 8 + 4 KB; traffic automata 8 KB | Six saved components (40, 41, 43–46). Riots (leaders are pedestrian automata, disasters.md §6), dispatch (fire and police crews), city situation (U-Drive-It), the fire occupant (`fire_occupant` controller). Lua (`lua-host.md`) calls the script system. | none. Recommended next lane: a short-form spec of the saved records and the game hooks, not the animation. |
| 3 | `cSC4NetworkManager` and `cSC4NetworkLotManager` | `0xC990BD46`; `0xE9A6CD3B` | Network occupants (roads, rails, pipes, power lines): placement, the network flags, intersections, and the lots that networks need (toll booths, stations). | 76 KB; 98 KB | Traffic network (traffic-network.md reads the occupants), plumbing (pipe flags `0x10000` / `0x20000`), power, zones (road access), the riot (`MarkRiotNetworkOccupants`), demolition (`UpdateNetworks`). Saved component 53. | **[network-manager.md](network-manager.md)** (re-specs13c) |
| 4 | Occupant manager | `0x098F964D` | The spatial index of every occupant: insert, remove, cell queries with filters, the inserted / removed messages `0x99EF1142` / `0x99EF1143`. | unnamed in the Windows export: about 89 functions, 16 KB of code near `0x0049D910`–`0x004A05B0` | Every simulator (all queries go through it), the disaster layer (cost per removed occupant), demolition, the fire (`GetOccupantsInCellRect`), toxic spills (insert / remove). Saved component 4. | **[occupant-manager.md](occupant-manager.md)** (re-specs13b) |
| 5 | Terrain: `cSTETerrain`, `cSTESurfaceWater`, `cSTEVolcanoLava` | terrain `0xE98F9525` | Heights, brushes (`ApplyBrush`), water, the volcano's lava flow. | 337 KB; 82 KB; 59 KB | Earthquake and volcano (brushes, disasters.md §7, §10), pipe-burst sinkholes, flora, landfill, foundations (altitude), plumbing (water tables), the region (`ReconcileEdgesWithNeighboringCities`). Saved component 49. | partly: data-formats.md (the record), graphics-formats.md (textures). Behaviour is unspecified. Large; split it (heights and brushes first, water second). |
| 6 | `cSC4PropManager` and `cSC4LotConfigurationManager` | `0xA990BD10`; `0x299062C0` | Prop families and timed props; the lot configuration exemplars and their lookup. | 77 KB; 78 KB | Prop developer, lot developer, building development, flora, zones. Saved components 5 and 9. | partly (prop-developer.md, lot-developer.md read them). A short-form spec of each would close the gap. |
| 7 | `cSC4EffectsManager` | (effects, not a city component) | Visual effects and their actions; some actions call the game: `IgniteCellRadius` and `SetCause(4)` (`0x005BCBF0`, `0x005BCC80`), toxic spills (`0x005BD3C0`). | 147 KB | The effect disasters (meteors, UFOs, robot, tornado, Truckasaurus; disasters.md §11), the earthquake and volcano visuals. | none. Only the game actions matter for the simulation. |
| 8 | `cSC4TutorialSystem` | `0xAA3836FB` | Tutorial scripts; slot 8 is the "no burn-down" flag that the fire reads. | 24 KB | Fire (disasters.md §5.8), advisors. Saved component 48. | none. Short. |
| 9 | `cSC4NewsTickerWinProc` | (UI) | The news ticker: it reads advice and opinion messages. | 47 KB | UI only. | none; not needed for the simulation. |
| 10 | `cSC4Nation` | (region) | The region list and the regional economy modifier (`GetEconomyModifier`). | 42 KB | Neighbours, demand. | partly (neighbors.md, foundations §11 cover the region and its records). |
| 11 | `cSC4GameOfLifeSimulator` | (easter egg) | A cellular-automaton easter egg. | 3 KB | nothing | not needed. |

## 2. Why this order

1. **Disasters first.** They are the only unspecified component that changes the simulator's run state. A fire
   lit by the flammability simulator (flammability.md) calls `DisasterHasBegun`. That puts the simulator in an
   emergency pause (simulator slot 11) and disables saving (city slot 104) until the fire ends. The fire runs on
   the wall clock, not on game days. Arson (crime.md), riots (aura.md), pipe bursts (plumbing.md) and earthquakes
   (budget.md) enter through the same layer. Five existing specs call into it without describing what happens
   next.
2. **The automata layer** owns six saved components and the riot, fire-crew and U-Drive-It hooks. It is big, but
   most of it is animation that a replacement may draw differently. A short form that covers the records and the
   hooks is enough.
3. **The network manager** and **4. the occupant manager** are depended on by nearly every simulator. They are
   still the original classes (save-records.md §3.2), so a replacement of either must match its message order
   exactly. The occupant manager is small; specify it before the network manager if only one lane is free.
4. **Terrain** is the largest remaining class. Only the parts that the simulation reads (heights, brushes, water)
   are needed.
5. The rest are either partly covered by consumer specs (prop, lot configuration, Nation) or are UI and easter
   eggs.

## 3. Parallel lanes (suggested)

These can run at the same time. None depends on another's text:

- **re-specs13a: automata (short form).** The records of components 40, 41 and 43–46, the controller-manager
  calls that other specs already use (city slot 44 slot 15 "create automaton"; city slot 47 slots 5, 9 and 14),
  and the riot and fire hooks.
- **re-specs13b: occupant manager.** Every slot (insert, remove, cell and rect queries, filters), the two
  messages and their order, and the record.
- **re-specs13c: network manager and network lot manager.** The network flags (`0x10000` distressed, `0x20000`
  burst, `2` riot-blocked), `UpdateNetworks`, and the record of component 53.
