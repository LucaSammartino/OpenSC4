| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelinst-mi14 (model-instance owner closure) | Codex / `re-modelinst-mi14` | in progress | Locate MI14 owner closure rows and verify original behavior |

## Handoff
- Goal: answer MI14 owner closure questions in the model-instances spec with conformance cases.
- Done: native getter, fixture, cache, animation and processor bindings added to spec §§1.3, 2.2, 3.2, 4.2 (this milestone commit).
- Next: publish sprite resource layout, live discovery/observation boundary, conformance cases; archive MI14 and run questions/doccheck.
- Facts: MI14 located in glue-modelinst-shadow11 worktree's questions file; that lane's current report has no Owner closure/open: table. Its per-owner comparison boundary was read instead.
- Constructor 0x00802950 requires a stack construction flag (standalone=1), RET 4; previous receiver-only assumption is wrong. Destructor body 0x00802620 is receiver-only.
- GetModelsInVolume 0x005BE2E0 returns false without outputs. Network 0x0061AF90 mutates transform, returns one counted pointer without capacity check; no universal enumeration limit.
- q.py Bash wrapper fails with Win32 error 5; use native PowerShell and capped output. Export/assembly queries work; REA was not needed.
- Edit only spec/question/lane docs; never read engine/. No ABI or game runs expected.
