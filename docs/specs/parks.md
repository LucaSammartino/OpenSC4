# Spec: the park manager (`cSC4ParkManager`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (references §2.7, components §3.6, response curves §4, messages §5,
the preview-occupant property §5.2, random numbers §6), [budget.md](budget.md) (departments and line items §7–§8,
the service-simulator pattern §12), [simulator.md](simulator.md) §4 (agents),
[civic-buildings.md](civic-buildings.md) (the sibling ape escape and run-down logic), [advisors.md](advisors.md)
§7.6 (the zoo-escape event), [pollution.md](pollution.md) §6 (the occupant-list record format).

## 1. Role

The park manager keeps a list of the **park buildings** and a list of the **zoos** in the city. It does three
things:

1. it charges every park's cost to the **Parks and Recreation** department (`0x8A4260A6`), one line item per park
   type, and counts the parks of each type in the line item's secondary field (shown in the budget panel);
2. once a month it marks every park lot **run down** (lot condition 2) when the department's funding falls to the
   "distress threshold", and restores condition 0 when funding rises above it;
3. once a month, with a probability from a response curve of the department's funding, an animal **escapes from a
   zoo**: it posts the ZooEscape message (the safety advisor reacts) and starts the `zoomanji` automata.

The park manager has no interface methods of its own beyond `Init` and `Shutdown`; nothing else calls into it.

| Class | CLSID | Interfaces (IID) | City component | City getter |
|---|---|---|---|---|
| park manager | `0xAA3FE6BC` | `cISC4ParkManager` (`0xAA3FE6BC`, the same value as the CLSID) and `cIGZUnknown` (`1`) at +0, `cIGZMessageTarget2` (`0x452294AA`) at +4, `cIGZSerializable` (`0xE4FDA3D4`) at +8 | 15 | `cISC4City` slot 74 (`+0x128`) |

`QueryInterface` for any other IID returns false.

Factory `0x006D5AE0` (`GlobalHeapAlloc(0xA10)`: the object embeds a Mersenne Twister state), constructor
`0x006D57D0`. Vtables: primary `0x00AB20AC` (6 slots), message target `0x00AB209C` (`DoMessage` `0x006D5C50`),
serializable `0x00AB2084` (Write `0x006D5170`, Read `0x006D51B0`, GetGZCLSID `0x006D5050`).

## 2. State

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C` | `initialized` (u8) | 0 | no |
| `+0x10` | reference count | 0 | no |
| `+0x14` | `tuningRead` (u8) | 0 | no |
| `+0x15` | `underfunded` latch (u8) | 0 | **no** |
| `+0x18` | `parks`: list of building occupants in occupant group `0x1006`, each held | empty | yes |
| `+0x1C` | `zoos`: list of building occupants in occupant group `0x1702`, each held | empty | yes |
| `+0x20` … `+0x30` | held city, budget simulator, message server, Parks and Recreation department, lot manager | null | no |
| `+0x34` | `distressThreshold` (float) | 0.25 (`0x00B0D3D8`) | no |
| `+0x38` | `escapeCurve` (response curve, foundations §4) | empty | no |
| `+0x48` | generator: a `cRZRandomSuper` member seeded with −1 (wall clock) at construction | | no |

Both lists may hold the same occupant twice only through repeated insert messages; the shipped zoo is in **both**
groups (`0x1006` and `0x1702`), so it is on both lists.

## 3. Agents, messages and cadence

`Init` registers one agent: `AddAgent(+4, level 6 (month), "cSC4ParkManager" (string `0x00AB20C4`), flags 2)`
(enabled only when the city is established, simulator.md §4). It subscribes to `0x99EF1142` (occupant inserted) and
`0x99EF1143` (occupant removed). `DoMessage` returns true for every message.

| Message | Handling |
|---|---|
| `0x66956816` SimNewMonth | monthly pass (§4.4) |
| `0x99EF1142` | `occ := void1`; if `occ.AsPropertyHolder()` **has** the property `0x029244DB` (preview occupant; presence, not value) ignore, else §4.2 |
| `0x99EF1143` | the same test, then §4.3 |

It posts one message, `0xAA667CE2` ZooEscape (§4.5).

## 4. Behaviour

### 4.1 `Init()` and `Shutdown()`

`Init`: if `initialized`, return true. Otherwise set `initialized`, then:

1. If `tuningRead` is clear: read the tuning (§6), then set `tuningRead` (it is read once per object).
2. Hold the city and the budget simulator.
3. `dept := budget.GetDepartmentBudget(0x8A4260A6)`. If none: `CreateDepartmentBudget(0x8A4260A6, group 0x6A357B96)`,
   `SetDepartmentName(0x6A3FF01C, 0xCA49375E)` ("Parks and Recreation"), `SetMaxAllowedFundingPercentage(1.0)`.
   Hold it.
4. Hold the lot manager.
5. If no message server is held yet: hold it and subscribe the two messages.
6. `AddAgent` (§3). Return true.

Step 1 sets `tuningRead` whatever the read returned (`0x006D5B1C`–`0x006D5B21`), so a missing exemplar is not
retried by a later `Init` on the same object (QUESTIONS PK4).

`Shutdown`: if `initialized`: clear it; release every entry of both lists and empty them; `RemoveAgent(+4)` on the
city's simulator; unsubscribe both messages; release the held message server, department, lot manager, budget
simulator and city. Return true. The latch and the tuning are kept.

### 4.2 Occupant inserted *[compat]*

If `occ.GetType()` is `0x278128A0` (building):

1. **Park.** If `occ.IsOccupantGroup(0x1006)`:
   1. `AddRef` the occupant and append it to `parks`.
   2. If `budget.GetBudgetItemForPurpose(holder, 0xCA639989, &item)` fails, go to step 2.
   3. If `occ` does not answer `QueryInterface(0x87DDA3A9)` (building occupant), **return** (step 2 is skipped).
   4. `dept := budget.GetDepartmentBudget(item.department)`. *[quirk]* There is no null check: an item naming a
      missing department crashes the original. *[choice]* FreeCity4 skips steps 5–7 then.
   5. If `item.line = 0`: `line := building.GetBuildingType()` and `budget.ChangeBudgetItemLine(holder, item.index,
      line)`; if that type is 0, skip to step 8.
   6. `li := dept.GetLineItem(line)`; if absent, `li := dept.CreateLineItemForBuildingType(line, isLocal = false)`
      and `li.SetDisplayFlag(2, true)` (show the secondary field).
   7. `li.AddToFullExpenses(item.cost)`; `li.SetSecondaryInfoField(li.GetSecondaryInfoField() + 1)` (64-bit).
   8. Release the building interface.

   Null pointers *[choice]* (QUESTIONS PK5). The original tests neither of these: a missing department is
   dereferenced at `0x006D571F` (after step 5 has already changed the item's line), and a line item that
   `CreateLineItemForBuildingType` cannot make is dereferenced by `SetDisplayFlag` at `0x006D573E`; both crash it.
   FreeCity4 skips steps 5–7 for a missing department and step 7 for a missing line item, and then still runs step
   8 and the zoo step.
2. **Zoo.** If `occ.IsOccupantGroup(0x1702)`: `AddRef` and append it to `zoos`.

### 4.3 Occupant removed *[compat]*

If `occ` is a building:

1. **Park.** If in group `0x1006`: remove **every** entry equal to `occ` from `parks`, then release `occ` **once**
   (also when it was not on the list *[quirk]*). If the holder has a budget item for `0xCA639989`:
   `li := budget.GetLineItemFromBudgetItem(item)`; if it exists, `li.AddToFullExpenses(−item.cost)` and
   `li.SetSecondaryInfoField(li.GetSecondaryInfoField() − 1)`. The line item is **never removed**, even at 0 parks.
2. **Zoo.** If in group `0x1702`: remove every entry equal to `occ` from `zoos` and release it once.

The list removal (`0x006D5200`) only unlinks and frees the matching nodes; it releases nothing. The single release
follows it unconditionally (`0x006D52C3` for parks, `0x006D5366` for zoos), so removing an occupant that was never
listed still releases it once *[compat]* (QUESTIONS PK7). In the shipped game every removal message follows an
insert of the same occupant, so the counts balance.

### 4.4 Monthly pass (`0x006D58B0`) *[compat]*

1. If `parks` is empty, return (no zoo check either).
2. `f := *dept.GetFundingPercentage()` (a fraction: 1.0 = 100 %).
3. **Distress.**
   - If `f ≤ distressThreshold` (a NaN counts as not ≤) and the latch is clear: set the latch, `c := 2`.
   - Else if `f > distressThreshold` (strictly, NaN is not >) and the latch is set: clear the latch, `c := 0`.
   - Otherwise nothing is set and step 4 is skipped.
4. If `c` was set: for each entry of `parks`, in list order: find its lot (the lot manager's `GetLot(cellX, cellZ,
   false)` at the cell that `city.PositionToCell` gives for the occupant's position); if there is one,
   `lot.SetCondition(c)`.
5. **Zoo escape.** If `zoos` is not empty: `p := escapeCurve(f × 100) × 0.01` (both constants are `float`, the
   products are single precision, `p` is then widened to `double`); `r := generator.RandomDoubleUniform()`. If
   `r < p` (strictly): the zoo escape (§4.5).

Null pointers and the empty curve *[choice]* (QUESTIONS PK4, PK6). Step 2 calls the held department without a null
test (`0x006D58C4`) and reads through the returned funding pointer without one (`0x006D58CA`); the curve evaluator
(`0x005F4250`) reads the first point of an empty curve through a null array (`0x005F4289`). Each crashes the
original. FreeCity4 stops after step 1 when there is no department or no funding, and evaluates an empty curve as 0
(no escapes, but the draw of step 5 is still made).

*[quirk]* The shipped threshold is **25.0** (property name "Efficiency Distress Threshold", meant in percent) but it
is compared with a fraction. `f ≤ 25` always holds, so the first monthly pass after each `Init` with at least one
park sets every park lot to condition 2, and nothing ever restores it from here. Because the latch is not saved,
this repeats in every session. In the corpus every park lot has condition 0 (one has 1; 33 park lots in 8 saves), so
something else, most likely the lot developer's condition test (lot-developer.md §5.5), resets it *[inferred]*
(QUESTIONS PK1).

### 4.5 Zoo escape (`0x006D54F0`) *[compat]*

1. `n :=` the length of `zoos` (if 0, return). `i := generator.RandomUint32Uniform(n)`; `zoo :=` the i-th entry; if
   it is null, return.
2. Post `0xAA667CE2` (ZooEscape): a new standard message with void1 = the park manager and IGZUnknown = the zoo
   occupant, `MessagePost(msg, false)`. The safety advisor turns it into the event `0x2A667FC3` (advisors.md §7.6).
3. If `city.GetAutomataControllerManager()` exists: `AddControllersFromOccupantGroup("zoomanji", zoo, 0)`
   (automata controller manager slot 10).

### 4.6 Worked examples

Shipped tuning: threshold 25.0, curve (0, 40), (25, 10), (100, 1).

| Department funding | `f × 100` | curve | `p` per month (zoo present) |
|---|---|---|---|
| 0 % | 0 | 40 | 0.39999998 |
| 25 % | 25 | 10 | 0.099999994 |
| 50 % | 50 | 10 + 25 × (−9 / 75) = 7 | 0.0700000003 (float32 of 7 × 0.01f) |
| 100 % | 100 | 1 | 0.0099999998 (0.01f) |
| 120 % | 120 | 1 (clamped) | 0.0099999998 |

A city with a small park and the zoo: the park list has two entries (the zoo is a park too), the zoo list one. At
the first SimNewMonth after loading, both lots get condition 2 and the latch is set. At 100 % funding a draw below
0.0099999998 posts ZooEscape and starts `zoomanji` at the zoo.

With the code default threshold 0.25 (no "Parks Manager" exemplar): funding 20 % → condition 2; the next month at
30 % → condition 0.

Inserting three 1×1 grass parks (`PZ1x1_Grass`, cost 5, line 0) gives one Parks and Recreation line item keyed by
the grass park's building type, full expenses 15, secondary field 3. Removing all three leaves the line at 0 and 0.

## 5. Random numbers

The member `cRZRandomSuper` at `+0x48` (seeded with −1, the wall clock, when the object is constructed):

- one `RandomDoubleUniform()` per monthly pass with at least one park and one zoo;
- one `RandomUint32Uniform(n)` per escape.

The constructor builds it with `cRZRandomSuper(−1)` (`0x006D5850`–`0x006D5855`, constructor `0x00913B74`), so the
original seeds it from the wall clock once per object; `Init` does not reseed it. The original's state is not saved.
It is listed in foundations §6.2 ("a `cRZRandomSuper` member, wall clock (seed −1) at construction"). *[choice]*
FreeCity4 seeds it with `derive(0xAA3FE6BC)` (foundations §6.3, member generators: `K` = the GZCLSID), at
construction (QUESTIONS PK9). Like every `cRZRandomSuper`, it ignores bit 0 of its seed (foundations §6.1).

## 6. Tuning *[compat]*

Exemplar "Parks Manager" (type `0x6534284A`, group `0x67CD5FA1`, instance `0x8A4805BC`), read with IID
`0xA52160F5`:

| Property | Name (prior art) | Use | Missing | Shipped |
|---|---|---|---|---|
| `0xCA480830` | Efficiency Distress Threshold | `distressThreshold` (float) | keeps 0.25 | 25.0 |
| `0x4A6676AD` | Zoo Escape Chance Curve | `escapeCurve`; input funding in percent, output percent per month | code default (0, 50), (100, 1) | (0, 40), (25, 10), (100, 1) |
| `0xCA480831` | — | not read | | s64 0 |

If the exemplar itself is missing nothing is read: the threshold stays 0.25 and the curve stays empty
("keep" pattern of foundations §4.1, so an empty curve). *[choice]* FreeCity4 evaluates an empty curve as 0
(foundations §4.3), so no zoo escapes. When the exemplar exists without the curve property, the curve is rebuilt
from the code default ("default" pattern).

Data on the buildings:

| Data | Meaning | Shipped |
|---|---|---|
| occupant group `0x1006` | park | 32 exemplars, from 1×1 plazas to the stadiums, the Opera House and the zoo |
| occupant group `0x1702` | zoo | only `RW80x80_Zoo_0339` |
| budget item purpose `0xCA639989` | park maintenance | the "Park" cohort: department `0x8A4260A6`, line 0, cost 10; exemplars override the cost (0 for the tourist trap … 340 for the major league stadium) |

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0xAA3FE6BC`) *[compat]*

```
u16  version = 2
list parks      (pollution.md §6: u32 count, then count refs; class 0xA9BD882D)
list zoos
```

`Read`: fail unless the version is exactly 2; read both lists with the shared list reader (civic-buildings.md §7:
cleared first, refs read with IID `0x26BB767E`, an unresolvable reference appends null). `Write` and `Read` return
true unless the stream reports an error. The latch, the tuning and the department are not saved; the department
is found again by ID in `Init`.

`Read` (`0x006D51B0`) is **not atomic** *[compat facts]* (QUESTIONS PK8). A wrong version returns false with nothing
changed. Otherwise both lists are read one after the other with no error test in between, each cleared first
without releasing its old entries (`0x005DB3C0`), and the result is "the stream reports no error" (`0x006D51EE`).
*[choice]* FreeCity4 reads both lists into temporaries and changes nothing on failure; on success the lists are
replaced without releasing their old entries, as in the original. With the shipped game's one `Read` on a fresh
object the two agree on every valid save.

Corpus: all 421 distinct saves parse exactly. 413 records are `02 00 00 00 00 00 00 00 00 00` (no parks, 10 bytes);
the others hold 1, 3, 6 or 9 parks (33 references, all class `0xA9BD882D`) and no zoo.

## 8. Interface slot table *[compat]*

`cISC4ParkManager` (vtable `0x00AB20AC`):

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x006D5060` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x004F7BA0` (shared) | 0 | 0 | EAX |
| 2 | `+0x08` | `Release()` | `0x006D5140` (shared with the industrial simulator) | 0 | 0 | EAX (0 when destroyed) |
| 3 | `+0x0C` | `Init()` | `0x006D5B00` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x006D5420` | 0 | 0 | AL (1) |
| 5 | `+0x14` | deleting destructor | `0x006D5860` | 1 | 4 | EAX |

`Release` destroys the object (Shutdown, then the deleting destructor) when the count is 1 or less, without
decrementing it first.

## 9. Who reaches in

| Caller | Uses |
|---|---|
| the city (component 15) | creates it, `Init`, `Shutdown`, save and load |
| the safety advisor (`0x004169E0`) | subscribes to ZooEscape `0xAA667CE2` (advisors.md §7.6) |
| the automata controller manager | receives `zoomanji` controllers (§4.5) |

## 10. Original quirks (reproduced)

- The distress threshold 25.0 is compared with a fraction (§4.4).
- The latch is not saved, so the run-down pass repeats once per session (§4.4).
- No zoo check when the park list is empty (§4.4 step 1); only matters for zoos outside group `0x1006`.
- Removal releases once even when the occupant was not listed, and removes every duplicate (§4.3).
- Park line items are never removed (§4.3).
- A failed building `QueryInterface` on insert skips the zoo test (§4.2).

## 11. Open questions

See QUESTIONS.md, section "From the commercial, industrial, civic, park, history and clock specs" (PK1–PK3).

## 12. Conformance tests (must exist in `engine/conformance`)

1. **Init**: creates department `0x8A4260A6` (group `0x6A357B96`, name `0x6A3FF01C`/`0xCA49375E`, maximum 1.0) only
   when missing; subscribes the two occupant messages; one month agent with flags 2.
2. **Tuning**: shipped exemplar → threshold 25.0 and curve (0,40)(25,10)(100,1); exemplar without the curve
   property → curve (0,50)(100,1); no exemplar → threshold 0.25 and an empty curve.
3. **Insert**: a park with cost 10 and line 0 → the item's line becomes the building type, a line item with display
   flag 2, full expenses 10, secondary 1; a second park of the same type → 20 and 2. A preview occupant (property
   `0x029244DB` present, even false) changes nothing.
4. **Remove**: removing both → 0 and 0, the line item stays; a duplicate entry is removed together with the first.
5. **Distress latch**: threshold 0.25; f = 0.25 → condition 2 on every park lot, latch set; next month f = 0.2 →
   nothing; f = 0.3 → condition 0, latch clear; NaN → nothing in either direction.
6. **Shipped threshold**: threshold 25.0, f = 1.2 → condition 2 at the first pass, never restored.
7. **Escape draw**: f = 0.5 gives `p` = float32(7 × 0.01f) = 0.0700000003 (widened); a stub generator returning
   0.07 (double) escapes and 0.0700001 does not; the escape
   posts `0xAA667CE2` with void1 = the manager and IGZUnknown = the zoo, then adds `zoomanji`; no zoos → no draw;
   no parks → no draw.
8. **Save**: write → read → write is byte-identical; version 1 or 3 is rejected; all 421 corpus records round-trip.
