| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-exemplar-contracts4 (answer EX46-EX50, data-formats §4.4/§8) | `re-exemplar-contracts4` / (RE spec lane) | in progress | answer EX46-EX50 from native bytes |

## Handoff

- Goal: answer EX46-EX50 (docs/specs/questions/data-formats.md) from exact native bytes/callers; write rules into
  data-formats §4.4 / §8; move answered entries; `python tools/questions.py index`.
- Done: lane file created (72e48530). REA: no app open -> static fallback (re_query + decomp + build-runs/vt.py).
- Next: check holder 17/18 removal (0x67c8c0/0x67c9b0), holder 22 (0x67d9a0 / cohort 0x67d940), cohort 0x67dae0,
  then write spec §4.4/§8 + move EX46-EX50 to answered.
- Facts found (all static): exemplar/cohort slots 3,4 = 0x5DEB40 return false (1 arg), slot 5 = 0x4E13E0 returns 0.
  Factory slot 8 0x93FF03 = slot3(a,b,c,0,0), no direct callers. Factory slot 6 0x9754E8 Write(res,record) QI
  0xC56B8F08 -> slot 11 SerialWrite; manager slot 13 0x972C24 save path. SerialWrite: type must be 0x6534284A;
  write mode byte +0x10 (slot 12 SetWriteMode) 0=binary sig DAT_00AAF390 8 bytes, parent key (slot9 parent ->
  slot11 key), count u32 of OWN list (+0x24/+0x28), per prop: id, kind byte 0 + variant (record +0xC8) or kind 1
  + key when prop QIs 0xE52184AB; 1=text "EQZT1###\r\n", ParentCohort, PropCount, names.
  Exemplar slot 14 0x67E5B0 = DebugInstanceTag(cIGZString&) -> true. Slot 15 0x67DDC0 = rebuild(vector* or null
  -> own +0x34): parent(+0x30) slot9 holder slot4 GetPropertyList, Count/PopFront(0x452160EC), merge own.
  SetCohort 0x67C510 (cohort,bool): same ptr no-op; AddRef new, Release old, slot15(null); bool -> post 0xC53D10AA.
  Cohort SetParent 0x67C430 posts 0x853D11F1. Poster 0x67C380: GZCOM CreateInstance(0x46DF45E7,0x65297976),
  SetType, SetData1 key+4, SetData2 key+8, SetExtra(object) slot 36, MessagePost(msg,false) server 0x4FA845B.
  AddProperty 0x67DCE0 posts 0xC53D10AA likewise. SerialRead (both) and editor assignCohort pass false.
  GetCohort/GetParent 0x6AAC50 = mov eax,[ecx+0x30] borrowed.
  List: 12-byte, vtable 0xACE650 (19 slots), ctor 0x90DC0F only direct (6 callers), QI IIDs 0x12E and 1; slot 3
  Count, 4 GetAt(i,iid,out), 5/17 enumerate(obj slot3(elem,a,b)), 6 PushFront(list), 7 PushFront(elem), 8
  PushBack(list), 9 PushBack(elem), 10/11 front/back QI, 12/13 PopFront/PopBack(iid,out), 14 Clear, 15 assign,
  16 Clone, 18 dtor. GetPropertyList uses slot 7 -> list front-to-back is DESCENDING id.
  GetPropertyList callers (decomp scan build-runs/gpl.py): occupant EnumProperties 0x5FEE10/0x5FEF00, exemplar
  and cohort rebuild, cSCExemplarResourceView (editor, created from cSCResourceCatalogView) only.
