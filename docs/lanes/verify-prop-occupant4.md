| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-prop-occupant4: isolated plain prop native verifier | Codex, verify-prop-occupant4 | in progress | Inspect primitive bindings; freeze fixtures and authority contract |

## Handoff
- Goal: first bounded independent plain-prop factory verifier; no owner implementation or coverage promotion.
- Worktree reserved by supervisor; detached HEAD retained.
- Initial q.py failed before running its Bash command (WSL E_ACCESSDENIED); use native PowerShell with capped output.
- No baseline all: task explicitly requires inspection first and one final stable-source all.
- Existing primitives: fc4_prop_occupant_create(host, null types), destroy, orientation i32/set i32(bool), state u32/set u8(void), chance u8/set u8(void), usage u8/set u8(void), timer bitfield u32/state i32/set u8+bool(void), time-range float outs. No new exports needed. Host primitives are isolated constants (not-in-city, no exemplar); forbidden callbacks count failures.
- Frozen contract before comparator: typed published 24-slot interface; SDK's GetState/GetTimerBitfield/GetTimerState signatures require local corrected EAX declarations. Chance/usage return uint8_t AL, orientation setter uint8_t AL; never widen byte codes to bool.
- Supported partial slots: 0 QI primary IID owned identity; 2 owned Release lifecycle; 9/10 orientation; 11/12 unattached repeated byte state; 14 constructor time outs only; 17/18 chance; 19/20/21 byte timer high bits and incomplete low nibble; 22/23 usage. No direct AddRef test (QI implicit AddRef only).
- Excluded slots: 1 direct AddRef, 3 AsOccupant, 4 AABB, 5/6 placement, 7/8 type/global intern, 13 clock setter, 15/16 date/tick. No lot-tag interface expansion, Init, record/state seed, attachment, city insertion, RNG, manager, developer, models/effects/save owner claims.
- Frozen literal fixture header: two independent fresh objects; defaults (orientation0,state0,chance100,usage0,timer0), range0/0; 40 changes to object0 then 5 to object1. Orientation MIN/-1/0/1/2/3/4/MAX/-128; chance0/1/99/100/255; usage0/1/2/3/4/255; state0/1/1/2/255/128/0; timer sequence80,C0,C3,C7,C5,CD,4D,0D,0C,00,F0,00,00 never reaches low F. Object1 changes orientation2/chance1/state255/usage3/timer80. Both entire scalar snapshots checked after each mutation; six timer masks0/1/2/8/80/FF.
- Planned complete-success denominator: 92 scalar snapshots x11 getters=1012; 2 range getters; 45 setters (10 AL orientation results); 2 QI and4 Release=1065 typed dispatches; 2 factory calls separately. Actual refused paths must report actual counts, never planned counts as measured.
- Authority: before every native dispatch, existing 1.1.641 gate, exact published primary table identity and executable-module ownership of dispatch target. Factory endpoint refusal stops the fixture without searching alternatives. Explicit native-preserving providers exclude propmgr. Acquired replacement cleanup is counted separately, never a native comparison.
- Implemented comparator + typed factory adapter, per-dispatch module authority, isolated host, explicit lifecycle and method counts, no new ABI. Header-only local comparator tests exercise 7 injected faults, object aliasing, AL masking/full-EAX rejection, authority refusal, and production release boundary with exact/denied/over-release callbacks.
- Next: commit source; run one final all with before/after source hashes, freeze DLL/PDB, sequential OFF/ON controls. No Rust touched (rustfmt check n/a).
- No game requests. Initial lane commit 90e5e016. Main/tools/private material unread.
