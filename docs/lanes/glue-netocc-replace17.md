| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-replace17 (partial: pole + line) | Claude / `glue-netocc-replace17` (engine) | ready partial: verify PASS (CTest 92/92), cargo 7646/7646, clippy 0; netocc-claude-3 clean to 2032, shadow3 73/73 | 1. Remaining eleven classes (see Missing). 2. Pole model/wire drawing (NO33/NO43). |

`enable = netocc` registers cSC4PowerLineOccupant (0xC9C05C5D) and cSC4PowerPoleOccupant (0x09C05C6A).
Shared inherited slots: engine/plugin/FC4OccupantAdapter.h (CRTP over cISC4Occupant, occupant-manager 13.1).
The adapter makes the lazy placeable from the game's pool + ctor (prop-occupant 6.1; compat entries added).
The model maker calls slot 19 unguarded; a null placeable crashed run netocc-claude-1 at 0x0048F388.
cFC4PropertyHolder gained an exemplar parent, so inherited exemplar properties resolve like the game's holder.
Pole: engine/capi/src/netocc_pole.rs over core power_pole.rs; v5 record, connections, StaticInit tuning, 4.7.2 box.
The occupant view sits at primary+8, as model-maker 12.6 expects (o - 8); the test pins this.
Power fix: cFC4Power reads pole tokens' flags and properties through QI 0x26BB767E (power 5.6).
The old cast gave an RTC failure in run netocc-claude-2. Power's RemoveConnection now reaches FC4 poles.
Game: regress netocc-claude-3 (house list + netocc) clean 2032-01-01, ref_warnings=0, power state = train226.
Shadow: netocc-claude-shadow3 (house + netocc + netocc_shadow) 73/73; pole 219 and line 1314 records exact.
SPEC-GAP pole: no own model/wires/floor drawn (NO33/NO43), no powerpolezap effect, no line cells (slot 5/16).
Hazard: the native power-line tool calls non-virtual AddConnection/RemoveConnection on poles (power-line-tool).
On a loaded FC4 pole that is unsafe; the regress run does not draw lines. Do not ship netocc for play yet.
Missing (native): network, prebuilt, bridge, tunnel, underground, pipe, subway and three connection classes.
Missing (native): pylon. These need the 81-112 slot network interface, drawing and UG5/NO36/NO40.
DLL snapshots: build-runs/dll-netocc3 (game runs). Logs: build-runs/qbuild-*.log, fc4verify-*.log.
