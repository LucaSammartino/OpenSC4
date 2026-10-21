| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-disasters46-4: answer DS46 (special-building vector at record +0x50) | `re-disasters46-4` / .claude/worktrees/re-disasters46-4 | ready, docs only, no ABI change; DS46 MATCHES (disasters.md §2.6.1) | 1. glue: keep record +0x50 as three null pointers (conforming); no host binding needed. 2. if a UI ever wants the records, fill per §2.6.1 with allocator B only |

## Report

**What changed.** docs/specs/disasters.md new §2.6.1 "The special-building vector"; DS46 moved to
questions/answered/disasters.md (MATCHES); QUESTIONS.md regenerated. No engine, no ABI, no docs/abi edits.

**Findings (static: re_query.py asm/callers/xrefs, decomp/, exe bytes; REA not needed).**
- Vector at instance +0xA0 (record +0x50) = {begin +0xA0, finish +0xA4, end_of_storage +0xA8}, 12 bytes, ends at the
  fire float +0xAC. Count = (finish-begin)>>5 (0x00578940). Storage: allocator B (0x0090CF54 / 0x0090CF63, 0x00B62698).
- Element 0x20 bytes: cRZString 0x14 {vtable 0x00A80810, begin, finish, eos, refcount} at +0; +0x14 padding never
  written; int64 cost at +0x18. Strides at 0x00578B4E, 0x00578756.
- Life cycle: ctor 0x00578A10 zeroes the three pointers (only caller 0x00578B83); DoOnOccupantRemoval 0x00578CD1-0x00578D27
  builds a temp (0x00578390 + cRZString::Copy slot 13 0x0090E20A) and push_back 0x00578B30 (sole caller 0x00578D0C);
  copy-construct 0x005785B0 deep-copies the string, refcount := 0; grow 0x00578930 size+max(size,1);
  destroy-range 0x00578730 (dtor, grow, Reset); Reset 0x00578DC7 erase(begin,finish) keeps the buffer; dtor
  0x0057884B frees begin.
- Readers: none. Safety advisor (0x00416ED1 in 0x004169E0) reads record +0x00/+0x08/+0x60/+0x5C only. Layer not
  saved, so no serialization callers. Write-only store.
- Conformance: three null pointers forever is valid (the native ctor state); DS42-44 identity boundary unchanged.

**Questions.** DS46 answered; none filed.
