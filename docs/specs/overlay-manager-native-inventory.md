# Overlay native-body disposition (2026-10-22, re-overlays-final4)

Companion to [overlay-manager.md](overlay-manager.md). This is a coverage checklist, not an assertion that
every body is fully specified. **P** = preceding contract retained (no new proof claimed here);
**N** = newly traced primary owner body with rule in the named section; **R** = partially specified owner
body with an exact residual in OV7/OV8; **S** = storage/scaffolding inventory, no independent public contract
or complete exceptional/allocation proof supplied. S entries must not silently become implemented coverage.
Addresses identify native bodies, not implementations or test outputs.

The export has 97 starts in `[0x00735260,0x0073CD40)`. Its `0x0073576A/0x007357FE` starts are DrawRect
tail blocks, and `0x00736559` is within missing-start slot-20 body `0x007364F0`; they are not three additional
logical methods. Exclude folded unrelated `0x00735290` and separately named shared packed-render-state
vector helper `0x0073AE60` (8-byte elements); add the missing `0x007364F0` start. The resulting **93-entry
working set** below includes owner-local template/storage helpers and thunks. It is not a count of 93 public
methods. No inference of full coverage from the brief's 30436-byte size is warranted.

| Body | Disposition / spec or exact limitation |
|---|---|
| `00735260` | P QueryInterface, §1 |
| `007352A0` | P statistics formatting, §4.4 |
| `007352D0` | P position tolerance helper, §3.2 |
| `00735320` | S repeat-copy of 8-byte values; no separate public operation |
| `00735350` | S copy of 9-word records; no separate public operation |
| `00735380` | P finite rectangle rounding, §§2.2/3.1 |
| `00735480` | P quality/key wrapper, §3.5 |
| `007354E0` | N fallback model rectangle, §3.7 |
| `007356D0` | P shadow dispatch, §4.6; consumer arrays remain OV8 |
| `00735720` | P rectangle dispatch including both exported tails, §4.6; consumers remain OV8 |
| `00735820` | S overlay assignment/copy support; full reference/exception schedule not published |
| `007359F0` | S overlay construction/copy support; full reference/exception schedule not published |
| `00735CB0` | S overlay range assignment support used by Shutdown |
| `00735D10` | S 48-byte handle range/storage support; exceptional paths not specified |
| `00735F40` | P default overlay constructor, §1.1 |
| `00736100` | N complete message handler, §5 |
| `00736150` | P factory-slot-6 decal wrapper, §3 |
| `007361B0` | P factory-slot-5 decal wrapper, §3 |
| `00736210` | P handle FIFO free, §2.3 |
| `00736250` | S linked-list erase support; no additional public operation |
| `00736280` | S 48-byte record copy support; exceptional paths not specified |
| `007362E0` | S overlay range-copy support; exceptional paths not specified |
| `00736320` | S overlay repeat-construction support; exceptional paths not specified |
| `00736350` | S 48-byte record range support; exceptional paths not specified |
| `007363D0` | S repeat-copy of 48-byte records |
| `00736440` | S index-list chain copy/allocation support |
| `00736490` | P placement getter, §§2.3/3.2 |
| `007364F0` | P slot 20 including exported fragment 00736559, §9 review note |
| `00736580` | P shown getter, §2.3 |
| `007365E0` | P alpha getter, §2.3 |
| `00736640` | P colour getter/output order, §2.3 |
| `007366C0` | P draw-state-byte getter, §2.3 |
| `00736720` | P self-lighting getter, §2.3 |
| `00736790` | R decal masks/count/dispatch proved; all-entry dot orders OV7 |
| `00736BF0` | R full-shadow control/stages/location/water boundaries §§4.4/4.6/4.8; numerical composition OV7 |
| `00737870` | R rough-shadow selection/count/dispatch/exit §§4.4/4.6/4.8; all-entry dot orders OV7 |
| `00737D40` | P MarkChanged and empty-rectangle behavior, §§2.2/2.3 |
| `00737DB0` | S index-list assignment; no new public semantics |
| `00737E10` | N overlay payload range destruction, §1.2 |
| `00737E50` | P shown setter/group recursion, §2.3 |
| `00737F00` | P alpha acceptance, §3.3 (OV6 preserved) |
| `00738010` | P colour setter/group recursion, §2.3 |
| `007380C0` | P draw-state setter/group recursion, §2.3 |
| `00738160` | P all-entry texture scale, §3.4 |
| `00738230` | P cell-index insertion order, §2.2 |
| `007382E0` | P cell-index removal order, §2.2 |
| `00738390` | P visible collection/order/stamps, §4.5 |
| `00738590` | N single index-list chain destruction, consistent with §1.2 |
| `007385C0` | S overlay vector insertion/storage; no complete exceptional contract |
| `007386A0` | S handle vector insertion/storage; no complete exceptional contract |
| `007387D0` | S index-head range assignment support |
| `00738800` | S index-head range/storage support |
| `00738840` | S index-head range/storage support |
| `00738880` | S index-head range/storage support |
| `007388C0` | P Remove/free queues/groups, §2.3 (OV5 preserved) |
| `00738A10` | P post-add index/redisplay, §§2.3/3.5 |
| `00738A50` | R Draw selection/pass/exit boundaries proved; inverse/cofactor/all-entry rounding OV7 |
| `007393B0` | P UpdatePartRect ordering, §2.3 |
| `00739450` | S index-head copy/storage support |
| `00739490` | P draw-interface thunk subtracts 4 then forwards Draw |
| `007394B0` | P eight-part outline partition, §3.1 (OV9 preserved) |
| `00739EC0` | P handle allocation/FIFO, §2.3 |
| `00739FC0` | P group allocation/FIFO, §2.3 |
| `0073A070` | S overlay vector growth; capacity/reference/exception micro-contract not published |
| `0073A170` | N index-head range destruction, §1.2 |
| `0073A1B0` | P nine-field placement and matrices, §§3.2/3.2.1 |
| `0073AC90` | N complete explicit Shutdown boundary, §1.2 |
| `0073AD80` | P position update and tolerance, §3.2 |
| `0073ADE0` | P complete placement update, §3.2 |
| `0073AEC0` | P overlay allocation/FIFO, §2.3 |
| `0073AF80` | P source-overlay add, §§3/3.2 |
| `0073B170` | P round add/group ownership, §3.1 |
| `0073B3B0` | P key-shadow add and exact secondary transform, §3.5 |
| `0073B8A0` | N rectangle add, §3.6 |
| `0073BA10` | P initial draw-state setup retained; no general GPU restoration claim |
| `0073BB20` | S index-head vector growth; exceptional paths not specified |
| `0073BC20` | P constructor defaults, §1.1 |
| `0073BDA0` | P QueryInterface thunk, this−4 |
| `0073BDB0` | P QueryInterface thunk, this−8 |
| `0073BDC0` | P deleting-destructor thunk, this−8 |
| `0073BDD0` | P QueryInterface thunk, this−16 |
| `0073BDE0` | P AddRef thunk, this−16 |
| `0073BDF0` | P Release thunk, this−16 |
| `0073BE00` | N destructor body distinct from Shutdown, §1.2 |
| `0073BF10` | P binding decal add, §§3/3.2 |
| `0073C0B0` | P source add wrapper, §§3/3.2 |
| `0073C0D0` | N model add/records/secondary transform, §3.7; lighting producer OV1 |
| `0073C840` | S index-head vector insert/storage; exceptional paths not specified |
| `0073C980` | N deleting-destructor wrapper, §1.2 |
| `0073C9A0` | N city decal add, §3.6 |
| `0073CAB0` | S index-head vector resize; exceptional paths not specified |
| `0073CB10` | P index allocation/shift/stamp initialization, §2.2 |
| `0073CB60` | P mutating Init/repeat behavior, §1.1 (OV4 preserved) |

## Residual boundaries parked after the final round

- OV1: primary lighting producer `007D8C80` and high-quality bounds producer `007D6CC0`, source-point
  and descriptor internals, including terrain-dependent projection/precision. The owner call/return and
  postprocessing are specified; no accepted external finished-overlay API substitutes for those bodies.
- OV7: complete inline cofactor/spill equations `00738ADA–00739020` and all-entry accumulation schedules
  in decals/full/rough shadows. Reciprocal store and singular-path control flow are now specified.
- OV8: vertex/index consumer bodies `007541C0`, `007543C0`, `007545C0`, `00754910`, `00754C60`,
  `00754F10`, `00755560`, `00755700`; terrain slot-53 internal meaning; lighting slot-15 strength
  interpolation and its terrain inputs. No TR33–TR37 dependency contract or successful host GPU pass is inferred.
- S rows explicitly retain inventory-only storage/exceptional gaps. A full renderer or native equivalence
  verdict requires more than closing the public-method inventory; this report requests no further RE round.
