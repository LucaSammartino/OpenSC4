| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-application4: application framework spec (cSC4App / directors / preferences) | `re-application4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- goal: write docs/specs/application.md for the largest concrete owner family of the uncovered
  application subsystem (184210 bytes): lifecycle/startup/shutdown/service ownership/messages.
- done: lane file 80b50d1f. Sized owners (coverage.py --owners): cSC4App 86902 B/322 fn (largest),
  cSC4AutomataCOMDirector 41602, cGZToolsCOMDirector 31303, cSC4GameDataRegistry 5807, cWinSC4App 2224.
- next: write docs/specs/application.md for cSC4App (layout, vtables, Init/PreFWInit/AddSystemServices/
  PostFWInit/Shutdown order, DoMessage tables, directories, startup resolution).
- facts: REA unavailable ("No app is open") -> asm via re_query only. Helpers in build-runs/: dump.sh,
  asmlist.sh, filt.sh, cstr.py (PE string/dword reader). Object layout: +0 primary vt 00a86c2c (15 slots),
  +4 msgtarget2 00a86c1c (DoMessage 00451ed0), +8 00a86c0c, +0xC cISC4App 00a86af0 (71 slots),
  +0x10 cGZApp 00a86abc (Init/Shutdown +0x20/+0x24), +0x14 00a86aa4, +0x2C 00a86a68 (Pre/PostFWInit).
  App msgs 0x28E9397A..D posted by Init/Shutdown via PostSC4AppMessage(type, post?).
