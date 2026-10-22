| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-final-gaps4 (RE spec: audio-ambience AA4/AA10/AA11) | `re-audio-final-gaps4` / (RE lane) | **ready** (docs only, no engine/ABI change): AA4 and AA11 answered (MATCHES) with anchors and vectors; AA10 native null behaviour recorded, policy left open | AA10 needs a coordinator/user policy decision; implementers add the §3.2 AA11 vectors as tests |

## Report
- Evidence: static only (re_query.py asm/callers/func, re/names_windows.tsv symport, PE vtable read); REA not used.
- **AA11 (§3.2), MATCHES:** in `HandleMoveOccupant` `0x004E8D40` the failing `0x6A36AFAC` test (`0x004E8D95`) and the
  failing sound-system test (`0x004E8D9C`) both jump to `0x004E8DDE`, the `0xCA19D7CA` read; `0x004E8C70` (sole
  caller this handler) checks `+0x20` itself (`0x004E8C76`), returns 1 without posting when absent. Only read
  success counts (value 0 still enters). Vectors table added to §3.2: A only 1 post, B only 1, both 2 (A first),
  neither 0, no sound system 0.
- **AA4 (§3.1), MATCHES (vehicle):** `0x005628F0` is the `cSC4Vehicle` base ctor (vtable `0x00A9D488`, QI
  `0x0055B660` = cSC4Vehicle::QueryInterface by Mac symport score 1.0, answers IID `0x896E714A`); writes
  `0xC7CC75F1` at `+0x28` (`0x005629D2`); its ten callers are vehicle-subclass ctors (trailer, train car, ...)
  that do not rewrite `+0x28`.
- **AA10 (§3.6), open:** native fact written to §3.6 and the question: AddRef `o` (`0x004EA6D5`), null test
  `0x004EA6DE` skips only the clock AddRef, `0x004EA6E9` dereferences null (access violation at 0); callers
  `0x004EA8A0/8CB/9F2/A1D` never test the clock. Replacement behaviour is a product decision, not decided here.
- Files: docs/specs/audio-ambience.md (§3.1, §3.2, §3.6, question table), questions/audio-ambience.md,
  questions/answered/audio-ambience.md, docs/specs/QUESTIONS.md (regenerated).
