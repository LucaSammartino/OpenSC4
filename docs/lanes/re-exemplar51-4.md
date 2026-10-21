| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-exemplar51-4: answered EX51 a-f in data-formats §8.2 (slot 14 cohort text and GetValString name, holder add slots 10-16, rebuild dest layout, void returns, SerialWrite bytes); EX50 left open with an exact runtime follow-up | Claude lane, .claude/worktrees/re-exemplar51-4 | ready (docs only, static RE) | glue: fix add13 (own list only), void returns, cohort tag text, GetValString name, non-null dest, RemoveAll post rule, binary SerialWrite; game run gex51-r1-union-callers for EX50 |

## Report

Static RE only (re_query.py export and asm, raw vtable reads of bin/SimCity4.exe with build-runs/vt.py); REA was not needed.

What changed (docs/specs/data-formats.md §8.2):
- Slot 14 DebugInstanceTag (EX51 e/f): the cohort appends " Exemplar Cohort " (0x00AAF2DC), the exemplar " Exemplar ". Both look up 0x10/0x20 in the effective list through holder slot 9. The type test reads the raw first dword (folded AsX). The name goes through GetValString (variant slot 55, 0x00917ADB, §5.1 converting), so Uint32 5 gives '5'. Worked example included.
- Slot 15 rebuild with a non-null dest (EX51 a): a {begin,end,capacity} vector of 8-byte {id, property*} elements, ascending, one reference per element. A non-null dest is rebuilt the same way as the null case, the object's own effective list is not touched, and the slot returns void.
- Holder add slots (EX51 b): slots 10-13 are the base bodies. They ignore the bool and never post. They set an existing own property in place, but a new ID goes into the own list only, so the effective list misses it until a rebuild (this matches native add13_has=0). Slots 14 and 16 call slot 15 virtually (0x0067DCE0 / 0x0067DA00): both lists are updated and they post when the bool is true. A table and a worked example are in the spec.
- Returns (EX51 c): SetCohort (RET 8) and RemoveAllProperties (RET 4) return nothing, which explains the 0/108/208 values the contract test read. Correction: RemoveAll posts only when the own list was non-empty before the call AND the bool is true (0x0067C9C4).
- SerialWrite binary form (EX51 d): signature "EQZB1###" (45 51 5A 42 31 23 23 23) or "CQZB1###". Then record slot 49 SetFieldResKey(parent, or zeros), slot 37 count, and per own property: slot 37 id, slot 29 kind byte, then slot 50 SetFieldVariant (tag+value, no marker) or slot 49 key. A byte-level worked example is in the spec.
- Questions: EX51 moved to answered/data-formats.md (DIFFERS). EX50 got an update: static evidence is insufficient (EnumProperties has 0 direct callers and 79 vtable refs, so every call is indirect). The exact follow-up is run gex51-r1-union-callers (gex4-r5-union-c enable set + the contractfix4 return-address log, then a per-module split and re_query func on game addresses). QUESTIONS.md was regenerated.

The current contract test (current3/4) expectations that the spec contradicts: non-null dest refused, slots 10-13 updating the effective list, no-op SetCohort/RemoveAll returning true, cohort tag text, and a string-only name getter.

Next: a glue lane aligns the replacement with the rules above. Then run gex51-r1-union-callers for EX50.
