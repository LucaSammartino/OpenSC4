# Lane: re-netocc-contracts4

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-contracts4 (answer NO15-NO17 network occupants) | `re-netocc-contracts4` / `.claude/worktrees/re-netocc-contracts4` (docs lane) | in progress | answer NO15-NO17 |

## Handoff

- Goal: answer NO15 (S3DTransform stream), NO16 (base record fields), NO17 (underground record/heights) in
  docs/specs/network-occupants.md from the original program; move to answered/, run questions.py index.
- Done: lane file.
- Next: write NO15/NO16 answers into §2.2/§2.3; then NO17 (underground Write 0x0065B140 / Read 0x0065E510).
- Facts: Write/Read get this = object+8 (vftable_2 0x00AA7FDC, no thunk): every record offset is view-relative,
  object = view+8 (box object+0x124..+0x138, confirmed by GetBoundingBox on the +4 view). Stream slots: 0x14 u8,
  0x1C u16, 0x24 u32, 0x28 s64, 0x30 f32, 0x48 GetError. Quads = 4 vertices of 0x18 {x,y,z,RGBA,u,v} at object+0x9C.
  Transform object+0x64 {u8 counter, u8 mask, 9 f32 +4, 3 f32 +0x28, f32 scale +0x34}. Legacy = 9+3+1 f32 + u32 (56 B).
  Read tail: +0x164(obj)!=0 && v<7 reads 2 more floats -> cell -> slot 72 (prop 0x4835C6AB = z<<16+x); v<8 -> slot 7(1).
  Scripts: build-runs/dump.sh ADDR OUT (full asm), build-runs/peek.py HEXADDR (exe data).
