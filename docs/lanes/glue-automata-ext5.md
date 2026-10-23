| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ext5: remaining automata owner shadows | Codex / glue-automata-ext5 | in progress | Audit documented observation contracts, extend shadow where specified, verify through 2032. |

## Handoff
- Goal: compare AutomataManager, Automaton, TrafficAutomataManager, AutomataController and ConstructionOccupant state at load/month.
- Done: initial handoff 98cb12128; audited automata §§0,3.5,4,5.1,6.1,11, manager §2.1, construction lot-records §9.
- Implemented default-off automata_owners_shadow: inherited manager records, recursive automaton/controller records, indexed construction records via occupant-manager Write references.
- Added one C export for construction record roundtrip, with tracked reference cleanup and explicit same-input live crane presence seeding (not a lifecycle comparison).
- Focused test passes 1/1 (crane/scalars/refs preserved; failed reads release references); first fake-stream test failed because fake error callback never latched failure, corrected test to mimic CaptureIn.
- Next: final fmt and full qbuild all, then queued qbuild verify if required, then unique-label game run through 2032.
- No game requests started. ABI exports changed; ABI version unchanged.
- q.py cannot start Git Bash in sandbox (CreateFileMapping denied); direct native cargo works. fmt needs absolute filenames.
- No guessed native signatures. Unknown referenced automaton/controller classes fail rather than being silently skipped.
