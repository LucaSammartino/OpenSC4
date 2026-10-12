# Tract developer (`cSC4TractDeveloper`) — behaviour spec

Status: round 1 (M4), written from the Windows 1.1.641 executable. The earlier write-up in
`docs/subsystems/development.md` (largely from the Mac build) was the starting point. Every fact below was checked
against the Windows code. Corrections to that write-up and to other specs are listed in §10.3.

Tags:
- *[compat]*: observable, must match.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.

**Cited, not repeated here**
- foundations: streams and refs §2, class store and city components §3.6 / §3.7, messages §5, random generators §6,
  C `rand()` and random cell iterators §12, month order §13, slot-order rule §16, x87 precision §18.
- simulator spec: scheduler levels.
- simgrid spec: grid ownership rule §6.4.
- weather spec §6.7: the random iterator in prime-step mode.
- Specs being written in parallel; this spec states only the call contracts it relies on:
  - `building-development.md` (BDS): building catalogue, construction resources, `BuildingFitsInZone`, cheats,
    the daily caller of `Grow`, the caller of `Decline`;
  - `growth-developer.md`: the developer (`GrowthProfile.developer`), its maps, its caps, the caller of
    `UpdatePopulation`;
  - `lot-developer.md`: `StartConstruction`, abandon / reoccupy, `NeedsAnchorBuilding`, `GetAreaSlope`;
  - `zones.md`: zone types, the lot manager (`GetLot`, `CanCreateLot`, `CreateLotIfPossible`, `DeleteLot`, lot
    lists).

**Notation**
- `r24(·)` rounds to float32 (foundations §18).
- `trunc(·)` is the CRT `_ftol` (`0x009EEF04`, mislabelled `_lua_gettop`): int64 truncation toward zero, keeping the
  low 32 bits. NaN gives 0 (foundations §18.1).
- `rhe(x)` is round-half-even to an integer through `r24(x + 12582912.0f)`: the low bits of the float32 bit pattern
  minus `0x4B400000`, exact for `0 ≤ x < 2^22`.
- Purposes (`cISC4BuildingOccupant::PurposeType`): 1 residence, 2 services, 3 office, 4 tourism, 5 agriculture,
  6 processing, 7 manufacturing, 8 high tech.
- Wealth: 1–3 = $, $$, $$$.
- Network types (traffic): 0 road, 1 rail, 3 street, 6 avenue, 10 one-way road, 11 dirt road.
- Facings 0–3 as stored on lots.

---

## 1. Role and identity

The tract developer carries out one **growth attempt** on one tract for one developer:
`Grow(profile)` (§4.3), called daily by the BDS (`building-development.md` §4.4). It:

1. tries to **reoccupy** abandoned buildings, or buildings of lower wealth when kick-out is on;
2. otherwise lists the **lot configurations** the developer could grow;
3. lists **candidate sites**: existing lots, aggregations of several lots, subdivisions of one lot;
4. **scores** and sorts the candidates;
5. for each candidate in order, **picks a building** at random and starts construction through the lot developer,
   until it has built enough.

It also:
- abandons qualifying buildings for **decline** (`Decline`, §4.19);
- sets **per-lot populations** with a seeded random occupancy (`UpdatePopulation`, §4.18);
- keeps the **architectural style** rotation (`NewYear`, §4.20).

Every step reports an `Outcome` code (§4.1), and a debug histogram counts them. The tract developer **owns no
grids**. It holds no persistent per-tract state: only the style rotation is saved (§6).

### 1.1 Identity

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`) | `0x2990C142` |
| City component | #17 "tract developer", IID `0xA818ED1B` (foundations §3.6). Global pointer `0x00B43D24` (read by the BDS, `building-development.md` §8) |
| Constructor | `0x0070C900`. Object allocated from the global heap C |
| Interfaces (`QueryInterface` `0x007045B0`) | `cIGZUnknown` `0x00000001` and `cISC4TractDeveloper` `0xA818ED1B` → primary (vtable `0x00AB3118`, 15 interface slots + scalar deleting destructor). `cIGZMessageTarget2` `0x452294AA` → sub-object +4 (vtable `0x00AB3104`). `cIGZSerializable` `0xE4FDA3D4` → sub-object +8 (vtable `0x00AB30EC`). Success AddRefs |
| Unknown IID | returns false |
| `Release` (`0x007048B0`) | count > 1: decrement and return the new count. Otherwise: `Shutdown` (slot 4), then the scalar deleting destructor (slot 15) with 1, return 0 |

### 1.2 Creation and Init order

- Created by the city as component 17. Load order is `Read` before `Init` (foundations §3.7).
- `Init` order: … building development, **tract developer**, lot developer, … (foundations §3.7). The BDS is
  therefore initialised first. The lot developer's `Init` has not run yet, but `Init` only AddRefs its pointer.
- Month-level registration order: 13th (foundations §13). Its month handler does nothing (§3), so the order has no
  effect.

### 1.3 `Init()` → bool (slot 3, `0x00707780`)

1. Already initialised → return true.
2. Set the class-wide "configured" flag and read the tuning (§4.21).
3. Take and AddRef the city services: the city, the BDS, land value, the lot developer, the lot configuration
   manager, the lot manager, plumbing, power, traffic and the zone manager. Any of them null → return false.
4. Register this object (its `cIGZMessageTarget2` sub-object) with the simulator through simulator slot 19
   (`+0x4C`) `AddAgent(target, level, name "cSC4TractDeveloper", 2)`:
   - first at level 6 (Month, message `0x66956816`);
   - then at level 7 (Year, `0x66956817`).

   No simulator → false.
5. Take and AddRef the message server. Null → false.
6. Mark initialised and return true.

Read directly from city globals when used; never acquired:
- the fire protection simulator (`0x00B43D78`);
- the aura simulator (`0x00B43D60`);
- the terrain (`0x00B43CF4`).

### 1.4 `Shutdown()` → true (slot 4, `0x0070B210`)

If initialised:
1. Clear the flag and release the message server.
2. Remove the agent: simulator slot 20 (`+0x50`), obtained through the city.
3. Release the city, BDS, land value and lot developer.
4. If the lot configuration manager is held:
   - unregister every cached configuration filter (§4.6) with its slot 19 and free each filter (C);
   - clear the filter map;
   - release the manager.
5. Release the lot manager, plumbing, power, traffic and zone manager.
6. Clear the debug containers.
7. Destroy every pooled candidate record (§2.3) and clear the candidate list and the class-wide pool.

The style state (§2.1) is kept.

---

## 2. State

### 2.1 Persistent and configuration state

| State | Initial (constructor) | Saved | Meaning |
|---|---|---|---|
| `activeStyles` (vector<uint32>) | empty; at `Init`, if empty, := `availableStyles` | yes | style IDs in rotation |
| `currentStyleIndex` (int32) | 0 (reset to 0 at `Init` if ≥ size) | yes | index into `activeStyles` |
| `yearsSinceStyleChange` (int32) | 0 | yes | |
| `singleStyle` (bool) | true | yes | true = only the current style (`IsUsingAllStylesAtOnce` = false) |
| `yearsBetweenStyleChanges` (int32) | 0; at `Init`, if 0 → 40, then tuning `0x47E2C551` | yes (v ≥ 3) | |
| `availableStyles` (vector<uint32>) | empty; tuning `0x47E2C550` | no | |
| Tuning floats and kick-out flag | §4.21 | no | |

### 2.2 Per-attempt working state (rebuilt by each `Grow`; not saved)

| State | Meaning |
|---|---|
| `profile` | developer, tract `(x, z)`, `maxStage` (the low byte of the profile's stage field) |
| `purpose`, `constructionInfo` | `developer.GetPurpose()`, `developer.GetConstructionInfo()` (growth-developer §6.2: zone code, wealth, purpose, land value min / max) |
| `existingCapacity` | low 16 bits of the developer's capacity map tract value at `(x, z)` |
| `desired`, `desiredMax` | §4.2 |
| `built` | capacity built so far in this attempt (int32) |
| `relaxed` | false in the first pass, true afterwards |
| `outFlag` | written to `Grow`'s `bool&` (§4.9.1) |
| lot set | the existing lots touching the tract: `std::set<cISC4Lot*>` ordered by **pointer value** |
| configuration set | the suitable lot configurations: `std::set<cISC4LotConfiguration*>` ordered by **pointer value** |
| candidate list | vector of candidate records (§2.3) |
| suitability cache | map lot → `IsLotSuitable(lot, true)` result, cleared at the start of each pass |
| `chosenConfiguration`, `chosenBuilding` | set by `PickBuilding` (§4.16) |
| debug histogram | 13 rows × 28 counters (§4.1); debug lists |

### 2.3 Candidate record

Fixed-size records (`0xA4` bytes, C). They are taken from a class-wide pool (`0x00B4AFCC`, refilled 10 at a time)
and initialised from a template. The template constructor is `0x00707680`; the pooled copy is made by `0x00707D90`.

| Field | Template | Meaning |
|---|---|---|
| type | 0 | 1 reoccupy (§4.4), 2 existing lot (§4.9.1), 3 aggregation (§4.9.2), 4 subdivision (§4.9.3) |
| lot | null | the lot, for types 1, 2, 4 |
| `x`, `z` | −1 | anchor cell |
| facing | −1 | −1 = take facing, size, location and rectangle from the lot |
| size `w × h` | 0 × 0 | configuration size in cells (uint8 each) |
| lot set | empty | the lots merged by an aggregation |
| `existing`, `abandoned` | −1 | cached capacity sums (§4.11) |
| `stage`, `wealth` | 0 | highest growth stage and wealth already on the site (§4.11) |
| score | −1.0 | §4.14 |
| rectangle | (−1, −1, −1, −1) | cached cell rectangle (§4.10.1) |
| `n` | 0 | subdivision count |
| `street[4]`, `road[4]`, `avenue[4]`, `rail[4]` | 0 | network tiles per side (§4.10.2) |
| corner | false | §4.12 |
| capacity | | building capacity of a reoccupy candidate |

---

## 3. Agents, messages and cadence

| Message | Handler (`DoMessage` `0x007048E0`, on sub-object +4, returns true) |
|---|---|
| `0x66956816` (Month) | nothing |
| `0x66956817` (Year) | `NewYear` (§4.20) |

- All growth work happens inside calls from other subsystems:
  - `Grow`: the BDS, daily, once per qualifying developer, in a demand-weighted random order
    (`building-development.md` §4.4);
  - `Decline`: the BDS decline plan (`building-development.md` §4.5);
  - `UpdatePopulation`: the growth developer's state 18, once per tract (`growth-developer.md` §5.2).
- What a `Grow` call reads:
  - the developer's maps as left by its last update;
  - the current lot, zone, traffic and power / water / fire / park grids;
  - the BDS construction resources, which earlier attempts on the same day have already consumed.

---

## 4. Behaviour

### 4.1 Outcome codes and the histogram *[compat for the codes; debug only for the histogram]*

| Code | SDK name | Code | SDK name |
|---|---|---|---|
| 0 | UnknownFailure | 14 | GrowthStageTooBig |
| 1 | NothingToReoccupy | 15 | ResidentHasDifferentPurpose |
| 2 | MonopolyPiece | 16 | ResidentHasDifferentWealth |
| 3 | NoLot | 17 | NotBiggerThanExistingGrowthStage |
| 4 | LotNotReady | 18 | Historical |
| 5 | NoLotConfiguration | 19 | JustBuiltRecently |
| 6 | WrongZone | 20 | InsufficentConstructionResources |
| 7 | WrongPurpose | 21 | PoorRoads |
| 8 | WrongWealth | 22 | WrongLandValue |
| 9 | WrongStyle | 23 | NoFireCoverage |
| 10 | TooSteep | 24 | NoPark |
| 11 | BuildingTooSmall | 25 | NoWater |
| 12 | GrowthStageTooSmall | 26 | NoPower |
| 13 | BuildingTooBig | 27 | Success |

"best" below always means the **numerically largest** code seen. Success (27) is the largest.

**Histogram rows** (a 13 × 28 table of uint32 counters):

| Row | Step |
|---|---|
| 0 | Grow |
| 1 | TakeOverAbandoned |
| 2 | ListPossibleLotConfigurations |
| 3 | ListCandidateLots_Existing |
| 4 | ListCandidateLots_Aggregation |
| 5 | ListCandidateLots_Subdivision |
| 6 | Build |
| 7 | IsLotConfigurationSuitable |
| 8 | IsLotSuitable |
| 9 | IsAggregationLegal |
| 10 | PickBuilding |
| 11 | IsLotConfigurationCompatible |
| 12 | IsBuildingCompatible |

- The histogram is zeroed at the start of each `Grow`. It is never saved or read by simulation code.
- It is shown only by the debug recorders, which run under the BDS cheat `CheatDebugGrowth` (BDS slot 57).
- Each subsection says which counter a step increments. A replacement may omit the histogram entirely
  *[choice: FreeCity4 keeps it behind a debug switch]*.

### 4.2 Common quantities

```
desired    := max(1, trunc(sourceDemand.slot12()))   -- developer.GetSourceDemand() (growth-developer slot 18), cISC4Demand slot 12 (+0x30) float
desiredMax := trunc(r24(float(desired) × overbuild))  -- overbuild = tuning 0x47E2C500
```

`float(desired)` is exact. Both values are int32.

**Developer caps.** A growth-stage cap check passes unless all of the following hold:

| Cap | Fails when (the stage `s` compared is that of the configuration or building) | Code |
|---|---|---|
| power | `s > developer.GetPowerSupplyCap()` (slot 19) **and** not `CheatIgnorePower` (BDS slot 55) **and** power simulator slot 35 `GetPoweredGrid()` → grid slot 7 `GetCellValue(x, z) < 2` | 26 |
| water | `s > GetWaterSupplyCap()` (slot 20) **and** not `CheatIgnorePlumbing` (BDS slot 54) **and** plumbing slot 11 `IsCellWatered(x, z)` false | 25 |
| fire | `s > GetFireCoverageCap()` (slot 21) **and** fire simulator slot 5 → grid `GetCellValue(x, z)` = 0 (uint8) | 23 |
| park | `s > GetParkCap()` (slot 22) **and** aura slot 14 `GetParkMap()` → `GetCellValue(x, z)` = 0 (int16) | 24 |

- The checks run in the order power, water, fire, park. The first failure is returned.
- The whole block is skipped when `CheatIgnoreCaps` (BDS slot 56) is on.
- `(x, z)` is the candidate's anchor cell. Comparisons are unsigned on the uint8 caps.

### 4.3 `Grow(profile, outcomes&, outFlag&)` → int32 built (slot 5, `0x0070D130`) *[compat]*

**Setup**
1. Read the developer's purpose and construction info.
2. Compute `existingCapacity`, `desired` and `desiredMax` (§4.2), and set `built := 0`.
3. Clear the chosen configuration and building, `outFlag := false`, the debug list and the histogram.

**Pass loop**

```
passesLeft := (desirabilityMap.tractSize)² + 1        -- growth-developer slot 13's grid; 17 for size 4
first := true;  stop := false
loop:
    -- pass setup
    clear the lot set, the configuration set and the candidate list (records back to the pool);
    relaxed := not first;  clear chosen fields;  clear the suitability cache;  best := 0

    ListExistingLots()                                    -- §4.5 (one rand())
    r := TakeOverAbandoned()                              -- §4.4, always ≥ 1
    if r > best: best := r
    if r = 27:
        outcomes.push_back(27);  *outFlag := outFlag;  return built

    if ListPossibleLotConfigurations() = 27:              -- §4.6
        best := max(best, ListCandidateLots_Existing())   -- §4.9.1
        best := max(best, ListCandidateLots_Aggregation())-- §4.9.2
        if purpose ≠ 5: best := max(best, ListCandidateLots_Subdivision())   -- §4.9.3

        if the candidate list is empty:
            if best = 27: best := 3
            outcomes.push_back(best);  hist[0][best] += 1
        else:
            ScoreCandidateLots();  sort ascending by score (§4.14)
            for each candidate c in sorted order:
                r := (purpose = 5) ? BuildFarm(c) : Build(c)          -- §4.15
                best := r;  outcomes.push_back(r)
                if r = 20:                 stop := relaxed;  break
                if built ≥ desired:        stop := true;     break
                if r = 27:                 if 5 ≤ purpose ≤ 8: stop := true
                                           break
    -- (ListPossibleLotConfigurations ≠ 27: nothing is listed and nothing is pushed)

    if not first and best ≠ 27: stop := true
    passesLeft −= 1;  first := false
    if passesLeft < 0 or stop: exit loop
*outFlag := outFlag;  return built
```

**Consequences** *[compat]*
- **Pass 1** is strict: check 7 of §4.7 (code 11) applies. Every later pass is relaxed.
- A pass ends at its first success. Agricultural and industrial developers (purposes 5–8) then stop altogether.
- Residential and commercial developers start a new relaxed pass after each success, until one of these:
  - `built ≥ desired`;
  - a pass ends without success (its last code is not 27);
  - a relaxed pass runs out of construction resources (code 20);
  - the pass budget is used up. That is at most `size² + 2` passes: 18 passes for tract size 4.
- Construction-resource failure in the strict pass does **not** stop the loop. The relaxed pass is tried.
- Every pass re-lists lots and configurations. Lots built in an earlier pass appear as existing lots in state 2
  (Constructing) and fail with 4.
- `outcomes`:
  - receives one entry per build attempt, plus one per pass that had no candidate;
  - is appended to, never cleared (allocator B, §9.2);
  - is not read by the BDS.
- Return value: `built`, the capacity started in this call. It can be **negative**: demolished capacity is
  subtracted (§4.15). The BDS tests `> 0`.

### 4.4 `TakeOverAbandoned()` → code (`0x0070A3D0`) *[compat]*

Run first in every pass. It uses the lot set from §4.5.

```
devType := developer.GetSubType();  T0 := trunc(r24(float(desired − built) × reoccupy[0]))
                                    T1 := trunc(r24(float(desiredMax − built) × reoccupy[1]))
best := 1;  sum := 0
for each lot L in the lot set (pointer order):
    state := L.GetState()
    eligible := state = 4 (abandoned)  or  (kickOut and state = 3 and L.GetOccupantWealth() < developer wealth)
    if not eligible: continue
    the lot's zone must convert to the developer's target zone type (developer slot 5)           else skip
    if L.GetCapacity(devType, true) = 0 (lot slot 50, see §7.4): best := max(best, 15); continue
    r := IsLotSuitable(L, false); if r ∉ {27, 18}: best := max(best, r); continue      -- historic lots may be reoccupied
    make a type-1 candidate for L; FindExistingCapacity (§4.11)
    cap := L.GetAvailableCapacity(devType, false) (slot 52), or L.GetCapacity(devType, false) if that is 0
    caps (§4.2) with stage = candidate.stage:
        power and water: using the caps of the developer the BDS returns for the lot's building
                         (BDS slot 33 GetDeveloper(purpose, wealth) of the building's profile)
        fire and park:   using this developer's caps
        a cap failure: best := max(best, 26/25/23/24); release the candidate; continue
    candidate.capacity := cap;  sum += cap;  add the candidate
if the candidate list is not empty and sum > T0 (signed):
    ScoreCandidateLots(); sort (§4.14)
    taken := 0
    for each candidate c in order:
        if c.lot.GetState() = 3: lotDeveloper.AbandonBuilding(c.lot, 5)            -- slot 8 (kick out)
        lotDeveloper.ReoccupyBuilding(c.lot, devType)                             -- slot 9
        built += c.capacity;  taken += c.capacity
        if taken > T1 (signed): break
    clear the candidate list;  hist[1][27] += 1;  return 27
clear the candidate list
if best = 1: hist[1][1] += 1
return best
```

- With the shipped tuning, `reoccupy = {0, 1.2}`:
  - any qualifying capacity at all (`sum > 0`) triggers a takeover;
  - takeover continues until more than `trunc(1.2 · (desiredMax − built))` has been taken.
- `T0` and `T1` are computed **once**, at entry.
- The lot developer contracts (`AbandonBuilding(lot, reason)`, `ReoccupyBuilding(lot, devType)`) are owned by
  `lot-developer.md`.

### 4.5 `ListExistingLots()` (`0x00707FB0`)

- Uses a class-wide random cell iterator (`0x00B4B098`) in prime-step mode over the tract's cell rectangle (weather
  §6.7; foundations §12.4).
  - The rectangle is the tract `(x, z)` of the desirability map, scaled to cells by its tract size.
  - `Reset(true)` makes **one `rand()` call** (start index). For 4 × 4 cells: count 16, step 5,
    `cell_i = (start + 5i) mod 16`.
- For each cell, lot manager slot 6 `GetLot(x, z, false)`. Non-null lots are inserted into the lot set. There is
  no AddRef: the set holds borrowed pointers.
- The set is ordered by pointer, so the visit order has **no effect**: only the `rand()` advance is observable
  *[compat]*.

### 4.6 `ListPossibleLotConfigurations()` → code (`0x00709650`)

1. Get the configuration filter for `(wealth, purpose)` of the developer from a map cached in this object.
   - On a miss, a 12-byte filter `{vtable 0x00AB30DC, wealth, purpose}` is allocated (C), stored, and registered
     with the lot configuration manager (slot 18).
   - The filter's `IsLotConfigurationIncluded(cfg)` is `cfg.IsCompatibleWithPurposeType(purpose)` (cfg slot 17)
     **and** `cfg.IsCompatibleWithWealthType(wealth)` (slot 16).
2. Lot configuration manager slot 9 `GetLotConfigurationsByFilter(set, filter)` fills a local STLport
   `hash_set<cISC4LotConfiguration*>` (bucket hint 100 → 193 buckets).
   - A result of 0 → return **5** (no histogram).
3. For each configuration (hash order; the order is not observable):
   - `r := IsLotConfigurationSuitable(cfg)` (§4.7);
   - `hist[2][r] += 1`;
   - `best := max(best, r)`;
   - if `r = 27`, insert `cfg` into the configuration set (pointer order).
   - Then release `cfg` (the manager returns each with one reference).
4. Return `best`: 27 if any configuration is suitable, 0 if the set was empty.

### 4.7 `IsLotConfigurationSuitable(cfg)` → code (`0x00704C00`) *[compat]*

The checks run in order; the first failure is returned. `hist[7][code] += 1` for every result.

| # | Check | Code |
|---|---|---|
| 1 | `cfg.IsCompatibleWithPurposeType(purpose)` (slot 17) | 7 |
| 2 | `cfg.IsCompatibleWithWealthType(wealth)` (slot 16) | 8 |
| 3 | `stage := cfg.GetGrowthStage()` (slot 11); `stage > maxStage` | 14 |
| 4 | Style (purposes 1–3 with a non-empty `activeStyles`): if `singleStyle`, `cfg.IsCompatibleWithStyleType(activeStyles[currentStyleIndex])` (slot 18); otherwise compatible with **any** active style | 9 |
| 5 | `cfg.GetPrimaryBuildingConfigurationObject()` (slot 24) is null | 0 |
| 6 | `min := (stage = 1) ? 0 : cfg.GetMinBuildingCapacity()` (slot 12); `max := cfg.GetMaxBuildingCapacity()` (slot 13) | |
| 7 | **strict pass only** (`relaxed` false): `max < desired` **and** `stage < maxStage` | 11 |
| 8 | `min > desiredMax + existingCapacity` | 13 |
| | otherwise | 27 |

### 4.8 `IsLotSuitable(lot, redevelop)` → code (`0x00707A60`, pop 8) *[compat]*

- With `redevelop` true, a cached result for `lot` is returned without counting.
- Otherwise the checks below run in order. The result is cached (when `redevelop`) and `hist[8][code] += 1`.

| # | Check | Code |
|---|---|---|
| 1 | `BuildingFitsInZone(constructionInfo.zoneCode, zoneManager.slot12(lot location))` false (BDS helper `0x00695390`, `building-development.md`) | 6 |
| 2 | `redevelop` **and** `lot.IsInLivableState()` (slot 37) **and** `lot.GetDaysSinceLastStateChange()` (slot 40) `< 180` (unsigned) | 19 |
| 3 | Construction info land value range: if min ≠ 0 or max ≠ 0, `lv := landValue.GetLandValue(x, z)` (land value slot 6, uint8) at the lot location; `lv < min` or `lv > max` | 22 |
| 4 | `state := lot.GetState()` (slot 33): `state = 2` (Constructing) or `state ≥ 5` | 4 |
| 5 | `lotManager.CanCreateLot(x1, z1, w, h, 0, 0, 0)` on the lot's bounding rectangle (slot 20). Result 7 (a lot already exists on the cell) or 8 (free): pass. 0 → 0; 1 → 4; 2 → 6; 3 → 3; any other → 0 | as listed |
| 6 | If `lot.HasCapacity()` (slot 38), a building stands: (6a) `type := lot.GetBuildingType(false)` (slot 46) = 0 → 4; with `P :=` BDS `GetBuildingProfile(type)` (slot 39): (6b) `P.purpose = 5`, or `P.purpose = 7` while the developer's purpose is 6 → **15**; (6c) not kick-out **and** `P.wealth ≠ wealth` → 16; (6d) `wealth < P.wealth` → 16; (6e) `P.wealth = wealth` **and** `P.purpose ≠ purpose` → 16; (6f) `lot.IsHistorical()` (slot 43) → 18; (6g) stage: `s := ` the growth stage of the lot's configuration; equal wealth requires `maxStage > s`, otherwise `maxStage ≥ s`, else 17 | as listed |
| 7 | Road access: purpose 1 → the traffic simulator's cell value at the lot location (traffic grid, member at `+0x34` of the traffic simulator's data *[inferred: the road-access bitmap]*) ≠ 0; purposes 6–8 → `lot.HasOrganicRoadAccess(purpose)` (slot 29); others → `lot.HasRoadAccess()` (slot 28). Failure | 21 |
| | otherwise | 27 |

The aggregation, subdivision, existing-lot and farm helpers use the cached value, or an inlined copy of the same
checks with `redevelop = true`.

### 4.9 Candidate listing

#### 4.9.1 `ListCandidateLots_Existing()` → code (`0x00708110`)

```
best := 0
for each lot L in the lot set:
    r := IsLotSuitable(L, true)
    if L.GetState() ≠ 3 and r ≠ 6: outFlag := true
    if r ≠ 27: best := max(best, r); continue
    for each cfg in the configuration set (pointer order):
        if cfg size ≠ L size:                         hist[3][5] += 1, best ≥ 5;  next cfg
        if not GrowthStageFitsInZone(cfg stage, zone of L): hist[3][6], best ≥ 6;  next cfg   (BDS helper)
        slope := L.GetSlope() (slot 18); fail if slope < cfg.GetMinimumSlope() or slope > cfg.GetMaximumSlope()
                 (NaN fails)                          hist[3][10], best ≥ 10;  next cfg
        make a type-2 candidate (lot L, its facing, size, location); FindNetworks (§4.10.2)
        if IsLotConfigurationCompatible(cfg, c, false) = 27:     -- §4.12
            add c; hist[3][27] += 1; best := 27; stop scanning configurations for L   -- first compatible wins
        else: return c to the pool
return best
```

Only the first compatible configuration of each lot (pointer order) produces a candidate *[compat]*. `PickBuilding`
re-filters all configurations later anyway (§4.16).

#### 4.9.2 `ListCandidateLots_Aggregation()` → code (`0x007097E0`)

For each lot L in the lot set × each cfg in the configuration set (both in pointer order), with
`(w, h) := L.size` and `(cw, ch) := cfg.size`:

| Check | Effect |
|---|---|
| `cw < w` or `ch < h`, or `cw = w` and `ch = h` | `hist[4][5]`, best ≥ 5, next |
| growth stage does not fit L's zone | `hist[4][6]`, best ≥ **5** *[quirk]* |
| slope of L outside `[min, max]` of cfg (NaN **passes**) | `hist[4][10]`, best ≥ **5** *[quirk]* |

A passing pair makes two type-3 trial candidates, each of size `(cw, ch)` with L's facing:

- (a) anchored at L's location;
- (b) anchored at L's location shifted by `d := cw − w`:

  | Facing | Shift |
  |---|---|
  | 0 | x + d |
  | 1 | z + d |
  | 2 | x − d |
  | 3 | z − d |

For each trial: `FindNetworks`. Then, if `IsAggregationLegal(c)` (§4.13) **and**
`IsLotConfigurationCompatible(cfg, c, false) = 27`:
- add the candidate;
- `hist[4][27] += 1`;
- `best := 27`.

Otherwise return the trial to the pool and set best ≥ 5.

Both trials may be added. Returns `best`; 0 when the lot set is empty.

#### 4.9.3 `ListCandidateLots_Subdivision()` → code (`0x00708C00`)

Not called for purpose 5.

```
for each lot L in the lot set:
    r := IsLotSuitable(L, true);  if r ≠ 27: best := max(best, r); continue
    for each cfg in the configuration set:
        (w, h) := L.size; (cw, ch) := cfg.size
        if w < cw or h < ch or (w = cw and h = ch):      hist[5][3], best ≥ 3; next
        if w mod cw ≠ 0 or h ≠ ch:                       hist[5][3], best ≥ 3; next     (cw = 0 would divide by zero)
        if stage does not fit L's zone:                  hist[5][6], best ≥ 6; next
        if L's slope is outside cfg's range:             hist[5][10], best ≥ 10; next
        make a type-4 candidate: lot L, size (cw, ch), L's facing and location, n := w / cw; FindNetworks
        if IsLotConfigurationCompatible(cfg, c, false) = 27: add, hist[5][27], best := 27
        else: return to pool                             (best unchanged)
```

Only splits **along the lot's width** (`w`) into `n` equal pieces of the same depth are considered.

### 4.10 Geometry helpers

#### 4.10.1 `GetBoundingRect(c)` (`0x00704630`)

- Cached once all four fields are ≥ 0.
- If `c.facing = −1`, take facing, size, location and rectangle from `c.lot`.
- Otherwise, with anchor `(x, z)` and size `(w, h)`:

| Facing | x range | z range |
|---|---|---|
| 0 | `x−w+1 … x` | `z … z+h−1` |
| 1 | `x−h+1 … x` | `z−w+1 … z` |
| 2 | `x … x+w−1` | `z−h+1 … z` |
| 3 | `x … x+h−1` | `z … z+w−1` |

#### 4.10.2 `FindNetworks(c)` (`0x00705040`, pop 4)

The rectangle comes from `GetBoundingRect(c)`; for type 4, the lot's own rectangle. The four sides are scanned
just outside the rectangle:

| Side | Scanned when | Cells |
|---|---|---|
| 0 | `z1 > 0` | row `z1 − 1`, `x1 … x2` |
| 1 | `x2 < cellCountX − 1` | column `x2 + 1`, `z1 … z2` |
| 2 | `z2 < cellCountZ − 1` | row `z2 + 1`, `x1 … x2` |
| 3 | `x1 > 0` | column `x1 − 1`, `z1 … z2` |

- `cellCountX/Z` come from city slots 93/94.
- For each cell: traffic network map (city slot 40) slot 8 `FindNetworkTile(x, z, 0xC4B, 1)`.
- For a found tile, each category whose test is true adds 1 to that side's counter, at most once per category per
  tile (network occupant slot 22 `IsOfType(type)`):

  | Category | Network types |
  |---|---|
  | street | 3 or 11 |
  | road | 0 or 10 |
  | avenue | 6 |
  | rail | 1 |

- The counters are not reset: they start at the template's zeros.

#### 4.10.3 `TestForCornerLot(cfg, c)` → bool (`0x00705DA0`, pop 8, AL)

- `cfg.GetRequiredRoads(bits)` (slot 21) gives a `vector<bool>` of up to 4 sides, in the configuration's own frame.
- For every set bit `i` in 0…3, the side `j := (c.facing + i + 1) mod 4` needs `avenue[j] + road[j] + street[j] > 0`.
  If any required side has none, the result is false. **Rail does not count.**
- Otherwise true. The bit buffer is freed with B.

### 4.11 `FindExistingCapacity(c)` (`0x007055B0`, pop 4)

- Cached when `existing ≥ 0` and `abandoned ≥ 0`.
- Otherwise zero `existing`, `abandoned`, `stage` and `wealth`, then take the lots:
  - types 1, 2, 4: `c.lot`;
  - type 3: `c.lotSet`.
- For each lot:

```
cap := lot.GetCapacity(developer.GetSubType(), true)
if lot.GetState() = 4: abandoned += cap   else: existing += cap
stage := max(stage, configuration ? configuration.GetGrowthStage() : 0)
if lot.GetBuildingType(false) ≠ 0: wealth := max(wealth, BDS.GetBuildingProfile(type).wealth)
```

### 4.12 `IsLotConfigurationCompatible(cfg, c, requireRoads)` → code (`0x00706500`, pop 12) *[compat]*

`FindExistingCapacity(c)` runs first. `hist[11][code] += 1` for every result.

| # | Check | Code |
|---|---|---|
| 1 | `cfg.GetSize()` (slot 7) ≠ `(c.w, c.h)` | 0 |
| 2 | `cfg.IsCompatibleWithZoneType(zoneManager.slot12(c.x, c.z))` (slot 15) false | 6 |
| 3 | `s := cfg.GetGrowthStage()`. If `cfg.IsCompatibleWithWealthType(c.wealth)` (slot 16): need `s > c.stage`, else need `s ≥ c.stage` | 12 |
| 4 | `slope := c.lot ? c.lot.GetSlope() : lotDeveloper.GetAreaSlope(&c.rect, 0)` (slot 29); fail if `slope < cfg.GetMinimumSlope()` (slot 9) or `slope > cfg.GetMaximumSlope()` (slot 8). **NaN passes** | 10 |
| 5 | Developer caps (§4.2) with `s` at `(c.x, c.z)` | 26 / 25 / 23 / 24 |
| 6 | If the developer's target zone type (slot 5) ≠ 3: `hasReq := ` any of required-road bits **0, 1, 2** set (bit 3 is not considered) *[quirk]*. If `requireRoads` and not `hasReq` → 21. Else if `TestForCornerLot(cfg, c)` is false → 21. Else if `hasReq` → `c.corner := true` | 21 |
| | otherwise | 27 |

### 4.13 `IsAggregationLegal(c)` → bool (`0x00708700`, pop 4)

```
rect := GetBoundingRect(c);  if any part is outside the city → false (no histogram)
sameFacing := false;  zone := unset
for each cell of rect:
    L := lotManager.GetLot(x, z, false);  if null → false
    r := cached or inline IsLotSuitable(L, true)
    if r = 27: if L.facing = c.facing: sameFacing := true
    else if r ≠ 21: → false                  (no row-9 histogram)
    z' := L.GetZoneType();  if zone unset: zone := z'  else if z' ≠ zone: hist[9][6]; → false
    insert L into c.lotSet
if not sameFacing:                hist[9][21]; → false
if |c.lotSet| < 2:                hist[9][3];  → false
if any L's rectangle is not inside rect: hist[9][3]; → false
hist[9][27]; → true
```

- Lots with poor roads (21) may be absorbed.
- At least one fully suitable lot must face the same way as the new lot.

### 4.14 `ScoreCandidateLots()` (`0x00705750`) and sorting *[compat]*

Weights are chosen by the developer's target zone type (developer slot 5):

| Zone type | Road `wR` | Avenue `wA` | Rail `wL` | Random `wX` | Lot costs |
|---|---|---|---|---|---|
| 1 (residential) | 1 | 2 | 10 | 0 | yes |
| 2 (commercial) | −2 | −5 | 0 | 0 | yes |
| 3 (industrial) | −1 | −1 | −5 | 20 | **no** |
| other | 0 | 0 | 0 | 0 | yes |

For each candidate, in list order, every operation is float32:

```
score := 0
if type ≠ 1:
    FindExistingCapacity(c)
    score := r24( r24( r24(float(abandoned) × demolition[1]) + r24(float(existing) × demolition[0]) ) + 0 )
if lot costs:
    type 3: score += r24(float(uint32(|lotSet| − 1)) × newLot[0])     -- then r24
    type 4: score += newLot[1]                                        -- then r24
if wL ≠ 0:                                     -- the guard is the RAIL weight: commercial gets no network terms
    side 0:      s := r24( r24( r24(avenue[0]·wA) + r24(rail[0]·wL) ) + r24(road[0]·wR) );  score := r24(score + s)
    sides 1..3:  t := r24( r24( r24(road[j]·wR) + r24(avenue[j]·wA) ) + r24(rail[j]·wL) );  score := r24(score + t)
if wX ≠ 0:
    score := r24(score + r24(rng.RandomDoubleUniform() × 20))            -- file-local generator, §4.22
```

- Streets are never weighted.
- **Lower is better.**
- Network counts and weights are integers converted exactly.

**Sort.** The candidate list (a vector of record pointers) is sorted ascending by score with the game's
`std::sort`:
- STLport introsort with depth limit `2·⌊log₂ n⌋`;
- unguarded insertion sort with threshold 16;
- comparator `0x00704770`: `a.score < b.score`, false when unordered.

This is the same algorithm as the growth developer's list sort (`growth-developer.md` §5.4). Ties (common: most
residential candidates on an empty tract score 0) keep the order this exact algorithm produces. FreeCity4 must use
the same algorithm *[compat]*.

### 4.15 Building

#### 4.15.1 `Build(c)` → code (`0x0070B440`, pop 4) *[compat]*

```
r := PickBuilding(c);  if r ≠ 27: return r                                   -- §4.16
cap := BDS.GetBuildingProfile(chosenBuilding).capacity
if not BDS.ClaimConstructionResources(cap) (slot 50): hist[6][20] += 1; return 20
switch c.type:
  4: BDS.ReleaseConstructionResources(cap) (slot 51); PerformSubdivision(c) (§4.15.4); return 27
  3: lot := PerformAggregation(c) (§4.15.3)
  2: lot := c.lot (AddRef); erase it from the suitability cache;
     demolition.DemolishLots(true, [lot], 0, 0x800, false, 0, 0, 0) (demolition slot 7);  built −= c.existing
  else: lot := null
if lot is null: hist[6][0] += 1; return 0            -- the claimed resources are NOT released [quirk]
lotDeveloper.StartConstruction(lot, chosenConfiguration, chosenBuilding, true)   -- slot 6
built += cap; release lot; return 27                  -- no histogram entry for success
```

- The capacity claimed and added to `built` is the building profile's capacity, not the configuration's.
- For type 4, the claim only checks that resources are available: the subdivision claims per piece.
- Type 1 never reaches `Build`; it is handled in §4.4. A type-1 record that reached it would fail with 0.

#### 4.15.2 `BuildFarm(c)` → code (`0x0070B640`, pop 4) *[compat]*

Purpose 5 only. Instead of `Build`:

```
near := []                                                        -- list (B)
for f in (1, 2, 4, 8): lotManager.GetLotsSurroundingRect(c.rect, near, f)     -- slot 10, appends
fields := []
for each L in near (list order; the list grows while walked):
    if L overlaps c.rect: skip
    r := cached or inline IsLotSuitable(L, true)
    if r ∈ {27, 21} and L ∉ fields:
        append L to fields (no AddRef)
        for f in (1, 2, 4, 8): lotManager.GetLotsSurroundingLot(L, near, f)     -- slot 9, appends
n := |fields|
if n < 15: return 0
if not Claim(n + 30): return 0
Release(30)
r := Build(c);  if r ≠ 27: Release(n); return r
ids := property 0x29B55F73 of the chosen building's exemplar (uint32 array)
if present: k := rng.RandomUint32Uniform(count(ids)); id := ids[k]      -- file-local generator; count 0 reads ids[0]
fieldCap := BDS.GetBuildingProfile(id).capacity
fieldCfg := lotConfigurationManager.GetLotConfiguration(id) (slot 8)
if fieldCfg is null: Release(n)
else for each L in fields (order of discovery):
    L.SetFacing(c.facing) (slot 24)
    lotDeveloper.StartConstruction(L, fieldCfg, id, false)
    erase L from the suitability cache
    L.SetState(3, 0, 0) (slot 34)
    L.SetPopulation(0x4100, fieldCap) (slot 55)        -- 0x4100 = I-Agriculture
return r
```

- The farm keeps `n` units of construction resources claimed for its fields. These are released when the field
  lots leave state 2 (`building-development.md` §4.6).
- The same ID `id` is used as a building type (`GetBuildingProfile`) **and** as a lot configuration ID
  *[inferred: the shipped field exemplars share the ID]*.

#### 4.15.3 `PerformAggregation(c, lot&)` → bool (`0x007068E0`, pop 8, AL)

1. For each L in `c.lotSet`:
   - AddRef it and erase it from the suitability cache;
   - `DemolishLots(true, [L], …)` with the same arguments as §4.15.1;
   - lot manager slot 24 `DeleteLot(L)`;
   - release it.
2. `built −= c.existing`; `lot := null`.
3. `facing := c.facing`. If the purpose is 5–8 and the chosen configuration is square, `facing := −1`.
4. Lot manager slot 22 `CreateLotIfPossible(x1, z1, w, h, facing, &lot, 0, 0, 0)` on `c.rect`. With facing −1 this
   costs one `U(4)` draw on the zone developer's generator (R9, zones.md).
5. Terrain slot 68 `RedisplayTerrain(0, 0, &rect, 0)`.
6. Return `lot ≠ null`.

#### 4.15.4 `PerformSubdivision(c)` (`0x0070A9B0`, pop 4) *[compat]*

Piece size, origin offset and step by facing (`(cw, ch)` = configuration size, origin = the parent lot's location):

| Facing | Piece `(w', h')` | Origin offset | Step per piece |
|---|---|---|---|
| 0 | `(cw, ch)` | `x += 1 − cw` | `x −= cw` |
| 1 | `(ch, cw)` | `x += 1 − ch`, `z += 1 − cw` | `z −= cw` |
| 2 | `(cw, ch)` | `z += 1 − ch` | `x += cw` |
| 3 | `(ch, cw)` | none | `z += cw` |

```
erase the parent from the suitability cache; demolish it (as §4.15.1); built −= c.existing; DeleteLot(parent)
target := desiredMax − built + c.existing;  if target = 0: target := 1
for i in 0 … n−1:
    lot := CreateLotIfPossible(origin, (w', h'), c.facing, …)
    if lot and built < target:
        ok := (i = 0) or PickBuilding(c) = 27          -- new draws for pieces 2…n; the parent no longer exists
        if ok and Claim(cap of chosenBuilding):
            StartConstruction(lot, chosenConfiguration, chosenBuilding, true); built += cap
    release lot;  step the origin
```

- Pieces that are not built stay as empty zoned lots.
- Piece `i ≥ 1` repeats the whole `PickBuilding` on the same candidate. Its `FindExistingCapacity` is cached, so
  the capacity window is unchanged (§4.16 step 7).

### 4.16 `PickBuilding(c)` → code (`0x00709BA0`, pop 4) *[compat]*

```
1  FindExistingCapacity(c)
2  group := 0
   if BDS.GetLotDeveloper() (slot 36).NeedsAnchorBuilding(&c.rect, purpose) (lot developer slot 36): group := 0x3000
3  else if developer.GetStylePercentages() (slot 26) is non-empty:            -- map<uint32 key, float pct>
       r := rng.RandomUint32Uniform(100)                                        -- DRAW A
       for (key, pct) in the map in ascending key order:
           p := rhe(r24(pct × 100))   (uint32)
           if r < p: group := key; break
           r −= p
       (no match → group stays 0)
4  S_cfg := {}                                                                  -- local set, pointer order
   for cfg in the configuration set:
       if group ≠ 0 and not cfg.IsCompatibleWithStyleType(group) (slot 18): hist[10][9] += 1; continue
       if IsLotConfigurationCompatible(cfg, c, c.corner) = 27: insert cfg into S_cfg
5  if S_cfg is empty: return 0
6  k := rng.RandomUint32Uniform(|S_cfg|)                                        -- DRAW B
   chosenConfiguration := the k-th element of S_cfg (pointer order)
7  lo := c.existing + 1 (if this overflows, it stays c.existing);  if c.type = 4: lo := lo / c.n (signed)
   hi := max(desiredMax − built + c.existing, lo + 1)
8  S27 := {}, S2 := {}      -- STLport hash_set<uint32>, 193 buckets
   for each id in chosenConfiguration.GetPrimaryBuildingConfigurationObject()'s object-ID list (in order):
       T := {} (hash_set<uint32>, 193 buckets);  BDS.GetBuildingTypesByFamily(T, id) (slot 19)
       for b in T (hash order):
           if group ≠ 0 and not IsOccupantGroup(BDS.GetBuildingKeyFromType(b) (slot 14), group): hist[10][9] += 1; continue
           r := IsBuildingCompatible(b, c) (§4.17)
           if r = 27: insert b into S27
           else if r = 2 and developer.GetMonopolyUse() (slot 25) = 1: insert b into S2
9  if S27 is empty:
       if S2 is empty: return r (the last IsBuildingCompatible result; 0 if none ran)
       S27 := S2
10 m := rng.RandomUint32Uniform(|S27|)                                          -- DRAW C
   chosenBuilding := the m-th element of S27 in hash iteration order
   return 27
```

- `IsOccupantGroup` (`0x005E8F70`) reads the building exemplar's occupant groups.
- `0x3000` is the occupant group the anchor building must belong to *[inferred: the "anchor" landmark / mixed-use
  group]*.

**Hash iteration order** of an STLport `hash_set<uint32>` (identity hash):
- buckets in ascending order of `id mod B`;
- within a bucket, the most recently inserted element first (insertion at the chain front, `0x00624DD0`).

`B` is 193 while the set holds at most 193 elements. On growth it resizes to the next prime of the table at
`0x00A829B0` (53, 97, 193, 389, …). The old buckets are walked in ascending order, moving each chain's front node to
the front of its new bucket. This order is part of the contract, because draw C indexes into it.

### 4.17 `IsBuildingCompatible(b, c)` → code (`0x00704E00`, pop 8) *[compat]*

`P := BDS.GetBuildingProfile(b)`. `hist[12][code] += 1`.

| # | Check | Code |
|---|---|---|
| 1 | `P.purpose ≠ purpose` | 7 |
| 2 | `P.wealth ≠ wealth` | 8 |
| 3 | Purposes 1–3 with non-empty `activeStyles`: the building must be in occupant group `activeStyles[currentStyleIndex]`. Only the **current** style is checked, whatever `singleStyle` says *[quirk]* | 9 |
| 4 | `P.capacity < lo` (§4.16 step 7) | 11 |
| 5 | `P.capacity > hi` | 13 |
| 6 | `monopoly :=` bool property `0x2781283F` of the building exemplar (loaded through the resource manager; absent = false). `use := developer.GetMonopolyUse()`. `use < 0` → 27. `use ∈ {0, 1}` and `monopoly` → 2. `use = 2` and not `monopoly` → 2 | 2 |
| | otherwise | 27 |

### 4.18 `UpdatePopulation(profile)` (slot 7, `0x00706BC0`, pop 4) *[compat]*

```
s := the desirability map's tract shift (growth-developer slot 13 grid)
L := lotManager.GetLotsInCellRect(list, tx<<s, tz<<s, ((tx+1)<<s)−1, ((tz+1)<<s)−1, true)   -- slot 7; list (B)
if L is empty: return
d  := desirabilityMap.GetTractValue(tx, tz)          (int16)
dS := r24(float(d) × 0.01f)                           (dword constant 0x3C23D70A)
devType := developer.GetSubType();  wealth := developer.GetWealth()
g := cRZRandom(seed = ((tx << 16) + tz) << 4)        -- temporary generator, §4.22
for each lot in L (list order):
    skip unless lot.HasCapacity() and lot.GetOccupantWealth() = wealth
    maxCap := lot.GetAvailableCapacity(devType, false)  (uint16);  skip if 0
    j   := g.RandomDoubleRangeUniform(−variation, +variation)     -- variation = tuning 0x47E2C530, as doubles
    occ := r24( r24(j + 1.0) × dS )
    if occ < 0: occ := 0
    else if not (occ < 1): occ := r24(1.0 − g.RandomDoubleRangeUniform(minVacancy[0], minVacancy[1]))
    pop := low 16 bits of rhe(r24(float(maxCap) × occ))
    lot.SetPopulation(devType, pop)                     -- slot 55
release every lot in L; free the list
```

- NaN `occ` is kept: it compares as "< 1". Its pop is the low half of the NaN pattern, which is 0.
- The profile's stage field is not read.
- The return value is EAX garbage. The growth developer ignores it.
- Each lot draws once, or twice when it would be full, in list order.
- *[quirk]* Tract (0, 0) seeds 0. `RandomDoubleUniform` with state 0 reseeds from the process clock
  (foundations §6.1), so tract (0, 0) is not reproducible. *[choice]* FreeCity4 uses a fixed nonzero substitute
  (`0x00000001`) for that tract only, and documents the divergence.

### 4.19 `Decline(profile)` → int32 (slot 6, `0x00709200`, pop 4) *[compat]*

- A class-wide random cell iterator (`0x00B4B0CC`) in prime-step mode walks the tract's cells. `Reset(true)` makes
  one `rand()` call.
- For each cell, `L := GetLot(x, z, false)`. L qualifies when all of these hold:
  - `GetState() = 3`;
  - `GetDaysSinceLastStateChange() ≥ 60` (unsigned);
  - `GetBuildingType(false) ≠ 0`;
  - the building profile's purpose is the developer's purpose;
  - `GetOccupantWealth()` is the developer's wealth.
- For a qualifying lot: `sum += L.GetCapacity(devType, false)`, then `lotDeveloper.AbandonBuilding(L, 6)`
  (slot 8).
- Returns `sum` (EAX).
- A multi-cell lot is visited once per cell. After the first abandonment its state is no longer 3, so it is
  counted once (AbandonBuilding sets the state synchronously; lot-developer.md).
- The caller uses the result as described in `building-development.md` §4.5.

### 4.20 Styles

**`NewYear`** (Year message):

```
yearsSinceStyleChange += 1
if yearsSinceStyleChange ≥ yearsBetweenStyleChanges (signed):
    currentStyleIndex += 1;  yearsSinceStyleChange := 0
    if currentStyleIndex ≥ |activeStyles| (unsigned): currentStyleIndex := 0
```

**Interface accessors**

| Method | Behaviour |
|---|---|
| `GetCurrentStyle` | 0 if `activeStyles` is empty, else `activeStyles[currentStyleIndex]` |
| `SetActiveStyles(v)` | `idx := (old index < |v|) ? old index : 0`. Then, with `cur := old activeStyles[old index]` (no bounds check), the **last** `i` with `v[i] = cur` becomes `idx`. Copy `v` (B), set the index. `yearsSinceStyleChange` is not reset |
| `UseAllStylesAtOnce(f)` | `singleStyle := not f` |
| `IsUsingAllStylesAtOnce` | `not singleStyle` |
| `SetYearsBetweenStyleChanges(n)` | stores `n` |

- *[quirk]* `SetActiveStyles` on an empty current list reads element 0 of an empty vector. FreeCity4 treats `cur`
  as "none" *[choice]*.
- The style IDs are occupant groups (shipped `0x2000`–`0x2003`).

### 4.21 Tuning (`ReadTuning` `0x00706FC0`) *[compat]*

- Exemplar: `{type 0x6534284A, group 0xE7E2C2DB, instance 0xE8DA7677}` ("Building Development"), loaded through the
  resource manager (`cISCResExemplar`, IID `0xA52160F5`). Floats are read with `0x005FD4F0`.
- **Single values** have their default written first. Missing → default.
- **Pairs**: when the property exists, its first two floats are copied **without a count check** *[quirk]*.

| Property | Name (re/tuning) | Code default | Shipped | Use |
|---|---|---|---|---|
| `0x47E2C500` | Overbuild | 1.5 | 1.5 | `desiredMax` (§4.2) |
| `0x47E2C501` | Reoccupy Thresholds | {0, 1.2} | {0, 1.2} | §4.4 |
| `0x47E2C502` | Redevelop Size Threshold | 1.5 | 1.5 | read, not used by any traced path *[inferred]* |
| `0x47E2C510` | Demolition Costs | {1, 0.2} | {1, 0.2} | §4.14: [0] × existing, [1] × abandoned |
| `0x47E2C511` | New Lot Costs | {10, 5} | {1, 1} | §4.14: [0] per extra aggregated lot, [1] per subdivision |
| `0x47E2C530` | Occupancy Variation | 0.2 | 0 | §4.18 |
| `0x47E2C531` | Min Vacancy Range | {0.01, 0.1} (constructor {0.1, 0.1}) | {0.01, 0.1} | §4.18 |
| `0x47E2C540` | Kick Out Lower Wealth | true (bool, `0x005FD390`) | true | §4.4, §4.8 |
| `0x47E2C550` | Available Styles | empty | 8192–8195 | §2.1. The list is cleared first, then filled when present |
| `0x47E2C551` | Years Between Styles | 40 (only when the field is 0) | 5 | §2.1 (int32, `0x005FD450`) |

- After reading: if `activeStyles` is empty, `currentStyleIndex := 0` and `activeStyles := availableStyles`.
- If `currentStyleIndex ≥ |activeStyles|`, it is set to 0.
- Because `Read` precedes `Init`:
  - a loaded city keeps its saved styles and years;
  - a new city gets the shipped 8192–8195 rotating every 5 years.

### 4.22 Random draws *[compat]*

| # | Operation | Generator | Method (state advances) | Arguments | Position |
|---|---|---|---|---|---|
| R1 | `Grow` → `ListExistingLots` (§4.5), every pass | C `rand()` (foundations §12) | 1 call (iterator `Reset(true)`) | start = `rand() mod cellCount` | first draw of each pass |
| R2 | `ScoreCandidateLots` (§4.14), industrial (zone type 3) only | file-local `cRZRandom` `0x00B4AFD8` | `RandomDoubleUniform` (`0x0091372E`): 1 multiplicative step | — | one per candidate, in list order, after its deterministic terms; once in `TakeOverAbandoned` (if it scores) and once per pass with candidates |
| R3 | `PickBuilding` draw A (§4.16 step 3) | file-local | `RandomUint32Uniform(100)` (`0x009136B1`): 1 LCG step | 100 | only when no anchor is needed and the style map is non-empty |
| R4 | `PickBuilding` draw B (step 6) | file-local | `RandomUint32Uniform(|S_cfg|)` | `|S_cfg| ≥ 1` | only when `S_cfg` is non-empty |
| R5 | `PickBuilding` draw C (step 10) | file-local | `RandomUint32Uniform(|S27|)` | `|S27| ≥ 1` | only when a building qualified |
| R6 | `BuildFarm` field type (§4.15.2) | file-local | `RandomUint32Uniform(count)` | count of `0x29B55F73` (0 → no advance) | after `Build` succeeded, before the fields |
| R7 | `UpdatePopulation` (§4.18) | temporary `cRZRandom`, seed `((tx<<16)+tz)<<4` | `RandomDoubleRangeUniform(a, b)` (`0x00913761`): 1 multiplicative step | `(−var, +var)`, then `(minVac0, minVac1)` when full | per lot in list order: 1 or 2 draws |
| R8 | `Decline` (§4.19) | C `rand()` | 1 call (iterator `Reset(true)`) | | start of the call |
| R9 | `PerformAggregation` (§4.15.3) with facing −1, inside the lot manager's `CreateLotIfPossible` → `CreateLotToFaceRoad` | the **zone developer's** file-local `cRZRandom` `0x00B4B398` (zones.md) | `RandomUint32Uniform(4)`: 1 LCG step | 4 | once per such call, on entry, before the lot is made |

**Order within one `Grow`, per pass:**
1. R1.
2. R2 for the takeover candidates, if any were scored.
3. R2 for each listed candidate.
4. For each candidate tried in sorted order: R3 / R4 / R5 in `PickBuilding`, then (farms) R6.
5. Subdivision pieces 2…n each repeat R3–R5.

`PickBuilding` returns early without the later draws when `S_cfg` is empty (no R4, R5) or no building qualifies
(no R5).

**The file-local generator** (`0x00B4AFD8`):
- It is a **`cRZRandom`**, not a `cRZRandomSuper` (its constructor `0x00913A5C` calls `Seed` `0x00913667`).
- It is seeded once at process start by the static initialiser `0x00A74E80` with
  `cRZTime().<first field> XOR 0x4C09C113` (`cRZTime` `0x00470B50`, local time). **Clock-seeded.**
- It is never reseeded or saved.
- *[choice]* FreeCity4 seeds it from the session seed (foundations §6.3), mixed with `0x4C09C113`, and does not save
  it.
- Cross-run reproduction is statistical, as for the original.

`RandomUint32Uniform(n) = (NextRaw × n) >> 32` with `NextRaw` the LCG (`× 0x41C64E6D + 0x3039`, high 32 of the
product per foundations §6.1). `n = 0` returns 0 without advancing. The `RandomDouble*` methods use the separate
multiplicative state (`× 0x278DDE6D`).

---

## 5. Messages posted

None directly. The visible effects (construction, abandonment, reoccupation, lot creation and deletion, population)
go through the lot developer, the lot manager, the demolition utility and the lots. Their messages are specified
there.

---

## 6. Persistence (`cIGZSerializable`, sub-object +8, class ID `0x2990C142`) *[compat]*

**Write** (`0x00704B60`), little-endian, `cIGZOStream`:

| Field | Type | Value |
|---|---|---|
| version | uint16 | 3 |
| currentStyleIndex | int32 | |
| yearsSinceStyleChange | int32 | |
| singleStyle | uint8 | 0 / 1 |
| count | uint32 | `|activeStyles|` |
| styles | count × uint32 | |
| yearsBetweenStyleChanges | int32 | |

Returns `GetError() = 0`.

**Read** (`0x00707950`):
1. version > 3 → false.
2. Read the index and the years-since field.
3. If version ≥ 2: the flag (bool), the count, then `activeStyles` resized to count (zero-filled) and filled.
   Else: `singleStyle := true`, `activeStyles := {0x2000, 0x2001, 0x2002}`.
4. If version ≥ 3: `yearsBetweenStyleChanges`.
5. Returns `GetError() = 0`.

- *[quirk]* If the count read fails, the uninitialised local count is used for the resize. FreeCity4 uses 0
  *[choice]*.
- No grid, lot or candidate state is saved.
- On load, `Init` (which runs after `Read`) fills an empty style list from the tuning and clamps the index (§4.21).

---

## 7. Interfaces *[compat]*

All slots are `__thiscall`. Pops were checked with `abi_extract.py --check`; returns were read from the epilogues.

### 7.1 `cISC4TractDeveloper` (vtable `0x00AB3118`, 15 interface slots + slot 15 destructor)

| Slot | Off | As compiled | Pop | Return | Address | SDK |
|---|---|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface(iid, void**)` | 8 | AL | `0x007045B0` | ok |
| 1 | +0x04 | `AddRef` | 0 | EAX | `0x004F7BA0` | ok |
| 2 | +0x08 | `Release` | 0 | EAX | `0x007048B0` | ok |
| 3 | +0x0C | `bool Init()` | 0 | AL | `0x00707780` | ok |
| 4 | +0x10 | `bool Shutdown()` | 0 | AL (1) | `0x0070B210` | ok |
| 5 | +0x14 | `int32 Grow(GrowthProfile const&, std::list<Outcome>&, bool&)` | 12 | **EAX = built (int32)** | `0x0070D130` | **error**: SDK `bool`. A count that is a multiple of 256 reads as false, and negative values read as true |
| 6 | +0x18 | `int32 Decline(GrowthProfile const&)` | 4 | **EAX = summed capacity** | `0x00709200` | **error**: SDK `bool` |
| 7 | +0x1C | `void UpdatePopulation(GrowthProfile const&)` | 4 | **none** (EAX garbage) | `0x00706BC0` | **error**: SDK `bool` |
| 8 | +0x20 | `int32 GetCurrentStyle() const` | 0 | EAX | `0x0070C8D0` | ok |
| 9 | +0x24 | `vector<uint32>& GetActiveStyles()` | 0 | EAX | `0x0070C840` | ok in shape. The vector is an MSVC/STLport `std::vector` (B), not an `SC4Vector` of the service allocator (§9.2) |
| 10 | +0x28 | `SetActiveStyles(vector<uint32> const&)` | 4 | **EAX = address of the member vector** (from the assignment) | `0x00707470` | **error**: SDK `bool` (always "true" by accident) |
| 11 | +0x2C | `bool IsUsingAllStylesAtOnce() const` | 0 | AL | `0x0070C850` | ok |
| 12 | +0x30 | `UseAllStylesAtOnce(bool)` | 4 | **AL = new `singleStyle`**, upper EAX garbage | `0x0070C860` | **error**: SDK returns `cISC4TractDeveloper*` |
| 13 | +0x34 | `int32 GetYearsBetweenStyleChanges() const` | 0 | EAX | `0x00407BA0` (shared getter) | ok |
| 14 | +0x38 | `SetYearsBetweenStyleChanges(int32)` | 4 | **EAX = n** | `0x0070C880` | **error**: SDK returns `cISC4TractDeveloper*` |
| 15 | +0x3C | scalar deleting destructor | 4 | EAX | `0x0070CA80` | not in the interface |

**What a replacement returns where the original returns garbage or something unexpected:**
- slot 5: `built` (callers test `> 0`);
- slot 6: the sum;
- slot 7: 0;
- slot 10: nonzero (any game caller reading AL sees nonzero today);
- slot 12: `this`;
- slot 14: `this`.

No game caller reads slots 10, 12 or 14 *[inferred: no call sites found outside the UI]*.

`GrowthProfile` as read: `{cISC4GrowthDeveloper* developer, int32 x, int32 z, stage}`. Only the low byte of the
stage field is used, by `Grow`.

### 7.2 `cIGZMessageTarget2` (vtable `0x00AB3104`, sub-object +4)

| Slot | Method | Pop | Return | Address |
|---|---|---|---|---|
| 0 | QueryInterface (adjusts to the primary) | 8 | AL | `0x0070C890` |
| 1 | AddRef | 0 | EAX | `0x0068D230` |
| 2 | Release | 0 | EAX | `0x0070C8B0` |
| 3 | `bool DoMessage(cIGZMessage2*)` | 4 | AL (1) | `0x007048E0` |

### 7.3 `cIGZSerializable` (vtable `0x00AB30EC`, sub-object +8)

| Slot | Method | Pop | Return | Address |
|---|---|---|---|---|
| 0 | QueryInterface | 8 | AL | `0x0070C8A0` |
| 1 | AddRef | 0 | EAX | `0x0068D250` |
| 2 | Release | 0 | EAX | `0x0070C8C0` |
| 3 | `bool Write(cIGZOStream&)` | 4 | AL | `0x00704B60` |
| 4 | `bool Read(cIGZIStream&)` | 4 | AL | `0x00707950` |
| 5 | `uint32 GetGZCLSID()` → `0x2990C142` | 0 | EAX | `0x007045A0` |

### 7.4 The configuration filter (`cISC4LotConfigurationFilter`, vtable `0x00AB30DC`, 4 slots)

| Slot | Method | Pop | Return | Address |
|---|---|---|---|---|
| 0 | QueryInterface → false | 8 | AL | `0x0062C980` |
| 1 | AddRef → 1 | 0 | EAX | `0x008CD450` |
| 2 | Release → 1 | 0 | EAX | `0x008CD450` |
| 3 | `bool IsLotConfigurationIncluded(cISC4LotConfiguration*)` | 4 | AL | `0x00704950` |

The filter is **not reference counted**. The tract developer allocates it (C), registers it, unregisters it at
`Shutdown` and frees it (C).

---

## 8. Dependencies *[compat]*

All are `__thiscall`. "Owner" is the spec that owns the callee's behaviour.

| Interface (vtable) | Slot / off | Call as used | Pop | Return | Owner |
|---|---|---|---|---|---|
| `cISC4GrowthDeveloper` (`0x00AB1488`) | 5 `+0x14` | GetType → target zone type | 0 | EAX | growth-developer |
| | 6 `+0x18` | GetSubType → developer type | 0 | EAX | |
| | 7 / 8 `+0x1C/+0x20` | GetPurpose / GetWealth | 0 | EAX | |
| | 9 `+0x24` | GetConstructionInfo(profile&) | 4 | none | |
| | 13 / 14 `+0x34/+0x38` | GetDesirabilityMap / GetCapacityMap | 0 | EAX | |
| | 18 `+0x48` | GetSourceDemand | 0 | EAX | |
| | 19–22 `+0x4C…+0x58` | power / water / fire / park caps | 0 | AL | |
| | 25 `+0x64` | GetMonopolyUse | 0 | EAX | |
| | 26 `+0x68` | GetStylePercentages → `map<uint32, float>*` (B) | 0 | EAX | |
| `cISC4Demand` (`0x00AA2758`) | 12 `+0x30` | demand value (float) | 0 | ST0 | demand |
| `cISC4SimGrid<T>` | tract value / tract size / shift, `GetCellValue` | per simgrid §5 | | | simgrid |
| `cISC4BuildingDevelopmentSimulator` (`0x00AB0AF8`) | 14 `+0x38` | GetBuildingKeyFromType(type, key&) | 8 | AL | building-development |
| | 19 `+0x4C` | GetBuildingTypesByFamily(hash_set&, id) — inserts | 8 | EAX count | |
| | 33 `+0x84` | GetDeveloper(purpose, wealth) | 8 | EAX, no AddRef | |
| | 36 `+0x90` | GetLotDeveloper | 0 | EAX, no AddRef | |
| | 39 `+0x9C` | GetBuildingProfile(type) → `BuildingProfile&` (inserts zero profile if unknown) | 4 | EAX | |
| | 50 / 51 `+0xC8/+0xCC` | Claim / ReleaseConstructionResources(uint16) | 4 | AL | |
| | 54 / 55 / 56 / 57 | CheatIgnorePlumbing / Power / Caps / DebugGrowth | 0 | AL | |
| BDS helpers (non-virtual) | `0x00695390` | BuildingFitsInZone(zoneCode, zone) (cdecl, 2 args) | caller | AL | building-development |
| | | GrowthStageFitsInZone (cdecl) | caller | AL | |
| `cISC4LotDeveloper` (`0x00AB1B98`) | 6 `+0x18` | StartConstruction(lot, cfg, buildingType, bool) | 16 | AL | lot-developer |
| | 8 `+0x20` | AbandonBuilding(lot, reason) | 8 | AL | |
| | 9 `+0x24` | ReoccupyBuilding(lot, devType) | 8 | AL | |
| | 29 `+0x74` | GetAreaSlope(rect*, 0) → float | 8 | ST0 | |
| | 36 `+0x90` | NeedsAnchorBuilding(rect*, purpose) | 8 | AL | |
| `cISC4LotManager` (`0x00AB37C0`) | 6 `+0x18` | GetLot(x, z, bool) | 12 | EAX, no AddRef | zones |
| | 7 `+0x1C` | GetLotsInCellRect(list&, x1, z1, x2, z2, bool) — AddRef'd values | 24 | | |
| | 9 `+0x24` | GetLotsSurroundingLot(lot, list&, flags) — appends | 12 | | |
| | 10 `+0x28` | GetLotsSurroundingRect(rect&, list&, flags) — appends | 12 | | |
| | 20 `+0x50` | CanCreateLot(x, z, w, h, 0, 0, 0) → code | 28 | EAX | |
| | 22 `+0x58` | CreateLotIfPossible(x, z, w, h, facing, cISC4Lot*& out, bool, bool, cfg) | 36 | AL | |
| | 24 `+0x60` | DeleteLot(lot) | 4 | AL | |
| `cISC4Lot` (`0x00AB19B0`) | 14 `+0x38` | GetBoundingRect(rect&) | 4 | AL | zones / lot |
| | 18 `+0x48` | GetSlope → float | 0 | ST0 | |
| | 21 / 22 / 23 / 24 | GetLocation(x&, z&) / GetSize(w&, h&) / GetFacing / SetFacing(f) | 8 / 8 / 0 / 4 | | |
| | 28 / 29 | HasRoadAccess / HasOrganicRoadAccess(purpose) | 0 / 4 | AL | |
| | 31 | GetZoneType | 0 | EAX | |
| | 33 / 34 | GetState / SetState(state, wealth, reason) | 0 / 12 | | |
| | 37 / 38 / 40 / 41 / 43 | IsInLivableState / HasCapacity / GetDaysSinceLastStateChange / GetOccupantWealth / IsHistorical | 0 | | |
| | 46 | GetBuildingType(bool) | 4 | EAX | |
| | 50 | **GetCapacity(DeveloperType, bool)** — compiled before the wealth overload (foundations §16) | 8 | AX | |
| | 51 | GetCapacity(WealthType) | 4 | AX | |
| | 52 | GetAvailableCapacity(DeveloperType, bool) | 8 | AX | |
| | 55 | SetPopulation(DeveloperType, uint16) | 8 | AL | |
| `cISC4LotConfiguration` (`0x00A8CEB8`) | 7, 8, 9, 11, 12, 13, 15, 16, 17, 18, 21, 24 | GetSize(w&, h&) (8), GetMaximumSlope (ST0), GetMinimumSlope (ST0), GetGrowthStage (AL), GetMin/MaxBuildingCapacity (AX), IsCompatibleWithZoneType / WealthType / PurposeType / StyleType (4, AL), GetRequiredRoads(vector<bool>&) (4), GetPrimaryBuildingConfigurationObject (EAX) | | | lot configuration (no spec yet) |
| `cISC4LotConfigurationManager` (`0x00A8EC58`) | 8 `+0x20` | GetLotConfiguration(id) | 4 | EAX | |
| | 9 `+0x24` | GetLotConfigurationsByFilter(hash_set&, filter) → count; each config AddRef'd | 8 | EAX | |
| | 18 / 19 `+0x48/+0x4C` | Register / UnregisterLotConfigurationFilter(filter) | 4 | AL | |
| `cISC4ZoneManager` | 12 `+0x30` | zone type at (x, z) | 8 | AL (signed byte) | zones |
| `cISC4LandValueSimulator` | 6 `+0x18` | GetLandValue(x, z) | 8 | AL | landvalue |
| `cISC4PowerSimulator` | 35 `+0x8C` | GetPoweredGrid | 0 | EAX | power |
| `cISC4PlumbingSimulator` | 11 `+0x2C` | IsCellWatered(x, z) | 8 | AL | plumbing |
| `cISC4FireProtectionSimulator` | 5 `+0x14` | fire protection grid (uint8) | 0 | EAX | protection |
| `cISC4AuraSimulator` | 14 `+0x38` | GetParkMap (int16) | 0 | EAX | aura |
| `cISC4City` | 31, 40, 84, 93, 94 | GetDemolitionUtility, GetTrafficNetwork, GetTerrain, cell counts X / Z | 0 | EAX | city |
| `cISC4TrafficNetworkMap` (`0x00A8FED8`) | 8 `+0x20` | FindNetworkTile(x, z, mask, flag) | 16 | EAX | landfill / traffic |
| `cISC4NetworkOccupant` (`0x00AA8080`) | 22 `+0x58` | IsOfType(type) (maps through the table at `0x00AA7CE8`, calls slot 8) | 4 | AL | |
| `cISC4Demolition` (`0x00A88004`) | 7 `+0x1C` | DemolishLots(bool, list&, …) — 8 dword args | 32 | AL | |
| `cISTETerrain` (`0x00AB3D18`) | 68 | RedisplayTerrain(0, 0, rect*, 0) | 16 | | landvalue / terrain |
| `cISC4Simulator` | 19 / 20 | AddAgent / RemoveAgent | | | simulator |
| cRZRandom (non-virtual) | `0x009136B1`, `0x0091372E`, `0x00913761`, ctor `0x00913A5C` | | 4 / 0 / 16 / 4 | EAX / ST0 / ST0 | foundations §6 |
| Random iterator (non-virtual) | `0x0049D1C0` Reset, `0x0049CB10` GetNext | | | | weather §6.7 |
| Building exemplar property helpers | `0x005E8F70` IsOccupantGroup | | | AL | |

---

## 9. Replaceability

### 9.1 Who reaches into the tract developer

- Only through `cISC4TractDeveloper`:
  - the BDS: slots 5, 6 and 35 of its own interface;
  - the growth developer: slot 7.
- The UI reads and writes the style state through slots 8–14 *[inferred]*.
- No code outside the class reads its members.
- The class-wide statics (candidate pool, the two random iterators, the file-local generator, the "configured" flag)
  are private to the class's translation unit.

The class is **replaceable through the GZCOM class registry** (CLSID `0x2990C142`) provided the replacement:
- implements the compiled slot table (§7.1) with the compiled return channels, not the SDK's;
- registers its configuration filter with the game's lot configuration manager, and unregisters it before freeing
  it. The manager keeps the raw pointer;
- uses allocator B for every container it hands to, or receives from, the game (§9.2).

### 9.2 Containers crossing an interface (allocators.md format)

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence |
|---|---|---|---|---|---|
| `cISC4TractDeveloper::Grow` (5) | `std::list<Outcome>` | B (nodes of 12 bytes) | no (SDK `SC4List` uses the service A) | caller (BDS) creates and frees; tract developer **appends** int32 codes, never clears | `0x0070D557`, `0x0070D65A`, `0x0070D727` |
| `cISC4TractDeveloper::GetActiveStyles` (9) / `SetActiveStyles` (10) | `std::vector<uint32>` | B | no | Get returns the member by reference; Set assigns (`0x00462C10`) | `0x00707470` |
| `cISC4LotConfigurationManager::GetLotConfigurationsByFilter` (9) | STLport `hash_set<cISC4LotConfiguration*>` | B | no | filled by the manager, not cleared first; one reference per element, the caller releases each | caller `0x00709650`, `0x0048B1A0` |
| `cISC4BuildingDevelopmentSimulator::GetBuildingTypesByFamily` (19) | STLport `hash_set<uint32>` | B | no | caller constructs (193 buckets), BDS inserts | `0x00709FEC` |
| `cISC4LotManager::GetLotsInCellRect` (7) | `std::list<cISC4Lot*>` | B | no | values AddRef'd by the callee; the caller releases each and frees the nodes (B) | `0x00706C41`, `0x00706DD0` |
| `cISC4LotManager::GetLotsSurroundingRect` (10) / `GetLotsSurroundingLot` (9) | `std::list<cISC4Lot*>` | B | no | one list appended across 4 + 4·k calls | `0x0070B693`, `0x0070BA5E` |
| `cISC4Demolition::DemolishLots` (7) | `std::list<cISC4Lot*>` | B (header 12 bytes) | no | built by the caller with one AddRef'd lot; freed by the caller | `0x0070B519`, `0x007069E4`, `0x0070AB59`; list helper `0x007E98C0` |
| `cISC4LotConfiguration::GetRequiredRoads` (21) | `std::vector<bool>` | B | no | callee fills; caller frees the bit buffer with B | `0x00705E64`, `0x00706838` |
| `cISC4GrowthDeveloper::GetStylePercentages` (26) | `std::map<uint32, float>` | B | n/a (read only) | the developer's member map, read in key order | `0x00709C1F` region |

### 9.3 Internal containers (no allocator row)

All of these stay inside the class:
- the lot set and configuration set;
- the candidate list and pool;
- the suitability cache;
- the filter map;
- the debug list and map;
- the histogram;
- the local `S_cfg`, `S27` and `S2` sets of `PickBuilding`;
- the farm's field list.

A replacement may use any containers there. It must still reproduce the **iteration orders** they impose, because
they decide the outcome and the draws (§10.1).

### 9.4 Grids

- The tract developer **owns no grid**. Its reads are all through the owners' interfaces:
  - the growth developer's desirability and capacity maps;
  - the powered grid;
  - the plumbing query;
  - the fire grid;
  - the park map;
  - the traffic simulator's road-access data;
  - land value.
- It does not hold the grids across calls.
- Grid ownership and the raw readers of those grids are their owners' business (simgrid §6.4).

### 9.5 Blockers and requirements for an exact replacement

1. **Pointer-ordered sets** (lot set, configuration set, `S_cfg`).
   - In-game, the order is that of the game objects' addresses. A replacement running in the game process reproduces
     it by ordering on the same pointers.
   - Standalone, FreeCity4 orders by object creation sequence *[choice]*. This changes which configuration draw B
     picks, but not the distribution.
2. **Hash-set order** (`S27`, `S2`): must follow §4.16 exactly.
3. **The SGI/STLport `std::sort`** of the candidates (§4.14).
4. **Clock-seeded generators**: the file-local generator, and tract (0, 0) in `UpdatePopulation`.
5. **The registered filter object** (§7.4).
6. **Allocator B** for all crossing containers (§9.2).

---

## 10. Quirks, open questions, corrections

### 10.1 Quirks (reproduced unless marked *[choice]*)

1. `Grow` returns an int (built capacity, possibly negative), not a bool.
2. The `outFlag` is set when any existing lot is not in state 3 and does not fail with 6 (§4.9.1). No caller reads
   it. Its meaning is unknown.
3. Aggregation counts its zone (6) and slope (10) rejections as code 5 in `best`, and lets NaN slopes pass. The
   existing-lot path rejects NaN slopes.
4. `IsLotConfigurationCompatible` ignores required-road bit 3 when deciding whether roads are required (§4.12).
5. `IsBuildingCompatible` checks only the current style, even in all-styles mode.
6. `Build` keeps the claimed construction resources when no lot results (§4.15.1).
7. Commercial developers get no road / avenue scoring: the guard is the rail weight, which is 0 for commercial
   (§4.14).
8. `ScoreCandidateLots`' side 0 adds the terms in a different order from sides 1–3 (affects rounding only).
9. A pair tuning property shorter than 2 floats is read past its end (§4.21).
10. `SetActiveStyles` with an empty current list reads element 0 of an empty vector.
11. The tract (0, 0) population seed is 0, which means clock seeding.
12. `Decline` visits multi-cell lots once per cell (§4.19).
13. In subdivision, a zero `target` becomes 1.
14. The farm's field-type draw with an empty `0x29B55F73` array reads element 0.

### 10.2 Open questions

1. Q1. *Answered* (`lot-developer.md`, `NeedsAnchorBuilding`): industrial purposes 6–8 need an anchor unless the
   rect already holds a building of occupant group `0x3000` (then true) or a neighbouring lot of the same purpose
   with a building that is past state 1 or is a unit can serve as the anchor (then false).
2. Q2. Is `0x29B55F73` (farm field IDs) always both a building type and a lot configuration ID in shipped data?
3. Q3. The exact semantics of the traffic simulator's road-access value used for purpose 1 (§4.8 row 7) *[inferred]*.
4. Q4. Order in which `GetStylePercentages` values are expressed: fraction 0–1 (× 100 here) is assumed from the code;
   shipped values not checked against `growth-developer.md` §4.8.
5. Q5. *Answered*: `AbandonBuilding` is a synchronous `lot.SetState(4, 0, reason)` (`lot-developer.md`), so §4.19
   counts a multi-cell lot once.
6. Q6. `0x47E2C502` (Redevelop Size Threshold) is read but no consumer was found.
7. Q7. Game callers of slots 8–14 (UI / Lua) and whether any reads the garbage returns of 10, 12 and 14.

### 10.3 Corrections to other documents

| Document | Section | Wrong | Right (evidence) |
|---|---|---|---|
| `docs/subsystems/development.md` | Grow | returns bool; single pass; strict throughout | returns int built, has the `bool&` out flag; up to `size²+2` passes, relaxed from pass 2; a success ends the pass, and industry / agriculture stop (`0x0070D67F`–`0x0070D6F6`) |
| same | IsLotConfigurationSuitable | "too small" check unconditional | applies only in the strict pass and only when `stage < maxStage` (`0x00704C00`) |
| same | TakeOverAbandoned | all caps from this developer | power and water caps from the building's own developer (BDS slot 33); fire and park from this one (`0x0070A3D0`) |
| same | PickBuilding | minimum capacity uses `desired − built` | `lo = existing + 1` (÷ n for subdivision); `hi = max(desiredMax − built + existing, lo + 1)` (`0x00709BA0`) |
| same | tuning | min vacancy {0.1, 0.1}; years between styles always read | constructor {0.1, 0.1} but the reader's default is {0.01, 0.1}; years between is read only when 0, with default 40 (`0x00706FC0`) |
| same | existing lots | all compatible configurations become candidates | only the first compatible configuration per lot (`0x00708110`) |
| same | IsBuildingCompatible | style per mode | current style only (`0x00704E00`) |
| same | IsLotConfigurationCompatible | required bits 0–3 | bits 0–2 decide "roads required" (`0x007067B2`) |
| `zones.md` | lot manager slot 22 | (SDK) `CreateLotIfPossible` without the out parameter | 9 dword arguments: `(x, z, w, h, facing, cISC4Lot*& out, bool, bool, cfg)`, pop 36 (`0x00730A80`, callers `0x007068E0`, `0x0070A9B0`) |
| `foundations.md` | §6.2 | tract developer file-local generator listed as `cRZRandomSuper` | it is a `cRZRandom` (constructor `0x00913A5C` at `0x00A74E80`) |
| `building-development.md` | §10.2 Q1 | who claims construction resources | the tract developer claims `GetBuildingProfile(building).capacity` in `Build` before construction starts; farms claim `n + 30` then release 30, keeping `n` for the fields (§4.15) |

---

## 11. Conformance tests (must exist in `engine/conformance`)

All floats are float32 unless stated. "Bits" is the IEEE-754 pattern.

1. **Outcome codes**: the enum values of §4.1 are 0…27 exactly. `best` is the numeric maximum.
2. **desired / desiredMax**: overbuild 1.5:

   | Demand | desired | desiredMax |
   |---|---|---|
   | 7.9 | 7 | `trunc(r24(7 × 1.5)) = trunc(10.5) =` 10 |
   | 0.3 | 1 | 1 |
   | −5.0 | 1 | 1 |
   | NaN | `max(1, 0) =` 1 | 1 |

3. **Pass budget**: tract size 4. A developer whose every attempt succeeds without reaching `desired` makes exactly
   18 passes (`passesLeft` 17 → −1). A pass whose last code is not 27 (from pass 2 on) ends the call.
4. **Strict vs relaxed** (§4.7 rule 7): `desired 10`, `maxStage 3`. A configuration with max capacity 8 and stage 2
   gives 11 in pass 1 and 27 in pass 2. The same configuration with stage 3 gives 27 in both.
5. **Takeover thresholds**: `desired 10`, `desiredMax 15`, `built 0`, reoccupy {0, 1.2}: `T0 = 0`,
   `T1 = trunc(r24(15 × 1.2)) = 18` (`1.2f` = 1.2000000477; product 18.0000007 → r24 18.0 → 18).
   - Candidates of capacity 10, 6, 5 in sorted order: all three are taken (16 ≤ 18 after two, so the loop continues;
     21 > 18 stops it). `built = 21`, result 27.
6. **Score, residential type 2**: existing 30, abandoned 10, demolition {1, 0.2}: base `r24(r24(10 × 0.2f) + 30) =`
   **32.0**.
   - Side counts road {1, 0, 0, 0}, avenue {0, 1, 0, 0}, rail 0, weights (1, 2, 10): `32 + 1 + 2 =` **35.0**.
7. **Score, commercial**: the same candidate scores 32.0 whatever its networks (rail weight 0).
8. **Score, aggregation lot cost** (shipped `newLot` {1, 1}): 3 merged lots and no existing capacity give 2.0. A
   subdivision gives 1.0. With code defaults {10, 5}: 20.0 and 5.0.
9. **Industrial random term**: the file-local generator forced to `lcg = mul = 12345`. One `RandomDoubleUniform` gives
   `u = 0.9073938131332397` (mul `0x684AF645`); term `r24(u × 20) = 18.147876739501953`. Score of a type-1 candidate
   = **18.147876739501953**.
10. **Bounding rectangle**: anchor (10, 20), size (3, 2):

    | Facing | x | z |
    |---|---|---|
    | 0 | 8…10 | 20…21 |
    | 1 | 9…10 | 18…20 |
    | 2 | 10…12 | 19…20 |
    | 3 | 10…11 | 20…22 |

11. **Corner test**: facing 1, required bits {0} → side `j = (1 + 0 + 1) mod 4 = 2`.
    - Only rail on side 2 → false.
    - One street tile on side 2 → true.
    - Required bits {3} only: `hasReq` is false. With `requireRoads` true → 21; with false →
      `TestForCornerLot` decides on side 1.
12. **Capacity window**:
    - type 2, existing 12, desiredMax 30, built 5: `lo = 13`, `hi = max(37, 14) = 37`.
    - Type 4, n = 3: `lo = 4`, `hi = max(37, 5) = 37`.
    - existing 0, desiredMax 0, built 5: `lo = 1`, `hi = max(−5, 2) = 2`.
13. **Style percentages → group**: map {0x2000: 0.3, 0x2001: 0.7} gives `p` = 30, 70.
    - Draw `r = 4` → `0x2000`; `r = 30` → `0x2001`; `r = 99` → `0x2001`.
    - Map {0x2000: 0.125} gives `p = rhe(12.5) = 12`: `r = 12` → no group (0).
    - 0.005 → `p = 0`.
14. **Seeded `PickBuilding` draw sequence (R3–R5)**:
    - Setup: file-local generator forced to `lcg = mul = 12345`; no anchor needed; style map
      {0x2000: 0.3, 0x2001: 0.7}; three compatible configurations `C1 < C2 < C3` (pointer order); five qualifying
      buildings.
    - Draw A `RandomUint32Uniform(100) = 4` (lcg after: `0xD3DC167E`) → group `0x2000`.
    - Draw B `RandomUint32Uniform(3) = 2` (lcg `0xA70427DF`) → `C3`.
    - Draw C `RandomUint32Uniform(5) = 2` (lcg `0xD6651C2C`).
    - Building: with types 0x100, 0x101, 0x102, 0x2C1, 0x3 inserted in that order into a 193-bucket set, the hash
      order is
      - 0x3 (bucket 3);
      - 0x100 (256 mod 193 = 63);
      - 0x101 (64);
      - 0x102 (65);
      - 0x2C1 (705 mod 193 = 126).

      Index 2 → building **0x101**.
15. **Hash order tie**: inserting 5 then 198 (both bucket 5) iterates 198, 5.
16. **Seeded `UpdatePopulation` (R7)**: tract (3, 5) → seed `0x00300050`; variation 0.2f, minVacancy {0.01f, 0.1f};
    three lots in list order with available capacity 40, 25, 7.
    - d = 100: `dS = 1.0` (bits `0x3F800000`):

      | Cap | j (bits) | occ before | Second draw v (bits) | Final occ (bits) | pop |
      |---|---|---|---|---|---|
      | 40 | 0.02620549499988556 (`0x3CD6ACE8`) | ≥ 1 | 0.0991901084780693 (`0x3DCB242F`) | 0.9008098840713501 (`0x3F669B7A`) | **36** |
      | 25 | 0.1932249218225479 (`0x3E45DCC1`) | ≥ 1 | 0.045238249003887177 (`0x3D394BBE`) | 0.9547617435455322 (`0x3F746B44`) | **24** |
      | 7 | 0.0026829540729522705 (`0x3B2FD480`) | ≥ 1 | 0.08752158284187317 (`0x3DB33E84`) | 0.9124784469604492 (`0x3F699830`) | **6** |

      Six draws; the multiplicative state after the first is `0x10C58210`, after the second `0x7DB240D0`.
    - d = 80: `dS = 0.7999999523162842` (`0x3F4CCCCC`):

      | Cap | occ (bits) | pop |
      |---|---|---|
      | 40 | 0.8209643959999084 (`0x3F522AB9`) | **33** |
      | 25 | 0.9571203589439392 (`0x3F7505D7`), j = 0.1964004784822464 | **24** |
      | 7 | 0.9545798897743225 (`0x3F745F59`), j = 0.1932249218225479 | **7** |

      Three draws.
    - Shipped variation 0, d = 80: every `occ = 0.7999999523162842`; caps 40 → **32**, 25 → **20**. The draws still
      happen and advance the state.
17. **Population clamps**:
    - d = −50 → `occ < 0` → pop 0, with one draw.
    - d = 300 → occ ≥ 1 → second draw.
18. **Decline**: a 2 × 1 lot in state 3, 90 days, matching purpose and wealth, capacity 12 → returns 12, one
    `AbandonBuilding(L, 6)` *[inferred, Q5]*. Days 59 → returns 0. One `rand()` is consumed per call.
19. **Save record**: index 1, years since 2, `singleStyle` true, styles {0x2000, 0x2001, 0x2002, 0x2003}, years
    between 5 → 35 bytes:
    `03 00 | 01 00 00 00 | 02 00 00 00 | 01 | 04 00 00 00 | 00 20 00 00 01 20 00 00 02 20 00 00 03 20 00 00 | 05 00 00 00`.
    - A version-1 record (`01 00`, index, years) loads with `singleStyle` true and styles {0x2000, 0x2001, 0x2002}.
    - Version 4 is rejected.
20. **Style rotation**: years between 5, 4 styles, index 3, years since 4. One Year message → index 0, years since 0.
    Years between 0 → the index advances every year.
21. **SetActiveStyles**: current {A, B, C} at index 1 (B); set {B, D, B} → index **2** (the last match). Set {D} →
    index 0.
22. **Subdivision geometry**: parent location (20, 10), facing 2, width 6, configuration (2, 3), n = 3.
    - Pieces of size (2, 3) are requested from `CreateLotIfPossible` at (20, 8), (22, 8) and (24, 8): origin
      `z += 1 − 3`, step `x += 2`.
    - Facing 0, same sizes, parent location (20, 10): (19, 10), (17, 10), (15, 10).
23. **Farm minimum**: 14 suitable neighbouring lots → 0 and no claim. 15 → `Claim(45)` then `Release(30)`.
24. **IsLotSuitable order**: a lot that is both in state 2 and outside the land value range returns 22 (land value is
    checked before state). A historical lot with a matching building returns 18 in `IsLotSuitable`, and is still
    accepted by `TakeOverAbandoned`.
