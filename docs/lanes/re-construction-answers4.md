| re-construction-answers4 (OP13-OP16 + OP7 slot23 answers, lot-records §9 / lot-developer §6.7) | `re-construction-answers4` / spec lane | in progress | answer OP13-OP16, OP7 |

## Handoff
- Goal: answer OP13-OP16 and OP7 (Init view slot 0x5C/0x5007) in lot-records §9 / lot-developer §6.7 from the exe.
- Done: lr §9.1 table rewritten in object frame (3 this-frames: obj/+4 DoMessage/+8 occupant/+0xC Write-Read), §9.2 Init/DoMessage/refresh/SetLot crane (commit 1).
- Next: ld §6.7.1 Init (base Init, 4 subs, AddOccupantGroup 0x5007), §6.7.6 AnimateCrane (k=1/(2pi) f32 at 0x00AA2098, mult = sfCraneTimeOfFullRotation global 0x00B0A7EC, limits from AttemptAddOuterCrane c±asin(d/R)), §6.7.7 sparks extent = max(x,z span) of scaffold box obj+0xC0 (no y); then move OP7,OP13-16 to answered + questions.py index.
- Facts: obj = Write-coded + 0xC; DoMessage-coded + 4; occupant-coded + 8. OP7 slot23 = 0x005E9350 body proved (append to prop 0xAA1DD396, post 0xEBFF18A3 if new). Tick ms = msg dword +0x14 /1000.
