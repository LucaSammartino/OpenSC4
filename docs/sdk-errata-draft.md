# gzcom-dll SDK errata (draft)

**Status: local draft. Not submitted upstream.** Reporting these to the SDK maintainers is a separate, public step that needs the project owner's approval.

This document lists places where the [gzcom-dll](https://github.com/nsgomez/gzcom-dll) SDK headers and sources, as vendored in `engine/third_party/gzcom-dll` (one commit, `3a1144f`, unchanged since), disagree with the binary interface of the game. It records **ABI facts only**: vtable slots, callee-popped byte counts, argument types and sizes, return channels, container layouts and allocators. It contains no game logic.

Before anything is reported, each item should be re-checked against the current upstream headers: the vendored copy does not record which upstream revision it came from, and some items may already be fixed upstream.

## Scope

- **Target**: SimCity 4 Deluxe 1.1.641, Windows (GOG build, `SimCity4.exe`). Addresses are virtual addresses in that executable (image base `0x00400000`). Other builds (1.1.638, the Mac port) were not checked.
- **Compiler model**: the SDK is meant to be built with MSVC (x86, `__thiscall`). Slot positions below are the positions MSVC gives the SDK declarations, including MSVC's overload grouping (see *How we verified*). Several header orders that look wrong on paper are correct under MSVC; they are listed under *Checked and correct under MSVC* and are not errata.
- **Interfaces covered**: every SDK interface header that maps to a game interface in `docs/abi/interfaces.tsv` (137 interface/implementation pairs), plus a vtable sweep of SDK interfaces outside that table. Interfaces where the game-side mapping is ambiguous are left out (see *Excluded*).

## Summary

289 findings: **124 crash**, **123 wrong value**, **42 cosmetic**. A further 22 return-channel flags are uncertain and are listed separately, not counted.

| Severity | Meaning | Per-interface | Containers | SDK sources | Total |
|---|---|---|---|---|---|
| crash | Calling through the SDK declaration (or implementing it) unbalances the stack, writes through a wrong pointer, reads a garbage pointer, or frees memory to the wrong allocator | 108 | 16 | 0 | 124 |
| wrong value | The call is ABI-safe, but the value read is garbage, truncated, misinterpreted or of the wrong type | 119 | 1 | 3 | 123 |
| cosmetic | Name, type spelling or a missing tail slot: no effect on a compiled call | 38 | 3 | 1 | 42 |

Per-interface findings by evidence:

| Evidence | crash | wrong value | cosmetic |
|---|---|---|---|
| ABI table + spec | 57 | 77 | 22 |
| spec | 5 | 42 | 14 |
| ABI table only | 11 | 0 | 1 |
| vtable sweep | 35 | 0 | 0 |
| SDK header text | 0 | 0 | 1 |

- *ABI table + spec*: the ABI tables show the mismatch and a project spec or the corrected declarations in `engine/plugin/FC4Interfaces.h` confirm the signature.
- *spec*: a project spec records the signature (return channel or meaning read from the function epilogue and callers); the ABI table is consistent with it but cannot show a type alone.
- *ABI table only*: the callee-popped byte count in the ABI table differs from the SDK declaration. The pop is exact, so the mismatch is certain; the correct parameter list is not established.
- *vtable sweep*: the SDK interface is not in the curated interface table; it was matched to a game vtable by its pop sequence. The pops are exact, but the mapping is inferred (medium confidence).

## Top 10

1. **`cISC4DepartmentBudget` 26 `GetLocalFundingPercent`**: returns a pointer, not a float in ST0; SDK callers dereference garbage (access violation at `0x3F800000` observed in game).
2. **`cISC4RegionalCity` 47/48 and 51-63**: the getter/setter pair at 47/48 is swapped by an overload, and the seven setters take `double` (pop 4 bytes more than declared).
3. **`cISCPropertyHolder` 11, 12, 17, 18, 19**: wrong argument lists on the most widely used property interface (pops 12, 12, 8, 4, 12).
4. **`cISC4NetworkManager` 13-24**: an extra SDK method shifts every later slot by one: every call from slot 13 on reaches the wrong function.
5. **`cISC4LotManager` 22 `CreateLotIfPossible`**: missing `cISC4Lot*&` out-parameter (pop 36, SDK 32).
6. **Containers: `SC4Vector`/`SC4List`/`SC4HashSet`/`SC4String` vs B**: about thirty methods exchange containers allocated by the game's STL node allocator; the SDK containers free them to the framework service, which corrupts allocator pools (delayed crash).
7. **`SC4HashSet` layout**: off by 4 bytes against the game's STLport `hash_set`.
8. **`SC4Percentage` by-value returns**: MSVC returns a class by value through a hidden pointer (pop +4): the SDK declares many of these as `float` or with a reference argument (residential 10-13, 30, 31; police 11, 25, 36; fire 10, 12; power 45, 46; lot 61; MySim 52, 53), and the reverse for `cISC4Ordinance` 10.
9. **`cIGZDate` 33-36**: the arithmetic operators return a `uint32` day number, not `cIGZDate&`.
10. **`cIGZVariant` 8, 60 and `cIGZPersistResourceManager` 22**: core framework methods with a missing argument (pops 4, 8, 8).

## Findings by interface

Columns: **Slot** is the vtable index (byte offset in brackets in the evidence). **SDK declares** is the declaration as MSVC places it at that slot. **Evidence** gives the callee-popped byte count from the ABI table (bold when it differs from the SDK; the SDK value in brackets includes a hidden return pointer where MSVC uses one), the return channel recorded by the ABI extractor with its confidence, the implementing function, and the spec section.

### cIGZCommandServer

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 25 | `~cIGZCommandServer` | `virtual ~cIGZCommandServer()` | The game interface has 25 slots (0-24) and no virtual destructor. The SDK adds `virtual ~cIGZCommandServer()` as slot 25 | slot 25 (`+0x64`); cGZCommandServer vtable ends at slot 24 (`ConvertCommandOutputToString`) *(ABI table only)* | cosmetic |

### cIGZDate

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 33 | `operator-` | `cIGZDate& operator-(int32_t nDays)` | `uint32 operator-(int32 days)`: returns a day number in EAX, does not modify the date | slot 33 (`+0x84`); pop 4; ret `eax-or-void` (medium); fn `0x00912A78`; foundations §17.4; FC4Interfaces.h `MinusDays` *(ABI table + spec)* | crash |
| 34 | `operator-` | `cIGZDate& operator-(cIGZDate const& sOther)` | `uint32 operator-(cIGZDate const&)`: a day-number difference in EAX | slot 34 (`+0x88`); pop 4; ret `eax-or-void` (medium); fn `0x00912A4E`; foundations §17.4 *(ABI table + spec)* | crash |
| 35 | `operator+` | `cIGZDate& operator+(int32_t nDays)` | `uint32 operator+(int32 days)`: a day number in EAX, the date is not modified | slot 35 (`+0x8C`); pop 4; ret `eax-or-void` (medium); fn `0x00912A82`; foundations §17.4 *(ABI table + spec)* | crash |
| 36 | `operator+` | `cIGZDate& operator+(cIGZDate const& sOther)` | `uint32 operator+(cIGZDate const&)`: a day-number sum in EAX | slot 36 (`+0x90`); pop 4; ret `eax-or-void` (medium); fn `0x00912A65`; foundations §17.4 *(ABI table + spec)* | crash |

### cIGZLua5Thread

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 71 | `DoFile` | `void DoFile(char const*)` | returns an `int32` status in EAX | slot 71 (`+0x11C`); pop 4; ret `eax-or-void` (medium); fn `0x008B7F90`; lua-host §5.1, "the same differences apply" to `cIGZLua5Thread` *(spec)* | wrong value |
| 72 | `DoString` | `void DoString(char const*)` | returns an `int32` status in EAX | slot 72 (`+0x120`); pop 4; ret `eax-or-void` (medium); fn `0x008B7FE0`; lua-host §5.1, "the same differences apply" to `cIGZLua5Thread` *(spec)* | wrong value |
| 73 | `DoBuffer` | `void DoBuffer(char const*, int32_t, char const*)` | returns an `int32` status in EAX | slot 73 (`+0x124`); pop 12; ret `eax-or-void` (medium); fn `0x008B8030`; lua-host §5.1, "the same differences apply" to `cIGZLua5Thread` *(spec)* | wrong value |
| 94 | `CloneTag` | `void CloneTag(int32_t)` | returns the new tag (`int32`) in EAX | slot 94 (`+0x178`); pop 4; ret `eax-or-void` (medium); fn `0x008B8750`; lua-host §5.1, "the same differences apply" to `cIGZLua5Thread` *(spec)* | wrong value |

### cIGZPersistResourceManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 22 | `FindObjectFactory` | `bool FindObjectFactory(cIGZPersistResourceFactory** unknown2)` | `FindObjectFactory(cGZPersistResourceKey const& key, cIGZPersistResourceFactory**)`: two arguments | slot 22 (`+0x58`); pop **8** (SDK 4); ret `via-call` (low); fn `0x00972D4E`; foundations §17.6 *(ABI table + spec)* | crash |

### cIGZSndMgr

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 19 | `SndAutoPlay` | `bool SndAutoPlay(uint32_t unknown1, int32_t volume)` | slot 19 pops 4: the one-argument `SndAutoPlay(uint32)` | slot 19 (`+0x4C`); pop **4** (SDK 8); ret `via-call` (low); fn `0x00968B1D`; vtable `cGZSndMgr::vftable_2` at `0x00AD7300`; overload pair compiled in the wrong order *(vtable sweep)* | crash |
| 20 | `SndAutoPlay` | `bool SndAutoPlay(uint32_t unknown1)` | slot 20 pops 8: `SndAutoPlay(uint32, int32 volume)` | slot 20 (`+0x50`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x009694A2`; vtable `cGZSndMgr::vftable_2` at `0x00AD7300`; same pair *(vtable sweep)* | crash |
| 21 | `SystemSndAutoPlay` | `bool SystemSndAutoPlay(uint32_t unknown1, int32_t volume)` | slot 21 pops 4: `SystemSndAutoPlay(uint32)` | slot 21 (`+0x54`); pop **4** (SDK 8); ret `via-call` (low); fn `0x00968B2C`; vtable `cGZSndMgr::vftable_2` at `0x00AD7300`; same pattern *(vtable sweep)* | crash |
| 22 | `SystemSndAutoPlay` | `bool SystemSndAutoPlay(uint32_t unknown1)` | slot 22 pops 8: `SystemSndAutoPlay(uint32, int32)` | slot 22 (`+0x58`); pop **8** (SDK 4); ret `via-call` (low); fn `0x00969536`; vtable `cGZSndMgr::vftable_2` at `0x00AD7300`; same pattern *(vtable sweep)* | crash |

### cIGZSndSys

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `DoEvent` | `bool DoEvent(cIGZString* unknown2, uint32_t unknown3, uint32_t unknown4, uint32_t unknown5, uint32_t unknown6)` | the callee pops 24 bytes (SDK: 20): one 4-byte argument is missing | slot 5 (`+0x14`); pop **24** (SDK 20); ret `eax-or-void` (medium); fn `0x00966D3B`; vtable `cGZSndSys::vftable` at `0x00AD7180` *(vtable sweep)* | crash |
| 9 | `PostEvent` | `bool PostEvent(cIGZString* unknown2, uint32_t unknown3, uint32_t unknown4, uint32_t unknown5, uint32_t unknown6)` | the callee pops 24 bytes (SDK: 20) | slot 9 (`+0x24`); pop **24** (SDK 20); ret `eax-or-void` (medium); fn `0x00966C15`; vtable `cGZSndSys::vftable` at `0x00AD7180` *(vtable sweep)* | crash |

### cIGZVariant

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 8 | `CanGetVal` | `bool CanGetVal() const` | `bool CanGetVal(uint16 type) const`: one argument | slot 8 (`+0x20`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x0091559A`; foundations §17.1; FC4Interfaces.h *(ABI table + spec)* | crash |
| 60 | `GetValUnknown` | `bool GetValUnknown(cIGZUnknown** outValue)` | `bool GetValUnknown(uint32 riid, void** ppv)`: two arguments | slot 60 (`+0xF0`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x00916644`; foundations §17.1; FC4Interfaces.h *(ABI table + spec)* | crash |

### cIGZWinCtrlMgr

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 9 | `CreateButton` | `cIGZWinBtn* CreateButton(uint32_t style, uint32_t id, cIGZString const& caption, cIGZBuffer* imageBuffer, cRZRect const* imageArea, uint32_t keyEquivalent)` | slot 9 pops 20 (the 5-argument `CreateButton`) | slot 9 (`+0x24`); pop **20** (SDK 24); ret `via-call` (low); fn `0x00996E39`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; overload pair compiled in the wrong order *(vtable sweep)* | crash |
| 10 | `CreateButton` | `cIGZWinBtn* CreateButton(uint32_t id, cIGZString const& caption, cIGZBuffer* imageBuffer, cRZRect const* imageArea, uint32_t keyEquivalent)` | slot 10 pops 24 (the 6-argument `CreateButton`) | slot 10 (`+0x28`); pop **24** (SDK 20); ret `eax-or-void` (medium); fn `0x00996D52`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; same pair *(vtable sweep)* | crash |
| 31 | `DefaultColor` | `uint32_t DefaultColor(int32_t id)` | slot 31 pops 16 (the 4-argument `DefaultColor`) | slot 31 (`+0x7C`); pop **16** (SDK 4); ret `eax-or-void` (medium); fn `0x00997D6B`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; overload pair reversed *(vtable sweep)* | crash |
| 32 | `DefaultColor` | `void DefaultColor(int32_t id, uint8_t& unknown2, uint8_t& unknown3, uint8_t& unknown4)` | slot 32 pops 4 (`DefaultColor(int32)`) | slot 32 (`+0x80`); pop **4** (SDK 16); ret `eax-or-void` (medium); fn `0x00997B8B`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; same pair *(vtable sweep)* | crash |
| 33 | `SetDefaultColor` | `void SetDefaultColor(int32_t id, uint32_t color)` | slot 33 pops 16 (the 4-byte-colour `SetDefaultColor`) | slot 33 (`+0x84`); pop **16** (SDK 8); ret `eax-or-void` (medium); fn `0x0099876A`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; overload pair reversed *(vtable sweep)* | crash |
| 34 | `SetDefaultColor` | `void SetDefaultColor(int32_t id, uint8_t unknown2, uint8_t unknown3, uint8_t unknown4)` | slot 34 pops 8 (`SetDefaultColor(int32, uint32)`) | slot 34 (`+0x88`); pop **8** (SDK 16); ret `via-call` (low); fn `0x0099761A`; vtable `cGZWinCtrlMgr::vftable_2` at `0x00ADC128`; same pair *(vtable sweep)* | crash |

### cIGZWinTextEdit

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 12 | `SetFontColor` | `bool SetFontColor(uint32_t color)` | slot 12 pops 8 (`SetFontColor(ColorIndex, uint32)`) | slot 12 (`+0x30`); pop **8** (SDK 4); ret `via-call` (low); fn `0x009BD3E1`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; overload pair reversed *(vtable sweep)* | crash |
| 13 | `SetFontColor` | `bool SetFontColor(ColorIndex index, uint32_t color)` | slot 13 pops 4 (`SetFontColor(uint32)`) | slot 13 (`+0x34`); pop **4** (SDK 8); ret `eax-or-void` (medium); fn `0x009BD3C0`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; same pair *(vtable sweep)* | crash |
| 44 | `GetText` | `const char* GetText()` | slot 44 pops 8 (the `(char*, uint32*)` overload) | slot 44 (`+0xB0`); pop **8** (SDK 0); ret `eax-or-void` (medium); fn `0x009BE2DB`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; three-member `GetText` group reversed (pops 8, 4, 0) *(vtable sweep)* | crash |
| 46 | `GetText` | `void GetText(char* textBuffer, uint32_t* param_2)` | slot 46 pops 0 (`GetText()`) | slot 46 (`+0xB8`); pop **0** (SDK 8); ret `eax-or-void` (medium); fn `0x009C32AC`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; same group *(vtable sweep)* | crash |
| 48 | `SetText` | `bool SetText(char* text, uint32_t textLength, bool sendTextChangedMessage)` | slot 48 pops 8 (`SetText(cIGZString const&, bool)`) | slot 48 (`+0xC0`); pop **8** (SDK 12); ret `via-call` (low); fn `0x00994E8C`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; overload pair reversed *(vtable sweep)* | crash |
| 49 | `SetText` | `void SetText(cIGZString const& text, bool sendTextChangedMessage)` | slot 49 pops 12 (`SetText(char*, uint32, bool)`) | slot 49 (`+0xC4`); pop **12** (SDK 8); ret `eax-or-void` (medium); fn `0x009C04FE`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; same pair *(vtable sweep)* | crash |
| 50 | `InsertText` | `void InsertText(char* text, uint32_t textLength, uint32_t index)` | slot 50 pops 16 (the line/column `InsertText`) | slot 50 (`+0xC8`); pop **16** (SDK 12); ret `via-call` (low); fn `0x009BCA39`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; overload pair reversed *(vtable sweep)* | crash |
| 51 | `InsertText` | `void InsertText(char* text, uint32_t textLength, uint32_t line, uint32_t column)` | slot 51 pops 12 (the index `InsertText`) | slot 51 (`+0xCC`); pop **12** (SDK 16); ret `eax-or-void` (medium); fn `0x009BF5B7`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; same pair *(vtable sweep)* | crash |
| 53 | `RemoveText` | `uint32_t RemoveText(uint32_t index, uint32_t length, cIGZString* removedText)` | slot 53 pops 20 (the 5-argument `RemoveText`) | slot 53 (`+0xD4`); pop **20** (SDK 12); ret `via-call` (low); fn `0x009BCA9D`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; three-member `RemoveText` group reversed (pops 20, 16, 12) *(vtable sweep)* | crash |
| 55 | `RemoveText` | `uint32_t RemoveText(uint32_t fromLine, uint32_t fromColumn, uint32_t toLine, uint32_t toColumn, cIGZString* removedText)` | slot 55 pops 12 (the index `RemoveText`) | slot 55 (`+0xDC`); pop **12** (SDK 20); ret `eax-or-void` (medium); fn `0x009BF8A8`; vtable `cGZWinTextEdit::vftable_1` at `0x00ADFA68`; same group *(vtable sweep)* | crash |

### cISC424HourClock

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 7 | `GetTimePercentage` | `float GetTimePercentage(void)` | the callee pops 4 and returns in EAX, nothing in ST0. Consistent with an `SC4Percentage` returned by value through a hidden pointer *[inferred]* | slot 7 (`+0x1C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x00445420`; vtable `cSC424HourClock::vftable` at `0x00A86588` *(vtable sweep)* | crash |
| 9 | `GetVisualTimePercentage` | `float GetVisualTimePercentage(void)` | same as slot 7 | slot 9 (`+0x24`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x00445450`; vtable `cSC424HourClock::vftable` at `0x00A86588` *(vtable sweep)* | crash |

### cISC43DRender

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 53 | `SetBackgroundPicture` | `bool SetBackgroundPicture(cIGZBuffer*, bool)` | slot 53 pops 16 (the 4-argument `SetBackgroundPicture`) | slot 53 (`+0xD4`); pop **16** (SDK 8); ret `eax-or-void` (medium); fn `0x007C26D0`; vtable `cSC43DRender::vftable` at `0x00ABAB60`; overload pair reversed *(vtable sweep)* | crash |
| 54 | `SetBackgroundPicture` | `bool SetBackgroundPicture(uint32_t, int32_t, int32_t, bool)` | slot 54 pops 8 (`SetBackgroundPicture(cIGZBuffer*, bool)`) | slot 54 (`+0xD8`); pop **8** (SDK 16); ret `eax-or-void` (medium); fn `0x007C2610`; vtable `cSC43DRender::vftable` at `0x00ABAB60`; same pair *(vtable sweep)* | crash |
| 64 | `Pick` | `bool Pick(int32_t, int32_t, cISC4OccupantFilter*, cIS3DModelInstance*&)` | slot 64 pops 24 (the 6-argument `Pick`) | slot 64 (`+0x100`); pop **24** (SDK 16); ret `eax-or-void` (medium); fn `0x007C6830`; vtable `cSC43DRender::vftable` at `0x00ABAB60`; overload pair reversed *(vtable sweep)* | crash |
| 65 | `Pick` | `bool Pick(int32_t, int32_t, float const*, float const*, cISC4OccupantFilter*, cIS3DModelInstance*&)` | slot 65 pops 16 (the 4-argument `Pick`) | slot 65 (`+0x104`); pop **16** (SDK 24); ret `eax-or-void` (medium); fn `0x007C2220`; vtable `cSC43DRender::vftable` at `0x00ABAB60`; same pair *(vtable sweep)* | crash |

### cISC4Advice

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 8 | `GetTrigger` | `intptr_t GetTrigger(void)` | `bool` in AL; the upper EAX bytes are garbage | slot 8 (`+0x20`); pop 0; ret `eax-or-void` (medium); fn `0x00408970`; advisors §12.3 *(ABI table + spec)* | wrong value |
| 9 | `GetTriggerString` | `char* GetTriggerString(void)` | returns a `cIGZString*` (string object), not `char*` | slot 9 (`+0x24`); pop 0; ret `eax-or-void` (medium); fn `0x00477890`; advisors §12.3 *(ABI table + spec)* | wrong value |
| 12 | `SetAdvisorID` | `cISC4Advice* SetAdvisorID(int32_t dwAdvisorID)` | returns nothing meaningful (EAX = the argument), not `cISC4Advice*` | slot 12 (`+0x30`); pop 4; ret `eax-or-void` (medium); fn `0x00407AD0`; advisors §12.3 *(ABI table + spec)* | crash |
| 18 | `GetTextTitle` | `char* GetTextTitle(void)` | returns a `cIGZString*`, not `char*` | slot 18 (`+0x48`); pop 0; ret `eax-or-void` (medium); fn `0x00800640`; advisors §12.3 *(ABI table + spec)* | wrong value |
| 20 | `GetTextMessage` | `char* GetTextMessage(void)` | returns a `cIGZString*`, not `char*` | slot 20 (`+0x50`); pop 0; ret `eax-or-void` (medium); fn `0x00407BB0`; advisors §12.3 *(ABI table + spec)* | wrong value |
| 23 | `NotifyGlobalEffectTriggerChanged` | `intptr_t NotifyGlobalEffectTriggerChanged(void)` | `void`: no meaningful EAX | slot 23 (`+0x5C`); pop 0; ret `eax-or-void` (medium); fn `0x00407F40`; advisors §12.3 *(ABI table + spec)* | wrong value |

### cISC4AdviceSubject

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 10 | `SetData1` | `bool SetData1(uint32_t dwData)` | returns the argument in EAX, not a `bool` | slot 10 (`+0x28`); pop 4; ret `eax-or-void` (medium); fn `0x008B6DE0`; advisors §12.4 *(ABI table + spec)* | wrong value |
| 12 | `SetData2` | `bool SetData2(uint32_t dwData)` | returns the argument in EAX, not a `bool` | slot 12 (`+0x30`); pop 4; ret `eax-or-void` (medium); fn `0x0040CE80`; advisors §12.4 *(ABI table + spec)* | wrong value |
| 13 | `` | (not declared) | slot 13 `bool Clone(cISC4AdviceSubject** out)` (pop 4) is missing. Tail slot: nothing shifts | slot 13 (`+0x34`); pop 4; ret `eax-or-void` (medium); fn `0x0040D180`; advisors §12.4; `0x0040D180` *(ABI table + spec)* | cosmetic |

### cISC4Advisor

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 10 | `GetCurrentAdvices` | `SC4List<pair<int32_t, cRZAutoRefCount<cISC4Advice>>>& GetCurrentAdvices(void)` | returns `std::map<cRZAutoRefCount<cISC4Advice>, int32>&` (the active map), not an `SC4List` | slot 10 (`+0x28`); pop 0; ret `eax-or-void` (medium); fn `0x0040D3D0`; advisors §12.2; `0x0040D3D0` *(ABI table + spec)* | crash |
| 11 | `GetCurrentAdvices` | `SC4List<pair<int32_t, cRZAutoRefCount<cISC4Advice>>>& GetCurrentAdvices(void) const` | same as slot 10 (one body) | slot 11 (`+0x2C`); pop 0; ret `eax-or-void` (medium); fn `0x0040D3D0`; advisors §12.2; `0x0040D3D0` *(ABI table + spec)* | crash |

### cISC4AdvisorSystem

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 6 | `RemoveAdivsor` | `bool RemoveAdivsor(int32_t dwAdvisorID)` | method is `RemoveAdvisor` (spelling) | slot 6 (`+0x18`); pop 4; ret `eax-or-void` (medium); fn `0x00419BC0`; advisors §12.1 *(ABI table + spec)* | cosmetic |
| 9 | `GetRandom` | `intptr_t GetRandom(void)` | returns `cRZRandom*` (address of the system's generator) | slot 9 (`+0x24`); pop 0; ret `eax-or-void` (medium); fn `0x00800640`; advisors §12.1 *(ABI table + spec)* | cosmetic |
| 18 | `PushContextAdvice` | `bool PushContextAdvice(cISC4Advice* pAdvice)` | `void`: EAX holds garbage | slot 18 (`+0x48`); pop 4; ret `eax-or-void` (medium); fn `0x00419840`; advisors §12.1 *(ABI table + spec)* | wrong value |
| 23-28 | `—` | (not declared) | slots 23-28 (`GetAdvisorWinProc`, `StartAdvisorWindow`, `ShowAdvisorWindow`, `HideAdvisorWindow`, `IsAdvisorWindowVisible`, `RemoveWinProcs`) are missing from the header. Tail slots: nothing shifts | slots 23-28; advisors §12.1; cSC4AdvisorSystem vtable has 29 slots *(ABI table + spec)* | cosmetic |

### cISC4AuraSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 6/7 | `—` | `AddAuraValue` overloads (slots 6/7) | the 4-float overload's parameters are (x, z, radius in metres, centre magnitude); the 3-argument overload's third parameter is a type 0..3, not the SDK enum | slots 6/7; aura §12.1 *(spec)* | cosmetic |

### cISC4AutomataControllerManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 17 | `GetController` | `cISC4AutomataController* GetController(cS3DVector3 const& unknown1, SC4String const& unknown2, float unknown3)` | the callee pops 20 bytes (SDK: 12) | slot 17 (`+0x44`); pop **20** (SDK 12); ret `eax-or-void` (medium); fn `0x00516410` *(ABI table only)* | crash |

### cISC4BudgetSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 14 | `GetTotalMonthlyExpense` | `uint32_t GetTotalMonthlyExpense(void)` | `int64` in EDX:EAX | slot 14 (`+0x38`); pop 0; ret `edx:eax` (medium); fn `0x00690990`; budget §6; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 16 | `GetTotalMonthlyIncome` | `int32_t GetTotalMonthlyIncome(void)` | `int64` in EDX:EAX | slot 16 (`+0x40`); pop 0; ret `edx:eax` (medium); fn `0x00690A30`; budget §6; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 20 | `SetTotalMonthlyExpense` | `int64_t SetTotalMonthlyExpense(int64_t llExpense)` | 32-bit value in EAX; EDX is not set | slot 20 (`+0x50`); pop 8; ret `eax-or-void` (medium); fn `0x00694280`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 21 | `SetTotalYearlyExpense` | `int64_t SetTotalYearlyExpense(int64_t llExpense)` | 32-bit value in EAX; EDX is not set | slot 21 (`+0x54`); pop 8; ret `eax-or-void` (medium); fn `0x006942A0`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 22 | `SetTotalMonthlyIncome` | `int64_t SetTotalMonthlyIncome(int64_t llIncome)` | 32-bit value in EAX; EDX is not set | slot 22 (`+0x58`); pop 8; ret `eax-or-void` (medium); fn `0x006942C0`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 23 | `SetTotalYearlyIncome` | `int64_t SetTotalYearlyIncome(int64_t llIncome)` | 32-bit value in EAX; EDX is not set | slot 23 (`+0x5C`); pop 8; ret `eax-or-void` (medium); fn `0x006942E0`; FC4Interfaces.h *(ABI table + spec)* | wrong value |

### cISC4BuildingDevelopmentSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 6 | `AddNewBuildingOccupant` | `bool AddNewBuildingOccupant(uint32_t dwUnknown, int32_t nUnknown1, int32_t nUnknown2, int32_t nUnknown3)` | returns `cISC4BuildingOccupant*` (or null) | slot 6 (`+0x18`); pop 16; ret `via-call` (low); fn `0x006956A0`; building-development §7.1; `0x006956A0` *(ABI table + spec)* | wrong value |
| 7 | `AddNewBuildingOccupant` | `bool AddNewBuildingOccupant(uint32_t dwUnknown, float* pfUnknown, int32_t nUnknown)` | returns `cISC4BuildingOccupant*` (or null) | slot 7 (`+0x1C`); pop 12; ret `eax-or-void` (medium); fn `0x00698DE0`; building-development §7.1; `0x00698DE0` *(ABI table + spec)* | wrong value |
| 9 | `GetCostOfOccupantDemolition` | `int64_t GetCostOfOccupantDemolition(void)` | `int64 GetCostOfOccupantDemolition(cISC4FoundationOccupant*, uint32)`: 8 bytes of arguments. The SDK declares this overload `(void)` and later in the header (non-adjacent overload) | slot 9 (`+0x24`); pop **8** (SDK 0); ret `edx:eax` (medium); fn `0x006AFC80`; foundations §17.8; building-development §7.1 *(ABI table + spec)* | crash |
| 11 | `GetAllBuildingTypes` | `size_t GetAllBuildingTypes(SC4Vector<uint32_t>& sList)` | EAX is undefined on return (no count) | slot 11 (`+0x2C`); pop 4; ret `eax-or-void` (medium); fn `0x00697C50`; allocators.md table (1); `0x00697C50` *(ABI table + spec)* | wrong value |
| 13 | `GetAllBuildingFamilyTypes` | `size_t GetAllBuildingFamilyTypes(SC4Vector<uint32_t>& sList)` | EAX is undefined on return (no count) | slot 13 (`+0x34`); pop 4; ret `eax-or-void` (medium); fn `0x00697D10`; allocators.md table (1); `0x00697D10` *(ABI table + spec)* | wrong value |
| 17 | `GetBuildingTypesByFilter` | `bool GetBuildingTypesByFilter(unordered_set<uint32_t>& sList, cISC4BuildingTypeFilter* pFilter)` | returns a `uint32` count in EAX; read as `bool` (AL), a multiple of 256 reads as false | slot 17 (`+0x44`); pop 8; ret `eax-or-void` (medium); fn `0x00699C50`; building-development §7.1; `0x00699C50` *(ABI table + spec)* | wrong value |
| 18 | `GetBuildingTypesByFamily` | `bool GetBuildingTypesByFamily(SC4Vector<uint32_t>& sList, uint32_t dwFamily)` | returns a `uint32` count in EAX; read as `bool` (AL), a multiple of 256 reads as false | slot 18 (`+0x48`); pop 8; ret `eax-or-void` (medium); fn `0x00697EF0`; building-development §7.1; `0x00697EF0` *(ABI table + spec)* | wrong value |
| 19 | `GetBuildingTypesByFamily` | `bool GetBuildingTypesByFamily(unordered_set<uint32_t>& sList, uint32_t dwFamily)` | returns a `uint32` count in EAX; read as `bool` (AL), a multiple of 256 reads as false | slot 19 (`+0x4C`); pop 8; ret `eax-or-void` (medium); fn `0x00699D20`; building-development §7.1; `0x00699D20` *(ABI table + spec)* | wrong value |
| 21 | `GetAllFoundationTypes` | `bool GetAllFoundationTypes(SC4Vector<uint32_t>& sList)` | returns a `uint32` count in EAX; read as `bool` (AL), a multiple of 256 reads as false | slot 21 (`+0x54`); pop 4; ret `eax-or-void` (medium); fn `0x00698050`; building-development §7.1; `0x00698050` *(ABI table + spec)* | wrong value |
| 27 | `GetAllLotRetainingWallTypes` | `bool GetAllLotRetainingWallTypes(SC4Vector<uint32_t>& sList)` | returns a `uint32` count in EAX; read as `bool` (AL), a multiple of 256 reads as false | slot 27 (`+0x6C`); pop 4; ret `eax-or-void` (medium); fn `0x00698180`; building-development §7.1; `0x00698180` *(ABI table + spec)* | wrong value |
| 34 | `GetDevelopers` | `intptr_t GetDevelopers(void)` | returns `std::map<uint32, cISC4GrowthDeveloper*>*` | slot 34 (`+0x88`); pop 0; ret `eax-or-void` (medium); fn `0x00698CD0`; building-development §7.1 *(ABI table + spec)* | cosmetic |
| 41 | `GetBuildingMap` | `intptr_t GetBuildingMap(void)` | returns `cISC4SimGrid<uint8>*` | slot 41 (`+0xA4`); pop 0; ret `eax-or-void` (medium); fn `0x00698CF0`; building-development §7.1 *(ABI table + spec)* | cosmetic |
| 42 | `GetBuildingMapKey` | `intptr_t GetBuildingMapKey(cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth)` | `uint8` in AL only; the upper EAX bits are garbage | slot 42 (`+0xA8`); pop 8; ret `eax-or-void` (medium); fn `0x00698D90`; building-development §7.1; `0x00698D90` *(ABI table + spec)* | wrong value |
| 43 | `GetBuildingMapKey` | `intptr_t GetBuildingMapKey(uint32_t dwBuilding)` | `uint8` in AL only | slot 43 (`+0xAC`); pop 4; ret `eax-or-void` (medium); fn `0x0069CCB0`; building-development §7.1; `0x0069CCB0` *(ABI table + spec)* | wrong value |
| 44 | `GetSpecialBuildings` | `intptr_t GetSpecialBuildings(void)` | returns a pointer to the internal occupant list (`ilist<cISC4Occupant*>*`) | slot 44 (`+0xB0`); pop 0; ret `eax-or-void` (medium); fn `0x00520080`; building-development §7.1 *(ABI table + spec)* | cosmetic |
| 45 | `GetMaxCapacity` | `int64_t GetMaxCapacity(cISC4ZoneManager::ZoneType zone, cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth)` | `float` in ST0 | slot 45 (`+0xB4`); pop 12; ret `x87` (high); fn `0x00696C20`; foundations §17.8; building-development §7.1 *(ABI table + spec)* | wrong value |
| 47 | `PlopBuilding` | `bool PlopBuilding(ConstructionProfile const& sConstructionProfile)` | returns `cISC4BuildingOccupant*` (or null) | slot 47 (`+0xBC`); pop 4; ret `via-call` (low); fn `0x00695570`; building-development §7.1; `0x00695570` *(ABI table + spec)* | wrong value |
| 48 | `GetFreeConstructionResources` | `int16_t GetFreeConstructionResources(void)` | `int32` in EAX (equal to the `int16` for values below 32768) | slot 48 (`+0xC0`); pop 0; ret `edx:eax?` (low); fn `0x006958E0`; building-development §7.1 *(ABI table + spec)* | cosmetic |

### cISC4CitySituationAdvice

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 27 | `GetSuccessMood` | `uint32_t GetSuccessMood() const` | `GetSuccessMood() const const`: duplicate `const` (MSVC warns C4114; GCC and Clang reject it) | slot 27 (`+0x6C`) *(SDK header text)* | cosmetic |
| 45 | `GetCoverageSuccessPct` | `float GetCoverageSuccessPct() const` | the callee pops 4 and returns in EAX, nothing in ST0 (consistent with a by-value `SC4Percentage` *[inferred]*) | slot 45 (`+0xB4`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x00423760`; vtable `cSC4CitySituationAdvice::vftable` at `0x00A845B8` *(vtable sweep)* | crash |

### cISC4Demand

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 30 | `EndOfCycle` | `float EndOfCycle()` | `bool` in EAX, nothing in ST0 | slot 30 (`+0x78`); pop 0; ret `eax-or-void` (medium); fn `0x005D4CF0`; demand §3.2 [compat]; FC4Interfaces.h *(ABI table + spec)* | wrong value |

### cISC4DemandSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 7 | `UpdateOccupantEffects` | `void UpdateOccupantEffects(SC4Percentage unknown1, SC4Percentage const& unknown2, SC4Percentage const& unknown3)` | the first parameter is the `uint32` building type, not an `SC4Percentage` (same size and slot) | slot 7 (`+0x1C`); pop 12; ret `eax-or-void` (medium); fn `0x006A6C90`; demand §5 [compat]; FC4Interfaces.h *(ABI table + spec)* | wrong value |

### cISC4DepartmentBudget

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 26 | `GetLocalFundingPercent` | `float GetLocalFundingPercent(cISCPropertyHolder* pPropertyHolder, uint32_t unknown2)` | returns `SC4Percentage const*` (a pointer to the float) in EAX; never null. Callers dereference it at once; a float in ST0 leaves garbage in EAX (in-game access violation reading `0x3F800000`, the bits of 1.0f) | slot 26 (`+0x68`); pop 8; ret `eax-or-void` (medium); fn `0x006AA610`; budget "Erratum: GetLocalFundingPercent"; FC4Interfaces.h *(ABI table + spec)* | crash |

### cISC4DisasterLayer

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 9 | `DisasterHasEnded` | `bool DisasterHasEnded(cISC4DisasterInstance* pDisaster)` | the callee pops 0 bytes (SDK: one pointer, 4) | slot 9 (`+0x24`); pop **0** (SDK 4); ret `eax-or-void` (medium); fn `0x00577E90` *(ABI table only)* | crash |

### cISC4DispatchManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 9 | `RevokeAllDispatches` | `bool RevokeAllDispatches()` | the callee pops 4 bytes (SDK: none) | slot 9 (`+0x24`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x00469D10` *(ABI table only)* | crash |

### cISC4FireProtectionSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `GetFireProtectionGrid` | `cISC4SimGrid<int16_t>* GetFireProtectionGrid() const` | returns `cISC4SimGrid<uint8>*` | slot 5 (`+0x14`); pop 0; ret `eax-or-void` (medium); fn `0x006EFA50`; protection §15.1; `0x006EFA50` *(ABI table + spec)* | wrong value |
| 10 | `GetStationEfficiency` | `float GetStationEfficiency()` | `SC4Percentage GetStationEfficiency(cISC4Occupant*)`: by-value class return through a hidden pointer; pop 8 (pointer + occupant), EAX = the pointer, nothing in ST0 | slot 10 (`+0x28`); pop **8** (SDK 0); ret `eax-or-void` (medium); fn `0x006AD430`; protection §15.1; `0x006AD430` *(ABI table + spec)* | crash |
| 12 | `GetChanceOfStrike` | `float GetChanceOfStrike()` | `SC4Percentage GetChanceOfStrike()` through a hidden pointer: pop 4, EAX = the pointer | slot 12 (`+0x30`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006AD950`; protection §15.1; `0x006AD950` *(ABI table + spec)* | crash |
| 17 | `GetStrikingStations` | `bool GetStrikingStations(SC4List<cISC4Occupant*>& list)` | returns the `int32` number appended, not `bool` | slot 17 (`+0x44`); pop 4; ret `eax-or-void` (medium); fn `0x006AE170`; protection §15.1; `0x006AE170` *(ABI table + spec)* | wrong value |
| 20 | `GetPercentCoverage` | `float GetPercentCoverage()` | returns `const SC4Percentage&` (a pointer) in EAX, nothing in ST0 | slot 20 (`+0x50`); pop 0; ret `eax-or-void` (medium); fn `0x004778A0`; protection §15.1; `0x004778A0` *(ABI table + spec)* | wrong value |
| 21 | `GetOccupantCoverage` | `bool GetOccupantCoverage(cISC4Occupant* pStation, SC4Percentage const& sPercentage, float& fRadiusX, float& fRadiusZ)` | the two `float&` outputs are the cover radius and the station radius | slot 21 (`+0x54`); pop 16; ret `eax-or-void` (medium); fn `0x006AD5E0`; protection §15.1 *(ABI table + spec)* | cosmetic |

### cISC4FloraOccupant

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `SetFloraType` | `bool SetFloraType(uint32_t)` | no meaningful `bool`: EAX = the argument (slots 5, 7, 9) or AL = its low byte (11, 13) | slot 5 (`+0x14`); pop 4; ret `eax-or-void` (medium); fn `0x006CE240`; flora §11 [SDK note] *(spec)* | wrong value |
| 7 | `SetBirthDate` | `bool SetBirthDate(uint32_t)` | no meaningful `bool`: EAX = the argument (slots 5, 7, 9) or AL = its low byte (11, 13) | slot 7 (`+0x1C`); pop 4; ret `eax-or-void` (medium); fn `0x004302D0`; flora §11 [SDK note] *(spec)* | wrong value |
| 9 | `SetLastSeedingDate` | `bool SetLastSeedingDate(uint32_t)` | no meaningful `bool`: EAX = the argument (slots 5, 7, 9) or AL = its low byte (11, 13) | slot 9 (`+0x24`); pop 4; ret `eax-or-void` (medium); fn `0x004302C0`; flora §11 [SDK note] *(spec)* | wrong value |
| 11 | `SetState` | `bool SetState(int32_t)` | no meaningful `bool`: EAX = the argument (slots 5, 7, 9) or AL = its low byte (11, 13) | slot 11 (`+0x2C`); pop 4; ret `eax-or-void` (medium); fn `0x005D6150`; flora §11 [SDK note] *(spec)* | wrong value |
| 13 | `SetOrientation` | `bool SetOrientation(uint32_t)` | no meaningful `bool`: EAX = the argument (slots 5, 7, 9) or AL = its low byte (11, 13) | slot 13 (`+0x34`); pop 4; ret `eax-or-void` (medium); fn `0x00472BB0`; flora §11 [SDK note] *(spec)* | wrong value |

### cISC4GrowthDeveloper

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 11 | `GetGrowthList` | `SC4Vector<GrowthItem>& GetGrowthList()` | `GrowthItem` is `{int32 x, int32 z, int32 maxStage}` (12 bytes); the SDK's four `uint8` fields are one `int32`. The vector is a game `std::vector` (allocator B, see Containers) | slot 11 (`+0x2C`); pop 0; ret `eax-or-void` (medium); fn `0x00798040`; growth-developer §10 item 1; allocators.md table (3) *(ABI table + spec)* | wrong value |
| 12 | `GetDeclineList` | `SC4Vector<GrowthItem>& GetDeclineList()` | same element type and allocator as slot 11 | slot 12 (`+0x30`); pop 0; ret `eax-or-void` (medium); fn `0x006B7540`; growth-developer §10; allocators.md table (3) *(ABI table + spec)* | wrong value |
| 24 | `GetAbandonmentThreshold` | `float GetAbandonmentThreshold(int32_t densityType)` | not `const`: the original inserts into its map | slot 24 (`+0x60`); pop 4; ret `x87` (high); fn `0x006B7620`; growth-developer §10 item 4 *(ABI table + spec)* | cosmetic |
| 26 | `GetStylePercentages` | `intptr_t GetStylePercentages()` | returns `std::map<uint32, SC4Percentage>*` (values are floats), not a map of `uint32` | slot 26 (`+0x68`); pop 0; ret `eax-or-void` (medium); fn `0x006B7590`; growth-developer §10 item 3; allocators.md table (3) *(ABI table + spec)* | wrong value |
| 29 | `EstimateDesirability` | `int32_t EstimateDesirability(int32_t cellX, int32_t cellZ)` | `int16` in AX; the upper EAX half is garbage | slot 29 (`+0x74`); pop 8; ret `eax-or-void` (medium); fn `0x006B4DF0`; growth-developer §10 item 2 *(ABI table + spec)* | wrong value |

### cISC4HistoryWarehouse

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 9 | `Coalesce` | `bool Coalesce(uint32_t dwHistoryType, cIGZDate const& sDate, int32_t lUnknown)` | `Coalesce(uint32 type, cIGZDate const& start, cIGZDate const& end, int32)`: 16 bytes | slot 9 (`+0x24`); pop **16** (SDK 12); ret `eax-or-void` (medium); fn `0x00476520`; foundations §17.8 *(ABI table + spec)* | crash |

### cISC4LandfillDeveloper

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 9 | `GetZoningCost` | `int32_t GetZoningCost(SC4CellRegion<int32_t> const* cellRegion)` | `int64` in EDX:EAX | slot 9 (`+0x24`); pop 4; ret `edx:eax` (medium); fn `0x006B9030`; landfill §7.1; `0x006B9030` *(ABI table + spec)* | wrong value |
| 12 | `GetLandfillZoneFromCell` | `intptr_t GetLandfillZoneFromCell(int cellX, int cellZ)` | returns the zone object pointer (`cSC4LandfillZone*`), not `intptr_t` | slot 12 (`+0x30`); pop 8; ret `eax-or-void` (medium); fn `0x006B9400`; landfill §7.1 *(ABI table + spec)* | cosmetic |

### cISC4LandfillOccupant

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 6 | `SetTileID` | `bool SetTileID(uint32_t)` | returns the previous tile ID (`uint32`), not `bool` | slot 6 (`+0x18`); pop 4; ret `eax-or-void` (medium); fn `0x00477450`; landfill §7.3; `0x00477450` *(spec)* | wrong value |

### cISC4LightingManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 19 | `GetColor` | `cS3DVector3 GetColor() const` | pops 20: the six `GetColor` overloads (slots 19-24) are in the reverse of the SDK's compiled order (game pops 20, 16, 12, 12, 8, 4) | slot 19 (`+0x4C`); pop **20** (SDK 4); ret `eax-or-void` (medium); fn `0x007D7840`; vtable `cSC4LightingManager::vftable` at `0x00ABB530`; six-member overload group reversed *(vtable sweep)* | crash |
| 20 | `GetColor` | `cS3DVector3 GetColor(cS3DVector3 const& normal) const` | pops 16 | slot 20 (`+0x50`); pop **16** (SDK 8); ret `eax-or-void` (medium); fn `0x007D7730`; vtable `cSC4LightingManager::vftable` at `0x00ABB530`; same group *(vtable sweep)* | crash |
| 23 | `GetColor` | `void GetColor(cS3DVector3 const& normal, unsigned char& outR, unsigned char& outG, unsigned char& outB) const` | pops 8 | slot 23 (`+0x5C`); pop **8** (SDK 16); ret `x87` (high); fn `0x007D7460`; vtable `cSC4LightingManager::vftable` at `0x00ABB530`; same group *(vtable sweep)* | crash |
| 24 | `GetColor` | `void GetColor(cS3DVector3 const& position, cS3DVector3 const& normal, unsigned char& outR, unsigned char& outG, unsigned char& outB) const` | pops 4 | slot 24 (`+0x60`); pop **4** (SDK 20); ret `eax-or-void` (medium); fn `0x007D73E0`; vtable `cSC4LightingManager::vftable` at `0x00ABB530`; same group *(vtable sweep)* | crash |

### cISC4Lot

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 61 | `GetOccupancy` | `float GetOccupancy(void) const` | `SC4Percentage GetOccupancy() const` by value through a hidden pointer: pop 4, EAX = the pointer | slot 61 (`+0xF4`); pop **4** (SDK 0); ret `x87\|eax` (low); fn `0x006C0CF0`; foundations §17.8; FC4Interfaces.h *(ABI table + spec)* | crash |

### cISC4LotConfiguration

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 19 | `IsCompatibleWithMonopolyPieceUse` | `bool IsCompatibleWithMonopolyPieceUse(void) const` | the callee pops 4 bytes (SDK: none) | slot 19 (`+0x4C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x004FC900` *(ABI table only)* | crash |

### cISC4LotConfigurationManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `AddLotConfiguration` | `bool AddLotConfiguration(cGZPersistResourceKey const& sUnknown, void* pUnknown)` | the callee pops 4 bytes (SDK: 8) | slot 5 (`+0x14`); pop **4** (SDK 8); ret `eax-or-void` (medium); fn `0x00489A60` *(ABI table only)* | crash |

### cISC4LotDeveloper

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 23 | `IsAreaDeveloped` | `bool IsAreaDeveloped(void)` | the callee pops 4 bytes (SDK: none) | slot 23 (`+0x5C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x005DEB40` *(ABI table only)* | crash |
| 53 | `RemoveLotFoundation` | `bool RemoveLotFoundation(cISC4Lot* pLot)` | the callee pops 8 bytes (SDK: 4) | slot 53 (`+0xD4`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x006C3EB0` *(ABI table only)* | crash |

### cISC4LotManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 7 | `GetLotsInCellRect` | `bool GetLotsInCellRect(SC4List<cISC4Lot*>& sLotList, int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nBottomRightCellX, int32_t nBottomRightCellZ, bool bUnknown)` | returns the `uint32` number of lots appended, not `bool` | slot 7 (`+0x1C`); pop 24; ret `eax-or-void` (medium); fn `0x0072E080`; budget Errata round 2 item 5; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 18 | `GetLotSavvyTerrainAltitude` | `int32_t GetLotSavvyTerrainAltitude(float fPointX, float fPointY, bool& bSucceeded)` | `float` in ST0, not `int32` | slot 18 (`+0x48`); pop 12; ret `via-call` (low); fn `0x0072A9F0`; flora §11 (dependencies) *(spec)* | wrong value |
| 22 | `CreateLotIfPossible` | `bool CreateLotIfPossible(int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nLotWidth, int32_t nLotHeight, int32_t nUnknown, bool bZoneNotNeeded, bool bIgnoreConfig, cISC4LotConfiguration* pConfig)` | `CreateLotIfPossible(int32 x, int32 z, int32 w, int32 h, int32, cISC4Lot*& lot, bool, bool, cISC4LotConfiguration*)`: the `cISC4Lot*&` out-parameter is missing; 36 bytes, not 32 | slot 22 (`+0x58`); pop **36** (SDK 32); ret `eax-or-void` (medium); fn `0x00730A80`; foundations §17.8; tract-developer; FC4Interfaces.h *(ABI table + spec)* | crash |

### cISC4MySim

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 52 | `LocalSchoolGrade` | `void LocalSchoolGrade(SC4Percentage&)` | `SC4Percentage LocalSchoolGrade(uint32 occupantGroup)` through a hidden pointer: pop 8, EAX = the pointer | slot 52 (`+0xD0`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x00429FE0`; mysims §7.4; `0x00429FE0` *(ABI table + spec)* | crash |
| 53 | `LocalHospitalGrade` | `void LocalHospitalGrade(SC4Percentage&)` | `SC4Percentage LocalHospitalGrade(uint32 occupantGroup)` through a hidden pointer: pop 8 | slot 53 (`+0xD4`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x0042A200`; mysims §7.4; `0x0042A200` *(ABI table + spec)* | crash |

### cISC4MySimAgentSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 12 | `GetActiveUIPanelSim` | `bool GetActiveUIPanelSim(int32_t& slot)` | returns `cISC4MySim*` in EAX (or null), not `bool`; the out parameter receives the vector index | slot 12 (`+0x30`); pop 4; ret `eax-or-void` (medium); fn `0x0042E960`; mysims §7.4; `0x0042E960` *(spec)* | wrong value |

### cISC4NeighborsSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 25 | `ProposeDeals` | `bool ProposeDeals(SC4List<cISC4NeighborConnection*>& deals, bool unknown2)` | the list holds `cISC4NeighborDeal*` (one reference each), not `cISC4NeighborConnection*` | slot 25 (`+0x64`); pop 8; ret `eax-or-void` (medium); fn `0x006D21A0`; neighbors §1 "List parameters"; allocators.md *(ABI table + spec)* | crash |
| 28 | `FindDealsAll` | `void FindDealsAll(SC4List<cISC4NeighborConnection*>& deals)` | list of `cISC4NeighborDeal*` | slot 28 (`+0x70`); pop 4; ret `eax-or-void` (medium); fn `0x006D4670`; neighbors §1 "List parameters"; allocators.md *(ABI table + spec)* | crash |
| 29 | `FindDeals` | `void FindDeals(int32_t dealType, cISC4RegionalCity* pRegionalCity, SC4List<cISC4NeighborConnection*>& deals)` | list of `cISC4NeighborDeal*` | slot 29 (`+0x74`); pop 12; ret `eax-or-void` (medium); fn `0x006D0840`; neighbors §1 "List parameters"; allocators.md *(ABI table + spec)* | crash |
| 30 | `FindDeals` | `void FindDeals(int32_t dealType, SC4List<cISC4NeighborConnection*>& deals)` | list of `cISC4NeighborDeal*` | slot 30 (`+0x78`); pop 8; ret `eax-or-void` (medium); fn `0x006D0760`; neighbors §1 "List parameters"; allocators.md *(ABI table + spec)* | crash |
| 32 | `GetTotalMonthlyNeighborDealIncome` | `int64_t GetTotalMonthlyNeighborDealIncome()` | 32 bits in EAX; EDX is not set (the SDK's high half is garbage) | slot 32 (`+0x80`); pop 0; ret `eax-or-void` (medium); fn `0x006CF860`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 33 | `GetTotalMonthlyNeighborDealExpense` | `int64_t GetTotalMonthlyNeighborDealExpense()` | 32 bits in EAX; EDX is not set | slot 33 (`+0x84`); pop 0; ret `eax-or-void` (medium); fn `0x006CF8D0`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 35 | `GetDealCost` | `int64_t GetDealCost(int32_t dealType, double unknown2, cISC4RegionalCity const* pRegionalCity)` | `double` in ST0, not `int64` | slot 35 (`+0x8C`); pop 16; ret `x87` (high); fn `0x006D3250`; FC4Interfaces.h *(ABI table + spec)* | wrong value |

### cISC4NetworkManager

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 13 | `GetAllIntersectionPieces2` | `bool GetAllIntersectionPieces2(cISC4NetworkOccupant* pOccupant, SC4List<cISC4Occupant*>& sList, cISC4OccupantManager* pManager, cISC4NetworkOccupant* pUnknown, cISC4OccupantFilter* pFilter)` | the game interface has no method here: slot 13 is `MapPreviewTexture` (pop 8). The SDK's `GetAllIntersectionPieces2` does not exist in the vtable, so every SDK slot from 13 to 24 is one too high (all eleven following pops match the SDK shifted by one; the game's slot 24 is its destructor) | slot 13 (`+0x34`); pop **8** (SDK 20); ret `eax-or-void` (medium); fn `0x00610AD0`; pops of slots 13-23: 8, 16, 4, 4, 4, 0, 0, 28, 8, 4, 4 = SDK slots 14-24 *(ABI table only)* | crash |

### cISC4NetworkOccupant

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 57 | `GetNormal` | `float GetNormal(float unknown1, float unknown2) const` | the callee pops 12 bytes (SDK: 8) | slot 57 (`+0xE4`); pop **12** (SDK 8); ret `x87` (high); fn `0x0061B6C0` *(ABI table only)* | crash |

### cISC4NetworkTool

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 10 | `UpdateOnZoneChange` | `void UpdateOnZoneChange()` | `UpdateOnZoneChange(SC4Rect<long> by value, uint32)`: 20 bytes (base and power-line tool) | slot 10 (`+0x28`); pop **20** (SDK 0); ret `unknown` (low); fn `0x0062C900`; foundations Errata M3 item 5; `0x0062C900`, `0x006522F0` *(ABI table + spec)* | crash |

### cISC4Ordinance

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 10 | `GetChanceAvailability` | `SC4Percentage GetChanceAvailability(void)` | returns the `SC4Percentage`'s f32 bits in EAX and pops 0. The SDK's by-value class return makes MSVC pass a hidden pointer (pop 4) | slot 10 (`+0x28`); pop **0** (SDK 4); ret `eax-or-void` (medium); fn `0x0080BC20`; FC4Interfaces.h; ordinance spec *(ABI table + spec)* | crash |

### cISC4OrdinanceSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 24 | `GetCumulativeOffset` | `int64_t GetCumulativeOffset(uint32_t unknown1, uint32_t unknown2)` | 32-bit sum in EAX; EDX is not set | slot 24 (`+0x60`); pop 8; ret `eax-or-void` (medium); fn `0x0080DD70`; ordinance §6.1 [compat] *(ABI table + spec)* | wrong value |

### cISC4PlumbingSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 6 | `GetSupplyValueMapPtr` | `bool GetSupplyValueMapPtr(cISC4SimGrid<uint8_t>*& grid)` | no meaningful `bool` (EAX = a pointer or garbage) | slot 6 (`+0x18`); pop 4; ret `eax-or-void` (medium); fn `0x006DA450`; plumbing §10.4; `0x006DA450` *(ABI table + spec)* | wrong value |
| 14 | `GetWaterProductionCapacity` | `int64_t GetWaterProductionCapacity()` | 32 bits in EAX; EDX is not set | slot 14 (`+0x38`); pop 0; ret `eax-or-void` (medium); fn `0x00423700`; plumbing §10.4; `0x00423700` *(ABI table + spec)* | wrong value |
| 15 | `GetWaterConsumed` | `int64_t GetWaterConsumed()` | 32 bits in EAX; EDX is not set | slot 15 (`+0x3C`); pop 0; ret `eax-or-void` (medium); fn `0x007C1910`; plumbing §10.4; `0x007C1910` *(ABI table + spec)* | wrong value |
| 16 | `GetWaterImported` | `int64_t GetWaterImported()` | 32 bits in EAX; EDX is not set | slot 16 (`+0x40`); pop 0; ret `eax-or-void` (medium); fn `0x007E56F0`; plumbing §10.4; `0x007E56F0` *(ABI table + spec)* | wrong value |
| 17 | `GetWaterExported` | `int64_t GetWaterExported()` | 32 bits in EAX; EDX is not set | slot 17 (`+0x44`); pop 0; ret `eax-or-void` (medium); fn `0x006DC680`; plumbing §10.4; `0x006DC680` *(ABI table + spec)* | wrong value |
| 23 | `GetWaterDemand` | `int64_t GetWaterDemand()` | 32 bits in EAX; EDX is not set | slot 23 (`+0x5C`); pop 0; ret `eax-or-void` (medium); fn `0x00798370`; plumbing §10.4; `0x00798370` *(ABI table + spec)* | wrong value |
| 27 | `FindWaterBuildings` | `bool FindWaterBuildings(cISC4Occupant** unknown1, SC4Percentage& unknown2, cISC4Occupant** unknown3, long& unknown4)` | no meaningful `bool` (EAX = a pointer or garbage) | slot 27 (`+0x6C`); pop 16; ret `edx:eax?` (low); fn `0x006DAB60`; plumbing §10.4; `0x006DAB60` *(ABI table + spec)* | wrong value |
| 28 | `GetWaterProducers` | `bool GetWaterProducers(ilist<cISC4Occupant*>& list)` | no meaningful `bool` (EAX = a pointer or garbage) | slot 28 (`+0x70`); pop 4; ret `via-call` (low); fn `0x006DC7D0`; plumbing §10.4; `0x006DC7D0` *(ABI table + spec)* | wrong value |

### cISC4PoliceSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `GetPolicePowerGrid` | `cISC4SimGrid<short>* GetPolicePowerGrid(void)` | returns `cISC4SimGrid<uint8>*` | slot 5 (`+0x14`); pop 0; ret `eax-or-void` (medium); fn `0x006EFA50`; protection §15.2; crime spec *(ABI table + spec)* | wrong value |
| 6 | `GetDepartmentBudget` | `intptr_t GetDepartmentBudget(void)` | returns `cISC4DepartmentBudget*` | slot 6 (`+0x18`); pop 0; ret `eax-or-void` (medium); fn `0x00407BA0`; protection §15.2 *(ABI table + spec)* | cosmetic |
| 9 | `GetJailDepartmentBudget` | `uint32_t GetJailDepartmentBudget(void)` | returns `cISC4DepartmentBudget*` | slot 9 (`+0x24`); pop 0; ret `eax-or-void` (medium); fn `0x00441080`; protection §15.2 *(ABI table + spec)* | cosmetic |
| 11 | `GetChanceOfStrike` | `float GetChanceOfStrike(void)` | `SC4Percentage GetChanceOfStrike()` through a hidden pointer: pop 4 | slot 11 (`+0x2C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006E0F90`; protection §15.2; `0x006E0F90` *(ABI table + spec)* | crash |
| 16 | `GetStrikingStations` | `bool GetStrikingStations(SC4List<cISC4Occupant*>& sList)` | returns the `int32` number appended | slot 16 (`+0x40`); pop 4; ret `eax-or-void` (medium); fn `0x006E1A00`; protection §15.2; `0x006E1A00` *(ABI table + spec)* | wrong value |
| 19 | `GetPercentCoverage` | `float GetPercentCoverage(void)` | returns `const SC4Percentage&` (a pointer) in EAX, nothing in ST0 | slot 19 (`+0x4C`); pop 0; ret `eax-or-void` (medium); fn `0x004778A0`; protection §15.2; `0x004778A0` *(ABI table + spec)* | wrong value |
| 20 | `GetOccupantCoverage` | `bool GetOccupantCoverage(cISC4Occupant* pStation, SC4Percentage const& sPercentage, float& fRadiusX, float& fRadiusZ)` | the two `float&` outputs are the cover radius and the station radius | slot 20 (`+0x50`); pop 16; ret `eax-or-void` (medium); fn `0x006E0940`; protection §15.2 *(ABI table + spec)* | cosmetic |
| 21 | `GetPoliceStations` | `bool GetPoliceStations(SC4List<cISC4Occupant*>& sList)` | returns the `int32` count | slot 21 (`+0x54`); pop 4; ret `eax-or-void` (medium); fn `0x006E0180`; protection §15.2 *(ABI table + spec)* | wrong value |
| 25 | `GetStationEfficiency` | `float GetStationEfficiency(cISC4Occupant* pStation)` | `SC4Percentage GetStationEfficiency(cISC4Occupant*)` through a hidden pointer: pop 8 | slot 25 (`+0x64`); pop **8** (SDK 4); ret `x87\|eax` (low); fn `0x006E0E70`; protection §15.2; `0x006E0E70` *(ABI table + spec)* | crash |
| 27 | `ResetUnjailedCriminals` | `bool ResetUnjailedCriminals(void)` | returns an `int32` count, not `bool` | slot 27 (`+0x6C`); pop 0; ret `eax-or-void` (medium); fn `0x006E0A00`; protection §15.2; crime spec *(ABI table + spec)* | wrong value |
| 36 | `GetJailedPercentage` | `uint32_t GetJailedPercentage(void)` | `SC4Percentage GetJailedPercentage()` through a hidden pointer: pop 4 | slot 36 (`+0x90`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006E0B30`; protection §15.2; `0x006E0B30` *(ABI table + spec)* | crash |
| 37 | `GetJailCurrentUpkeepCost` | `int64_t GetJailCurrentUpkeepCost(void)` | `int64 GetJailCurrentUpkeepCost(cISC4Occupant* jail)`: one argument | slot 37 (`+0x94`); pop **4** (SDK 0); ret `edx:eax?` (low); fn `0x006E0A40`; protection §15.2; `0x006E0A40` *(ABI table + spec)* | crash |
| 38 | `ProcessMonthlyCrimeStats` | `bool ProcessMonthlyCrimeStats(void)` | no meaningful `bool` | slot 38 (`+0x98`); pop 0; ret `via-call` (low); fn `0x006E2B50`; protection §15.2 *(ABI table + spec)* | wrong value |
| 39 | `IsInJailbreak` | `bool IsInJailbreak(void)` | `bool IsInJailbreak(cISC4Occupant* jail)`: one argument | slot 39 (`+0x9C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006E1070`; protection §15.2; `0x006E1070` *(ABI table + spec)* | crash |
| 40 | `BeginJailbreak` | `float BeginJailbreak(cISC4Occupant* pJail)` | `bool` in AL, nothing in ST0 | slot 40 (`+0xA0`); pop 4; ret `eax-or-void` (medium); fn `0x006E1AF0`; protection §15.2; `0x006E1AF0` *(ABI table + spec)* | wrong value |
| 42 | `GetJailbreakBuildings` | `bool GetJailbreakBuildings(SC4List<cISC4Occupant*>& sList)` | returns the `int32` count | slot 42 (`+0xA8`); pop 4; ret `eax-or-void` (medium); fn `0x006E08A0`; protection §15.2; `0x006E08A0` *(ABI table + spec)* | wrong value |

### cISC4PollutionSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 7 | `GetAirValue` | `bool GetAirValue(uint32_t dwCellX, uint32_t dwCellZ, int32_t& wValue)` | no meaningful `bool` (EAX holds an unrelated value) | slot 7 (`+0x1C`); pop 12; ret `eax-or-void` (medium); fn `0x006E2EF0`; pollution §8.1 *(spec)* | wrong value |
| 17 | `GetWorstAirTract` | `bool GetWorstAirTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` | no meaningful `bool` (EAX holds an unrelated value) | slot 17 (`+0x44`); pop 12; ret `eax-or-void` (medium); fn `0x006E4FA0`; pollution §8.1 *(spec)* | wrong value |
| 18 | `GetWorstWaterTract` | `bool GetWorstWaterTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` | no meaningful `bool` (EAX holds an unrelated value) | slot 18 (`+0x48`); pop 12; ret `eax-or-void` (medium); fn `0x006E4FD0`; pollution §8.1 *(spec)* | wrong value |
| 19 | `GetWorstGarbageTract` | `bool GetWorstGarbageTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` | no meaningful `bool` (EAX holds an unrelated value) | slot 19 (`+0x4C`); pop 12; ret `eax-or-void` (medium); fn `0x006E5000`; pollution §8.1 *(spec)* | wrong value |
| 25 | `SetIsRadioactive` | `bool SetIsRadioactive(uint32_t dwCellX, uint32_t dwCellZ, bool bRadioactive)` | no meaningful `bool` (EAX holds an unrelated value) | slot 25 (`+0x64`); pop 12; ret `via-call` (low); fn `0x006E3030`; pollution §8.1 *(spec)* | wrong value |
| 29 | `AddToGarbageSupply` | `bool AddToGarbageSupply(int32_t nGarbage)` | no meaningful `bool` (EAX holds an unrelated value) | slot 29 (`+0x74`); pop 4; ret `eax-or-void` (medium); fn `0x006E32E0`; pollution §8.1 *(spec)* | wrong value |
| 30 | `SetGarbageCapacity` | `bool SetGarbageCapacity(int32_t nCapacity)` | no meaningful `bool` (EAX holds an unrelated value) | slot 30 (`+0x78`); pop 4; ret `eax-or-void` (medium); fn `0x006E5050`; pollution §8.1 *(spec)* | wrong value |
| 33 | `GetRecyclingCenterEffect` | `int32_t GetRecyclingCenterEffect(void)` | returns `SC4Percentage*` (pointer to a float fraction), not `int32` | slot 33 (`+0x84`); pop 0; ret `eax-or-void` (medium); fn `0x006E3150`; pollution §8.1; ordinance spec *(spec)* | wrong value |
| 34 | `GetGarbageAbductedPrevMonth` | `int32_t GetGarbageAbductedPrevMonth(void)` | `int64` in EDX:EAX | slot 34 (`+0x88`); pop 0; ret `edx:eax` (medium); fn `0x006E4F20`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 35 | `GetGarbageProducedLastMonth` | `int32_t GetGarbageProducedLastMonth(void)` | `int64` in EDX:EAX | slot 35 (`+0x8C`); pop 0; ret `edx:eax` (medium); fn `0x006E4F30`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 36 | `GetGarbageImportedLastMonth` | `int32_t GetGarbageImportedLastMonth(void)` | `int64` in EDX:EAX | slot 36 (`+0x90`); pop 0; ret `edx:eax` (medium); fn `0x006E4F40`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 37 | `GetGarbageExportedLastMonth` | `int32_t GetGarbageExportedLastMonth(void)` | `int64` in EDX:EAX | slot 37 (`+0x94`); pop 0; ret `edx:eax` (medium); fn `0x006E4F50`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 38 | `GetGarbageRecycledLastMonth` | `int32_t GetGarbageRecycledLastMonth(void)` | `int64` in EDX:EAX | slot 38 (`+0x98`); pop 0; ret `edx:eax?` (low); fn `0x006E4F60`; pollution §8.1 *(spec)* | wrong value |
| 39 | `GetGarbageIncineratedLastMonth` | `int32_t GetGarbageIncineratedLastMonth(void)` | `int64` in EDX:EAX | slot 39 (`+0x9C`); pop 0; ret `edx:eax` (medium); fn `0x006E4F70`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 40 | `GetGarbageConvertedToEnergyLastMonth` | `int32_t GetGarbageConvertedToEnergyLastMonth(void)` | `int64` in EDX:EAX | slot 40 (`+0xA0`); pop 0; ret `edx:eax` (medium); fn `0x006E4F80`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 41 | `GetGarbageLandfilledLastMonth` | `int32_t GetGarbageLandfilledLastMonth(void)` | `int64` in EDX:EAX | slot 41 (`+0xA4`); pop 0; ret `edx:eax` (medium); fn `0x006E4F90`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 46 | `GetGarbageBuildings` | `bool GetGarbageBuildings(SC4List<cISC4Occupant*>& sList)` | no meaningful `bool` (EAX holds an unrelated value) | slot 46 (`+0xB8`); pop 4; ret `via-call` (low); fn `0x006E4C70`; pollution §8.1 *(spec)* | wrong value |
| 50 | `SetIsActiveLandfill` | `bool SetIsActiveLandfill(uint32_t dwCellX, uint32_t dwCellZ, bool bActive)` | no meaningful `bool` (EAX holds an unrelated value) | slot 50 (`+0xC8`); pop 12; ret `via-call` (low); fn `0x006E31C0`; pollution §8.1 *(spec)* | wrong value |
| 51 | `GarbagePutInLandfill` | `int64_t GarbagePutInLandfill(int32_t nGarbage)` | EDX:EAX holds no meaningful 64-bit value | slot 51 (`+0xCC`); pop 4; ret `eax-or-void` (medium); fn `0x006E3250`; pollution §8.1 *(ABI table + spec)* | wrong value |
| 65 | `CalcOccupantMaintenanceCost` | `bool CalcOccupantMaintenanceCost(cISC4Occupant* pOccupant, uint64_t& dwCost)` | the `int64&` output is signed | slot 65 (`+0x104`); pop 8; ret `eax-or-void` (medium); fn `0x006E4530`; pollution §8.1 *(spec)* | cosmetic |
| 66 | `AddRadiationToCity` | `void AddRadiationToCity(cS3DVector3& sCenter, float fRadiusX, float fRadiusZ)` | the floats are (centre strength, radius in tracts), not radiusX / radiusZ | slot 66 (`+0x108`); pop 12; ret `eax-or-void` (medium); fn `0x006E7040`; pollution §8.1 *(spec)* | cosmetic |

### cISC4PowerSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 32 | `GetOccupantPowerConsumed` | `void GetOccupantPowerConsumed(cISC4Occupant* occupant, uint32_t& powerConsumed)` | returns `bool` in AL (game callers may read it) | slot 32 (`+0x80`); pop 8; ret `eax-or-void` (medium); fn `0x006EB0D0`; power §11 SDK check; foundations Errata M3 item 5; `0x006EB0D0` *(ABI table + spec)* | wrong value |
| 33 | `GetOccupantPowerProduced` | `void GetOccupantPowerProduced(cISC4Occupant* occupant, uint32_t& powerProduced, uint32_t& powerUsed)` | returns `bool` in AL (game callers may read it) | slot 33 (`+0x84`); pop 12; ret `eax-or-void` (medium); fn `0x006EB420`; power §11 SDK check; foundations Errata M3 item 5; `0x006EB420` *(ABI table + spec)* | wrong value |
| 34 | `GetOccupantMaxPowerProduced` | `void GetOccupantMaxPowerProduced(cISC4Occupant* occupant, uint32_t& maxPowerProduced)` | returns `bool` in AL (game callers may read it) | slot 34 (`+0x88`); pop 8; ret `eax-or-void` (medium); fn `0x006EB110`; power §11 SDK check; foundations Errata M3 item 5; `0x006EB110` *(ABI table + spec)* | wrong value |
| 40 | `GetChanceOfStrike` | `void GetChanceOfStrike(SC4Percentage& chance)` | ABI-compatible (pop 4), but a game caller loads the float through the returned EAX: a replacement must return the argument's address (`SC4Percentage*`) | slot 40 (`+0xA0`); pop 4; ret `eax-or-void` (medium); fn `0x006EB130`; power §11 item 4; caller `0x0042220F` *(spec)* | crash |
| 45 | `GetCurrentPlantEfficiencyPercent` | `float GetCurrentPlantEfficiencyPercent(cISC4Occupant* occupant)` | `SC4Percentage f(cISC4Occupant*)` through a hidden pointer: pop 8, EAX = the pointer, ST0 empty | slot 45 (`+0xB4`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x006EB5F0`; power §11 SDK check; foundations Errata M3 item 5; `0x006EB5F0` *(ABI table + spec)* | crash |
| 46 | `GetCurrentPlantConditionPercent` | `float GetCurrentPlantConditionPercent(cISC4Occupant* occupant)` | same as slot 45 | slot 46 (`+0xB8`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x006EC820`; power §11 SDK check; foundations Errata M3 item 5; `0x006EC820` *(ABI table + spec)* | crash |

### cISC4RegionalCity

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 13 | `GetWorkforcePercentage` | `float GetWorkforcePercentage(void) const` | 32 bits in EAX, nothing in ST0 | slot 13 (`+0x34`); pop 0; ret `eax-or-void` (medium); fn `0x00800640`; FC4Interfaces.h *(ABI table + spec)* | wrong value |
| 47 | `GetWorldBaseElevation` | `void GetWorldBaseElevation(float fElevation)` | the game's slot 47 is the getter `float GetWorldBaseElevation() const` (pop 0, ST0). The SDK's second declaration `void GetWorldBaseElevation(float)` is really the setter; as an overload, MSVC puts it first | slot 47 (`+0xBC`); pop **0** (SDK 4); ret `x87` (high); fn `0x004ABCA0`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 48 | `GetWorldBaseElevation` | `float GetWorldBaseElevation(void) const` | the game's slot 48 is the setter `void SetWorldBaseElevation(float)` (pop 4) | slot 48 (`+0xC0`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x004ABCB0`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 50 | `GetBudget` | `float GetBudget(void) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 50 (`+0xC8`); pop 0; ret `x87` (high); fn `0x004A7640`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 51 | `SetBudget` | `void SetBudget(float fBudget)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 51 (`+0xCC`); pop **8** (SDK 4); ret `x87` (high); fn `0x004A7650`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 52 | `GetIncome` | `float GetIncome(void) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 52 (`+0xD0`); pop 0; ret `x87` (high); fn `0x004A7660`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 53 | `SetIncome` | `void SetIncome(float fIncome)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 53 (`+0xD4`); pop **8** (SDK 4); ret `x87` (high); fn `0x004A7670`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 54 | `GetExported` | `float GetExported(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 54 (`+0xD8`); pop 4; ret `x87` (high); fn `0x004A9EE0`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 55 | `SetExported` | `void SetExported(int32_t nCommodity, float fExports)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 55 (`+0xDC`); pop **12** (SDK 8); ret `x87` (high); fn `0x004A9F00`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 56 | `GetImported` | `float GetImported(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 56 (`+0xE0`); pop 4; ret `x87` (high); fn `0x004A9F40`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 57 | `SetImported` | `void SetImported(int32_t nCommodity, float fImports)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 57 (`+0xE4`); pop **12** (SDK 8); ret `x87` (high); fn `0x004A9F60`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 58 | `GetProduced` | `float GetProduced(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 58 (`+0xE8`); pop 4; ret `x87` (high); fn `0x004A9FA0`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 59 | `SetProduced` | `void SetProduced(int32_t nCommodity, float fProduced)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 59 (`+0xEC`); pop **12** (SDK 8); ret `x87` (high); fn `0x004A9FC0`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 60 | `GetDemanded` | `float GetDemanded(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 60 (`+0xF0`); pop 4; ret `x87` (high); fn `0x004AA000`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 61 | `SetDemanded` | `void SetDemanded(int32_t nCommodity, float fDemanded)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 61 (`+0xF4`); pop **12** (SDK 8); ret `x87` (high); fn `0x004AA020`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 62 | `GetCostPerUnit` | `float GetCostPerUnit(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 62 (`+0xF8`); pop 4; ret `x87` (high); fn `0x004AA060`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 63 | `SetCostPerUnit` | `void SetCostPerUnit(int32_t nCommodity, float fCostPerUnit)` | the value parameter is a `double` (8 bytes, pop 4 more than declared) | slot 63 (`+0xFC`); pop **12** (SDK 8); ret `x87` (high); fn `0x004AA080`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | crash |
| 64 | `GetCommodityBalance` | `float GetCommodityBalance(int32_t nCommodity) const` | returns a `double` in ST0; a `float` declaration still reads it, rounded | slot 64 (`+0x100`); pop 4; ret `x87` (high); fn `0x004A7680`; neighbors "Compiled slot tables"; FC4Interfaces.h *(ABI table + spec)* | cosmetic |
| 72 | `GetPointsOfInterest` | `void GetPointsOfInterest(uint32_t dwPointOfInterestType, SC4Vector<uint32_t>& sList)` | a game `std::vector` (allocator B) of **12-byte** elements, not `SC4Vector<uint32_t>` | slot 72 (`+0x120`); pop 8; ret `eax-or-void` (medium); fn `0x004A8A40`; allocators.md table (1); FC4Interfaces.h; `0x004A8A40` *(ABI table + spec)* | crash |

### cISC4ResidentialSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 10 | `ChanceOfSchoolStrike` | `float ChanceOfSchoolStrike(void)` | `SC4Percentage` by value through a hidden pointer: pop 4, EAX = the pointer, nothing in ST0 | slot 10 (`+0x28`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006F9670`; residential §5 slot table; residential-education-health; `0x006F9670` *(ABI table + spec)* | crash |
| 11 | `ChanceOfHealthStrike` | `float ChanceOfHealthStrike(void)` | `SC4Percentage` by value through a hidden pointer: pop 4, EAX = the pointer, nothing in ST0 | slot 11 (`+0x2C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006F9730`; residential §5 slot table; residential-education-health; `0x006F9730` *(ABI table + spec)* | crash |
| 12 | `GetSchoolSystemRating` | `float GetSchoolSystemRating(void)` | `SC4Percentage` by value through a hidden pointer: pop 4, EAX = the pointer, nothing in ST0 | slot 12 (`+0x30`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006F9F20`; residential §5 slot table; residential-education-health; `0x006F9F20` *(ABI table + spec)* | crash |
| 13 | `GetHealthSystemRating` | `float GetHealthSystemRating(void)` | `SC4Percentage` by value through a hidden pointer: pop 4, EAX = the pointer, nothing in ST0 | slot 13 (`+0x34`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006F9DD0`; residential §5 slot table; residential-education-health; `0x006F9DD0` *(ABI table + spec)* | crash |
| 14 | `GetSchoolSystemTotals` | `bool GetSchoolSystemTotals(intptr_t tQueryData, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` | returns a `uint32` count in EAX | slot 14 (`+0x38`); pop 8; ret `eax-or-void` (medium); fn `0x006FA240`; residential §5 slot table; residential-education-health *(ABI table + spec)* | wrong value |
| 15 | `GetHospitalSystemTotals` | `bool GetHospitalSystemTotals(intptr_t tQueryData, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` | returns a `uint32` count in EAX | slot 15 (`+0x3C`); pop 8; ret `eax-or-void` (medium); fn `0x006FA260`; residential §5 slot table; residential-education-health *(ABI table + spec)* | wrong value |
| 19 | `SetOccupantFundingPercentages` | `bool SetOccupantFundingPercentages(cISC4Occupant* pOccupant, SC4Percentage const& sSchoolFunding, SC4Percentage const& sHealthFunding, bool bUnknown)` | no meaningful `bool` (EAX incidental) | slot 19 (`+0x4C`); pop 16; ret `via-call` (low); fn `0x006F8290`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 20 | `GetOccupantFundingPercentages` | `bool GetOccupantFundingPercentages(cISC4Occupant* pOccupant, SC4Percentage& sSchoolFunding, SC4Percentage& sHealthFunding, bool bUnknown)` | no meaningful `bool` (EAX incidental) | slot 20 (`+0x50`); pop 16; ret `eax-or-void` (medium); fn `0x006F84B0`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 21 | `GetAverageEQGrid` | `bool GetAverageEQGrid(cISC4SimGrid<float>*& pGrid, float* fMin, float* fMax)` | EAX = the `max` pointer, not a `bool` | slot 21 (`+0x54`); pop 12; ret `eax-or-void` (medium); fn `0x006FCF30`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 22 | `GetAverageHQGrid` | `bool GetAverageHQGrid(cISC4SimGrid<float>*& pGrid, float* fMin, float* fMax)` | EAX = the `max` pointer, not a `bool` | slot 22 (`+0x58`); pop 12; ret `eax-or-void` (medium); fn `0x006FCFD0`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 23 | `GetEQGrids` | `bool GetEQGrids(cISC4SimGrid<float>*& pGrid, cISC4SimGrid<float>* pUnknown1, cISC4SimGrid<float>* pUnknown2)` | all three parameters are outputs (`cISC4SimGrid<T>*&`), each AddRef'd. The SDK declares the 2nd and 3rd as plain pointers, so the game writes through them | slot 23 (`+0x5C`); pop 12; ret `via-call` (low); fn `0x006F72C0`; residential §5 slot table; residential-education-health; crime and aura specs *(spec)* | crash |
| 24 | `GetHQGrids` | `bool GetHQGrids(cISC4SimGrid<float>*& pGrid, cISC4SimGrid<float>* pUnknown1, cISC4SimGrid<float>* pUnknown2)` | all three parameters are outputs (`cISC4SimGrid<T>*&`), each AddRef'd. The SDK declares the 2nd and 3rd as plain pointers, so the game writes through them | slot 24 (`+0x60`); pop 12; ret `via-call` (low); fn `0x006F7300`; residential §5 slot table; residential-education-health; crime and aura specs *(spec)* | crash |
| 25 | `GetPopulationGrids` | `bool GetPopulationGrids(cISC4SimGrid<uint16_t>*& pGrid, cISC4SimGrid<uint16_t>* pUnknown1, cISC4SimGrid<uint16_t>* pUnknown2)` | all three parameters are outputs (`cISC4SimGrid<T>*&`), each AddRef'd. The SDK declares the 2nd and 3rd as plain pointers, so the game writes through them | slot 25 (`+0x64`); pop 12; ret `via-call` (low); fn `0x006F7340`; residential §5 slot table; residential-education-health; crime and aura specs *(spec)* | crash |
| 28 | `EstimateCurrentOccupantCapacity` | `bool EstimateCurrentOccupantCapacity(cISC4Occupant* pOccupant, uint32_t& dwUnknown1, uint32_t& dwUnknown2)` | no meaningful `bool` (EAX incidental) | slot 28 (`+0x70`); pop 12; ret `unknown` (low); fn `0x006FD510`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 29 | `GetCellLifeExpectancy` | `int32_t GetCellLifeExpectancy(uint32_t dwCellX, uint32_t dwCellZ)` | `uint8` in AL only | slot 29 (`+0x74`); pop 8; ret `eax-or-void` (medium); fn `0x006FC570`; residential §5 slot table; residential-education-health; `0x006FC570` *(spec)* | wrong value |
| 30 | `GetCellWorkforcePercent` | `float GetCellWorkforcePercent(uint32_t dwCellX, uint32_t dwCellZ)` | `SC4Percentage GetCellWorkforcePercent(x, z)` through a hidden pointer: pop 12 | slot 30 (`+0x78`); pop **12** (SDK 8); ret `eax-or-void` (medium); fn `0x006FC6E0`; residential §5 slot table; residential-education-health; `0x006FC6E0` *(ABI table + spec)* | crash |
| 31 | `GetGlobalWorkforcePercent` | `float GetGlobalWorkforcePercent(void)` | `SC4Percentage` through a hidden pointer: pop 4 | slot 31 (`+0x7C`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006FC860`; residential §5 slot table; residential-education-health; `0x006FC860` *(ABI table + spec)* | crash |
| 34 | `GetGlobalLE` | `float GetGlobalLE(void)` | `uint8` in AL; nothing in ST0 | slot 34 (`+0x88`); pop 0; ret `eax-or-void` (medium); fn `0x006FC9E0`; residential §5 slot table; residential-education-health; `0x006FC9E0` *(ABI table + spec)* | wrong value |
| 39 | `GetSchoolAverageGradeMap` | `int32_t GetSchoolAverageGradeMap(void)` | returns `cISC4SimGrid<float>*` | slot 39 (`+0x9C`); pop 0; ret `eax-or-void` (medium); fn `0x00809240`; residential §5 slot table; residential-education-health *(spec)* | cosmetic |
| 40 | `GetHospitalAverageGradeMap` | `int32_t GetHospitalAverageGradeMap(void)` | returns `cISC4SimGrid<float>*` | slot 40 (`+0xA0`); pop 0; ret `eax-or-void` (medium); fn `0x00407AC0`; residential §5 slot table; residential-education-health *(spec)* | cosmetic |
| 41 | `GetAverageAgeMap` | `int32_t GetAverageAgeMap(void)` | returns `cISC4SimGrid<uint8>*` | slot 41 (`+0xA4`); pop 0; ret `eax-or-void` (medium); fn `0x006AAEB0`; residential §5 slot table; residential-education-health *(spec)* | cosmetic |
| 42 | `GetAverageNewAgeByWealth` | `int32_t GetAverageNewAgeByWealth(cISC4BuildingOccupant::WealthType eWealthType)` | `uint8` in AL only | slot 42 (`+0xA8`); pop 4; ret `eax-or-void` (medium); fn `0x006F88B0`; residential §5 slot table; residential-education-health; `0x006F88B0` *(spec)* | wrong value |
| 45 | `GetOccupantCoverage` | `bool GetOccupantCoverage(cISC4Occupant* pOccupant, SC4Percentage const& sEffectiveness, float& fRangeX, float& fRangeZ)` | the `float&` outputs are (distance at `level`, radius) | slot 45 (`+0xB4`); pop 16; ret `eax-or-void` (medium); fn `0x006FD260`; residential §5 slot table; residential-education-health *(spec)* | cosmetic |
| 47 | `GetSchoolBuildings` | `bool GetSchoolBuildings(SC4List<cISC4Occupant*>& sBuildings, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` | no meaningful `bool` (EAX incidental) | slot 47 (`+0xBC`); pop 8; ret `eax-or-void` (medium); fn `0x006FADE0`; residential §5 slot table; residential-education-health *(spec)* | wrong value |
| 56 | `GetAutoBudget` | `bool GetAutoBudget(void)` | `bool GetAutoBudget(cISC4Occupant*)`: one argument | slot 56 (`+0xE0`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x006F7D70`; residential §5 slot table; residential-education-health; `0x006F7D70` *(ABI table + spec)* | crash |
| 57 | `SetAutoBudget` | `bool SetAutoBudget(bool bEnable)` | `bool SetAutoBudget(cISC4Occupant*, bool)`: two arguments | slot 57 (`+0xE4`); pop **8** (SDK 4); ret `unknown` (low); fn `0x006F7DA0`; residential §5 slot table; residential-education-health; `0x006F7DA0` *(ABI table + spec)* | crash |

### cISC4SimGrid<uint8>

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 8 | `GetAverageValueInCellRect` | `T GetAverageValueInCellRect(int32_t nTopLeftX, int32_t nTopLeftZ, int32_t nBottomRightX, int32_t nBottomRightZ) const` | the parameters are (x, z, width, height), not two corners | slot 8 (`+0x20`); pop 16; ret `edx:eax?` (low); fn `0x0045DE50`; simgrid §4.3 *(spec)* | cosmetic |
| 9 | `SetTractSize` | `bool SetTractSize(int32_t nSize)` | no meaningful `bool` (callers ignore it) | slot 9 (`+0x24`); pop 4; ret `eax-or-void` (medium); fn `0x00460FC0`; simgrid §4.3 *(spec)* | wrong value |
| 19 | `PositionToTract` | `bool PositionToTract(float fPosX, float fPosZ, int32_t& nTractX, int32_t& nTractZ) const` | no meaningful `bool` (callers ignore it) | slot 19 (`+0x4C`); pop 16; ret `edx:eax?` (low); fn `0x0045DDF0`; simgrid §4.3 *(spec)* | wrong value |
| 20 | `TractCornerToPosition` | `bool TractCornerToPosition(int32_t nTractX, int32_t nTractZ, float& fPosX, float& fPosZ) const` | no meaningful `bool` (callers ignore it) | slot 20 (`+0x50`); pop 16; ret `x87` (high); fn `0x0045ECE0`; simgrid §4.3 *(spec)* | wrong value |
| 21 | `TractCenterToPosition` | `bool TractCenterToPosition(int32_t nTractX, int32_t nTractZ, float& fPosX, float& fPosZ) const` | no meaningful `bool` (callers ignore it) | slot 21 (`+0x54`); pop 16; ret `x87` (high); fn `0x0045E7C0`; simgrid §4.3 *(spec)* | wrong value |
| 24 | `GetGridData` | `intptr_t GetGridData(void) const` | returns a pointer to the internal STLport `vector<vector<T>>` | slot 24 (`+0x60`); pop 0; ret `eax-or-void` (medium); fn `0x006A9750`; simgrid §4.3 *(spec)* | cosmetic |

### cISC4TractDeveloper

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 5 | `Grow` | `bool Grow(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile, SC4List<Outcome>& sOutcomes, bool& bUnknown)` | returns an `int32` count in EAX; a multiple of 256 reads as false, a negative value as true | slot 5 (`+0x14`); pop 12; ret `eax-or-void` (medium); fn `0x0070D130`; tract-developer §7.1; `0x0070D130` *(ABI table + spec)* | wrong value |
| 6 | `Decline` | `bool Decline(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile)` | returns an `int32` sum in EAX | slot 6 (`+0x18`); pop 4; ret `eax-or-void` (medium); fn `0x00709200`; tract-developer §7.1; `0x00709200` *(ABI table + spec)* | wrong value |
| 7 | `UpdatePopulation` | `bool UpdatePopulation(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile)` | `void`: EAX is garbage | slot 7 (`+0x1C`); pop 4; ret `eax-or-void` (medium); fn `0x00706BC0`; tract-developer §7.1; `0x00706BC0` *(ABI table + spec)* | wrong value |
| 10 | `SetActiveStyles` | `bool SetActiveStyles(SC4Vector<uint32_t> const& sStyles)` | EAX = the address of the member vector, not a `bool` | slot 10 (`+0x28`); pop 4; ret `eax-or-void` (medium); fn `0x00707470`; tract-developer §7.1; `0x00707470` *(ABI table + spec)* | wrong value |
| 12 | `UseAllStylesAtOnce` | `cISC4TractDeveloper* UseAllStylesAtOnce(bool bFlag)` | returns the new flag in AL (upper EAX garbage), not `cISC4TractDeveloper*` | slot 12 (`+0x30`); pop 4; ret `eax-or-void` (medium); fn `0x0070C860`; tract-developer §7.1; `0x0070C860` *(ABI table + spec)* | crash |
| 14 | `SetYearsBetweenStyleChanges` | `cISC4TractDeveloper* SetYearsBetweenStyleChanges(int32_t nYears)` | returns the argument in EAX, not `cISC4TractDeveloper*` | slot 14 (`+0x38`); pop 4; ret `eax-or-void` (medium); fn `0x0070C880`; tract-developer §7.1; `0x0070C880` *(ABI table + spec)* | crash |

### cISC4TrafficSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 26 | `GetTravelStrategyPercentages` | `bool GetTravelStrategyPercentages(uint32_t wealthType, SC4Vector<SC4Percentage> unknown2)` | the callee pops 8 bytes. The SDK passes an `SC4Vector<SC4Percentage>` **by value** (12 bytes, 16 in all) | slot 26 (`+0x68`); pop **8** (SDK 16); ret `x87` (high); fn `0x00718BB0` *(ABI table only)* | crash |

### cISC4WeatherSimulator

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 18 | `GetMoistureValueMapPtr` | `void GetMoistureValueMapPtr(SC4Vector<SC4Vector<uint8_t>>& vector)` | the game writes the address of its own grid object into the caller's variable: the parameter is `SC4Vector<SC4Vector<uint8_t>>*&`, not a vector reference | slot 18 (`+0x48`); pop 4; ret `eax-or-void` (medium); fn `0x007256C0`; weather §11.2 [compat]; FC4Interfaces.h *(spec)* | crash |

### cISCLua

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 4 | `Close` | `void Close()` | `bool Close()`: AL = 1 | slot 4 (`+0x10`); pop 0; ret `eax-or-void` (medium); fn `0x00880ED0`; lua-host §5.1; `0x00880950` *(spec)* | wrong value |
| 21 | `ToUint32` | `uint32_t ToUint32(int32_t)` | `ToUint32` and `ToSint32` are one function (low 32 bits of the number) | slot 21 (`+0x54`); pop 4; ret `eax-or-void` (medium); fn `0x00880BC0`; lua-host §5.1; `0x008B7640` *(spec)* | cosmetic |
| 28 | `ToUserTag` | `int32_t ToUserTag(int32_t)` | returns the `void*` boxed in a userdata (no tag); same register | slot 28 (`+0x70`); pop 4; ret `eax-or-void` (medium); fn `0x00880C20`; lua-host §5.1; `0x008B87F0` *(spec)* | wrong value |
| 35 | `PushUserTag` | `void PushUserTag(void*, int32_t)` | `PushUserTag` and `PushUserData` are one function `(void*, int32 tag)` | slot 35 (`+0x8C`); pop 8; ret `eax-or-void` (medium); fn `0x00880C90`; lua-host §5.1; `0x008B87A0` *(spec)* | cosmetic |
| 44 | `NewTable` | `int32_t NewTable()` | `void`: EAX is not set | slot 44 (`+0xB0`); pop 0; ret `eax-or-void` (medium); fn `0x00880420`; lua-host §5.1; `0x008B7C30` *(spec)* | wrong value |
| 50 | `Ref` | `int32_t Ref(int32_t)` | the `int32` argument is ignored (pop 4 matches) | slot 50 (`+0xC8`); pop 4; ret `eax-or-void` (medium); fn `0x00880480`; lua-host §5.1; `0x00880480` *(spec)* | cosmetic |
| 53 | `DoFile` | `void DoFile(char const*)` | returns an `int32` status in EAX | slot 53 (`+0xD4`); pop 4; ret `eax-or-void` (medium); fn `0x00880D60`; lua-host §5.1 *(spec)* | wrong value |
| 54 | `DoString` | `void DoString(char const*)` | returns an `int32` status in EAX | slot 54 (`+0xD8`); pop 4; ret `eax-or-void` (medium); fn `0x00880D70`; lua-host §5.1 *(spec)* | wrong value |
| 55 | `DoBuffer` | `void DoBuffer(char const*, int32_t, char const*)` | returns an `int32` status in EAX | slot 55 (`+0xDC`); pop 12; ret `eax-or-void` (medium); fn `0x00880D80`; lua-host §5.1 *(spec)* | wrong value |
| 65 | `Error` | `void Error(char const*)` | the message argument is ignored (pop 4 matches) | slot 65 (`+0x104`); pop 4; ret `eax-or-void` (medium); fn `0x00880E10`; lua-host §5.1; `0x008B80F0` *(spec)* | cosmetic |
| 70 | `CloneTag` | `void CloneTag(int32_t)` | returns the new tag (`int32`) in EAX | slot 70 (`+0x118`); pop 4; ret `eax-or-void` (medium); fn `0x00880E40`; lua-host §5.1 *(spec)* | wrong value |
| 75 | `ToDataRegistryEntryID` | `uint32_t ToDataRegistryEntryID(cISC4GameDataRegistry*, int32_t)` | returns the registry entry pointer (`cEntry*`), not a `uint32` id; same register | slot 75 (`+0x12C`); pop 8; ret `eax-or-void` (medium); fn `0x00880760`; lua-host §5.1; `0x00880760` *(spec)* | cosmetic |

### cISCPropertyHolder

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 11 | `AddProperty` | `bool AddProperty(uint32_t dwProperty, int32_t lValue, bool bUnknown)` | `bool AddProperty(uint32 id, cIGZUnknown* value, bool)`: the original stores an object value; the SDK says `int32` | slot 11 (`+0x2C`); pop 12; ret `via-call` (low); fn `0x005FF950`; foundations §17.2; FC4Interfaces.h *(ABI table + spec)* | crash |
| 12 | `AddProperty` | `bool AddProperty(uint32_t dwProperty, cIGZString const& szValue)` | `bool AddProperty(uint32 id, cIGZString const& value, bool)`: the `bool` is missing (pop 12) | slot 12 (`+0x30`); pop **12** (SDK 8); ret `via-call` (low); fn `0x005FF860`; foundations §17.2; FC4Interfaces.h *(ABI table + spec)* | crash |
| 17 | `RemoveProperty` | `bool RemoveProperty(uint32_t dwProperty)` | `bool RemoveProperty(uint32 id, bool)` | slot 17 (`+0x44`); pop **8** (SDK 4); ret `eax-or-void` (medium); fn `0x005FEBC0`; foundations §17.2; FC4Interfaces.h *(ABI table + spec)* | crash |
| 18 | `RemoveAllProperties` | `bool RemoveAllProperties(void)` | `bool RemoveAllProperties(bool)` | slot 18 (`+0x48`); pop **4** (SDK 0); ret `eax-or-void` (medium); fn `0x005FF3C0`; foundations §17.2; FC4Interfaces.h *(ABI table + spec)* | crash |
| 19 | `EnumProperties` | `bool EnumProperties(FunctionPtr2 pFunction2, FunctionPtr1 pFunctionPipe) const` | `bool EnumProperties(filter, fn, void* data) const`: three arguments | slot 19 (`+0x4C`); pop **12** (SDK 8); ret `via-call` (low); fn `0x005FDA80`; foundations §17.2; FC4Interfaces.h *(ABI table + spec)* | crash |

### cISTESurfaceWater

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 7 | `IsNavigable` | `bool IsNavigable(int32_t cellX, int32_t cellZ, int32_t radius, cISC4Lot* lot) const` | the callee pops 20 (SDK: 16) | slot 7 (`+0x1C`); pop **20** (SDK 16); ret `via-call` (low); fn `0x0073CEF0`; vtable `cSTESurfaceWater::vftable` at `0x00AB3B00`; both `IsNavigable` overloads take 4 more bytes than declared *(vtable sweep)* | crash |
| 8 | `IsNavigable` | `bool IsNavigable(cS3DVector3* position, int32_t radius, cISC4Lot* lot) const` | the callee pops 16 (SDK: 12) | slot 8 (`+0x20`); pop **16** (SDK 12); ret `eax-or-void` (medium); fn `0x0073EBF0`; vtable `cSTESurfaceWater::vftable` at `0x00AB3B00`; same *(vtable sweep)* | crash |

### cISTETerrain

| Slot | Method | SDK declares | The exe says | Evidence | Severity |
|---|---|---|---|---|---|
| 59 | `BeginBrushScrub` | `void BeginBrushScrub(uint32_t unknown1)` | the callee pops 8 bytes (SDK: 4) | slot 59 (`+0xEC`); pop **8** (SDK 4); ret `via-call` (low); fn `0x00747A30` *(ABI table only)* | crash |

## Containers and allocators

The game has three instances of one pooled allocator, and a block must be freed to the instance that allocated it:

| | Instance | Allocate / free | Used by |
|---|---|---|---|
| A | framework allocator service (IID `0x3AE4BEAB`) | `cIGZAllocatorService` slots 3 / 4 | the game's `ilist<>`; **all SDK containers** (`SC4List`, `SC4Vector`, `SC4HashSet`, `SC4String`) |
| B | STL node allocator | `0x0090CF54` / `0x0090CF63` (cdecl) | the game's `std::list`, `std::vector`, `std::map`, `hash_set`, `std::deque` with the default allocator; the `cRZString` character buffer |
| C | global heap | `0x005E55E0` / `0x005E5620` (cdecl) | the game's `operator new` / `delete` |

### Type and allocator errors

| Item | Detail | Severity | Evidence |
|---|---|---|---|
| `SC4HashSet` layout | The game's STLport `hash_set` is 0x14 bytes: a 4-byte functor member at +0x00, the bucket vector `{begin, end, capacity_end}` at +0x04..+0x0C, the element count at +0x10. `SC4HashSet` starts its members at +0x00 (`mpStart`), 0x10 bytes in all, although its own comments give the +0x04..+0x10 offsets. Every field the game reads is off by 4 bytes. | crash | allocators.md "hash_set"; `SC4HashSet.h` |
| `SC4HashSet` allocator | Nodes (8 bytes, `{next, value}`) and the bucket buffer are B blocks in the game; `SC4HashSet` allocates both from A. | crash | allocators.md "hash_set"; insert `0x00624DD0`, resize `0x004F0940`, destroy `0x008EBA60` |
| `SC4String` allocator | `cRZString` (0x14 bytes, same layout as `SC4String`) allocates its character buffer with B (`0x0090CF54` / `0x0090CF63`); `SC4String.cpp` `allocate`/`deallocate` use A. The game destroys strings inside containers inline (it frees `begin` with B, bypassing the vtable), so an `SC4String` handed to the game has its buffer freed with B, and a game string grown or freed by SDK code has its B buffer freed with A. | crash | allocators.md "Strings"; ctor `0x00578390`, grow `0x0040EBC0`, inline destroy in `0x00767270` |
| `SC4Vector` copy | `push_back(const T*, size_t)` copies elements with `memmove`, and `operator=(const SC4Vector&)` uses it. For an element type that owns memory (for example `SC4String`, or `BudgetGroupInfo` which contains one), the copy and the source then share buffers, and both destructors free them: a double free. Element types are only safe when trivially copyable (or `cIGZUnknown` pointers, which `operator=` AddRefs). | crash | `SC4Vector.h` `push_back`, `operator=` (SDK source; not an exe fact) |
| `SC4HashSet` chain teardown | `DeleteBucketNodes` deallocates only the first node of each bucket chain; chained nodes leak. | wrong value | `SC4HashSet.h` (SDK source; not an exe fact) |
| Allocator rule | Every list here, game or SDK, is one pointer to a 12-byte head node with 12-byte nodes, and every vector is `{begin, end, capacity_end}`: the layouts match, so the allocator alone decides compatibility. A game signature with `std::list`/`std::vector`/`hash_set` and the default allocator uses B; one with `ilist<>` uses A, and only those are compatible with `SC4List`. A block freed to the wrong instance corrupts that instance's pools; the crash appears later and elsewhere (access violation in the pool allocator at `0x005E16F5`). Blocks above 2044 bytes go to the CRT heap whatever the instance, which can hide the bug. | (context, not counted) | allocators.md "Three allocator instances" |

### Methods that exchange B containers through SDK container types

A row marked crash is a crash for a plugin that calls the method with the SDK type and then grows or frees the container, or that implements the method and fills the game's container with A blocks. `SC4List` remains correct for the `ilist<>` (A) methods: neighbour connections and deals (`cISC4NeighborsSimulator` 16-22, 25, 28-30; `cISC4RegionalCity` 21, 69, 70), `cISC4Lot::GetLotOccupants`, the occupant-manager queries and `ReleaseOccupantList`, the residential, power, plumbing, fire and police station lists, and `cISC4BuildingDevelopmentSimulator::GetSpecialBuildings`. `cISC4RegionalCity::GetPointsOfInterest` (72) and the growth-developer lists are counted in their interface sections above.

| SDK type | Interface | Methods (slot) | SDK declares | Game type and allocator | Severity | Evidence |
|---|---|---|---|---|---|---|
| SC4List | `cISC4LotManager` | `GetLotsInCellRect` (7), `GetLotsSurroundingLot` (8, 9), `GetLotsSurroundingRect` (10) | `SC4List<cISC4Lot*>&` | game `std::list` (B); slot 7 first frees the caller's existing nodes with B | crash | `0x0072E080`, `0x0072A230`, `0x0072A750`, `0x0072E2B0` |
| SC4List | `cISC4DepartmentBudget` | `GetLocallyFundedItemsByPurpose` (32) | `SC4List<cISCPropertyHolder*>&` | `std::list` (B, head and nodes); the game caller frees nodes and head with B | crash | callers `0x00785740`, `0x00786690`; original `0x006AA9C0` |
| SC4List | `cISC4TractDeveloper` | `Grow` (5) | `SC4List<Outcome>&` | `std::list` (B); the caller allocates the head and frees every node with B | crash | tract-developer; allocators.md table (2) |
| SC4List | `cISC4Demolition` | `DemolishLots` (7) | `SC4List<cISC4Lot*> const&` | `std::list` (B by declaration), read only by the original. Compatible for reading; the tract developer builds and frees its list with B | cosmetic | original `0x007E98C0` |
| SC4Vector | `cISC4BuildingDevelopmentSimulator` | `GetAllBuildingTypes` (11), `GetAllBuildingFamilyTypes` (13), `GetBuildingTypesByFamily` (18), `GetAllFoundationTypes` (21), `GetAllLotRetainingWallTypes` (27) | `SC4Vector<uint32_t>&` | `std::vector` (B); the game appends or reassigns with B and may free the caller's buffer | crash | `0x00697C50`, `0x00697D10`, `0x00697EF0`, `0x00698050`, `0x00698180` |
| SC4Vector | `cISC4BudgetSimulator` | `GetAllGroups` (29), `GetDepartmentBudgetsInGroup` (31), `GetAllLoans` (46), `GetBudgetItemInfo` (50) | `SC4Vector<…>&` | `std::vector` (B). In slot 29 each element's name is a `cRZString` with a B character buffer, destroyed inline by the game (see `SC4String` below) | crash | callers `0x007876B0`, `0x00765C30`, `0x0069B910`; originals `0x00691D00`, `0x00692C70`, `0x006937B0` |
| SC4Vector | `cISC4DepartmentBudget` | `GetAllLineItems` (21) | `SC4Vector<cISC4LineItem*>&` | `std::vector` (B); the game reuses one vector that already owns a B buffer | crash | callers `0x007876B0`, `0x007898A0` |
| SC4Vector | `cISC4Simulator` | `EnumerateAgentsByName` (24) | `SC4Vector<SC4String>&` | `std::vector<cRZString>` (B for the buffer and every character buffer). No game caller found | crash | original `0x00704450` |
| SC4Vector | `cISC4FloraSimulator` | `GetAllFloraTypes` (8, vector overload), `GetAllFloraFamilyTypes` (10) | `SC4Vector<uint32_t>&` | `std::vector` (B); the original grows with B and frees the old buffer with B | crash | flora spec; allocators.md table (2) |
| SC4Vector | `cISC4LotConfiguration` | `GetRequiredRoads` (21) | `SC4Vector<bool>&` | `std::vector<bool>` (a bit vector, B), not an array of `bool` | crash | callers `0x00705E64`, `0x00706838` |
| SC4Vector | `cISC4TractDeveloper` | `GetActiveStyles` (9), `SetActiveStyles` (10) | `SC4Vector<uint32_t>&` | `std::vector` (B); Set assigns into the member and may free and reallocate it with B | crash | `0x0070C840`, `0x00707470` |
| SC4Vector | `cISC4PowerSimulator` | `GetMonthlyConsumptionMapPtr` (18) | `SC4Vector<SC4Vector<int16_t>>*&` | `std::vector<std::vector<int16_t>>` (B, outer and rows). Layout matches; only code that grows or frees it is affected. No game caller found | cosmetic | allocators.md table (3) |
| hash set | `cISC4BuildingDevelopmentSimulator` | `GetBuildingTypesByFilter` (17), `GetBuildingTypesByFamily` (19, set overload) | `std::unordered_set<uint32_t>&` | STLport `hash_set<uint32_t>` (0x14 bytes, B). `std::unordered_set` has a different layout in every compiler's library | crash | `0x00699C50`, `0x00699D20` |
| hash set | `cISC4LotConfigurationManager` | `GetLotConfigurationsByFilter` (9) and the other `SC4HashSet` methods | `SC4HashSet<…>&` | STLport `hash_set` (B). The SDK type has the wrong layout and the wrong allocator (below) | crash | `0x0048B1A0`; caller `0x00709650` |
| hash set | `cISC4FloraSimulator` | `GetFloraTypesByFilter` (13), `GetFloraTypesByFamily` (14) | `intptr_t&` | STLport `hash_set<uint32_t>&` (B). The SDK type gives no usable declaration | cosmetic | `0x006B23A0`, `0x006B2460` |

## SDK sources (`src/`)

From `docs/specs/foundations.md` §5.1. The standard message vtable itself matches the SDK declaration order exactly; these are behaviour differences in the SDK's own implementation.

| Class | Detail | Severity |
|---|---|---|
| `cRZMessage2Standard` | copies strings where the original stores a reference to the `cIGZString` object | wrong value |
| `cRZMessage2Standard` | does not implement `GetExtra` / `SetExtra` | wrong value |
| `cRZMessage2Standard` | `GetHasData2..4` test the wrong bit | wrong value |
| `cIGZMessage2Standard::GetString`, `GetExtra` | the first parameter is an interface ID (`QueryInterface` on the stored object; `GZIID_cIGZString` = `0x089B7DC8`), not `dwMaxLen` | cosmetic |

## Checked and correct under MSVC (not errata)

These looked wrong when the header text was compared slot-by-slot with the game, but MSVC's overload placement (each overload group at the position of its first declaration, members in reverse declaration order) produces the game's order. They would be wrong for a non-MSVC compiler, which is outside the SDK's supported toolchain.

- `cISC4App::SaveCity` (15/16): the whole interface matches `cSC4App`'s vtable by pop.
- `cISC4LotManager::GetLot` (5/6) and `GetLotsSurroundingLot` (8/9).
- `cISC4OccupantManager::GetOccupantsByBBox` (36/37) and the other filter/typed pairs.
- `cISC4PoliceSimulator::GetJailCapacity` (28/29) and the apparent shift of slots 30-35.
- `cISC4PowerSimulator` 16/17 and 23/24; `cISC4AuraSimulator::AddAuraValue` (6/7, types aside); `cISC4BudgetSimulator` 18/19 and 27/28.
- `cISCStringDetokenizer::Detokenize` (8/9); `cISC4MySim::GetJobTitle`.
- `cIGZCommandServer::RegisterCommand` (slot 6, `0x0092B075`, pop 12): the vendored header already declares `(uint32_t, const char*, const char*)`.

## Uncertain (not counted)

The ABI extractor records a return channel for these slots that disagrees with the SDK, but no spec confirms it, and for some a spec contradicts it. The extractor has known false positives: a function that only copies a `double` argument looks like an x87 return (confirmed for `cISCLua::PushNumber` and for `cISC4PollutionSimulator` slot 55, which returns in AL). Treat these as leads, not errata.

| Interface | Slot | ABI table | Note |
|---|---|---|---|
| `cIGZVariant` | 41 | x87 (high) for `void SetValFloat64(double)` | probably the same artefact as `cISCLua::PushNumber` (a function that only copies its double argument); FC4 returns the value anyway |
| `cISC4AutomataControllerManager` | 23 | x87 (high) for a pointer return | not checked by a spec |
| `cISC4City` | 96 | x87 (high) for `void CellCornerToPosition(...)` | the landfill spec lists slot 97 with no return value |
| `cISC4City` | 97 | x87 (high) for `void CellCenterToPosition(...)` | as slot 96 |
| `cISC4Demand` | 16 | x87 (high) for `bool AddToSupplyValue(float)` | FC4Interfaces.h follows the ABI table, but the demand spec says the method returns true; a function that adds its float argument can look like this to the extractor |
| `cISC4Demand` | 17 | x87 (high) for `bool AddToDemandValue(float)` | as slot 16 |
| `cISC4DemandSimulator` | 8 | x87 (high) for `void CalculateJobsPerUnitOfDemand(float*, uint32, uint32)` | the method writes floats to its output array; not confirmed by the demand spec |
| `cISC4Lot` | 15 | x87 (high) for `bool GetCenter(float*) const` | FC4Interfaces.h follows the ABI table; the method writes floats to its output; no spec states a return value |
| `cISC4NeighborsSimulator` | 36 | `x87\|eax` (low) for `int64 GetDealMaxAmount` | FC4Interfaces.h chooses `double` [choice]; not called by the game |
| `cISC4NetworkTool` | 33 | x87 (high) for `void GetBridgeAltitudeOverride()` | not checked by a spec |
| `cISC4OccupantManager` | 14 | x87 (high) for a `bool` coordinate conversion | not checked by a spec; a `float&`-output function can look like this to the extractor |
| `cISC4OccupantManager` | 15 | x87 (high) for a `bool` coordinate conversion | not checked by a spec; a `float&`-output function can look like this to the extractor |
| `cISC4OccupantManager` | 16 | x87 (high) for a `bool` coordinate conversion | not checked by a spec; a `float&`-output function can look like this to the extractor |
| `cISC4OccupantManager` | 17 | x87 (high) for a `bool` coordinate conversion | not checked by a spec; a `float&`-output function can look like this to the extractor |
| `cISC4OccupantManager` | 18 | x87 (high) for a `bool` coordinate conversion | not checked by a spec; a `float&`-output function can look like this to the extractor |
| `cISC4PropOccupant` | 14 | x87 (high) for `void GetTimeOfDayRange(float&, float&)` | not checked by a spec |
| `cISC4TrafficSimulator` | 11 | x87 (high) for `intptr_t GetBackgroundTraffic(int, int)` | not checked by a spec |
| `cISC4TrafficSimulator` | 23 | x87 (high) for `uint32_t GetCapacity(...)` | not checked by a spec |
| `cISC4WeatherSimulator` | 11 | x87 (high) for `void CellToPosition(int32, int32, float&, float&)` | FC4Interfaces.h follows the ABI table; no game code calls it (weather spec §11.2); a float-output function can look like this |
| `cISC4WeatherSimulator` | 13 | `edx:eax` for `void StandardCityCellToCell(int32, int32, int32&, int32&)` | as slot 11; an integer division leaves a value in EDX, which can look like a 64-bit return |
| `cISTETerrain` | 6 | x87 (high) for `uint32_t CellWidth()` | the city's `CellWidthX/Z` return floats in ST0 (power, aura specs); likely real |
| `cISTETerrain` | 27 | x87 (high) for `void GetLerpedNormal(...)` | not checked by a spec |

## Excluded

- x87 false positives: `cISCLua::PushNumber`, `cIGZLua5Thread::PushNumber` (the function copies its `double` argument), `cISC4PollutionSimulator` 55.
- `cIGZCheatCodeManager` 14: an `edx:eax` tag on a `bool` method (extractor artefact).
- Slot-count differences caused only by an implementation's destructor or class-only slots after the interface's last method.
- Ambiguous interface-to-vtable mappings: `cIGZMouseMessage`, `cISC4DBSegmentIStream`, `cISCExemplarPropertyHolder`.
- Parameter names, unless they mislead about the meaning or the number of values.

## How we verified

- **ABI tables.** `docs/abi/interfaces.tsv` (game interface → implementing vtable, one row per slot) and `docs/abi/vtables.tsv` (every vtable in the executable) were extracted from `SimCity4.exe` with REA and Ghidra. For each slot they give the target function, the number of bytes the callee pops (`ret n`, exact), and the return channel inferred from the epilogue and callers: `x87` (ST0), `edx:eax` (64-bit), `eax-or-void`, with a confidence.
- **SDK model.** A script parsed every SDK interface header and laid out its vtable the way MSVC does: base-class slots first, then new virtuals in declaration order, with all overloads of one name placed together at the position of the first declaration, in reverse declaration order (verified against MSVC 2022 output, `docs/specs/foundations.md` §16). For each slot it computed the bytes the callee must pop under `__thiscall` (4 per argument; 8 for `int64_t`/`double`; a by-value struct its size; references and pointers 4; plus 4 for the hidden result pointer when a member function returns a class by value) and the return channel (`float`/`double` → ST0, 64-bit integers → EDX:EAX, others → EAX).
- **Comparison.** The script flagged slot-count differences, pop differences, float/double vs integer return-channel differences, and 64-bit returns without EDX:EAX. Interfaces outside the curated table were matched to the game vtable with the best pop agreement (*vtable sweep*).
- **Confirmation.** Every flag was checked against the project's specs (`docs/specs/*.md`) and the corrected declarations used by the project's own plugin (`engine/plugin/FC4Interfaces.h`). Flags that a spec explains as an extractor artefact were dropped; flags no spec covers are either marked *ABI table only* (pop differences, which are exact) or listed as *Uncertain* (return channels, which are inferred). Several findings were also observed at run time (for example the `GetLocalFundingPercent` access violation).
- **Containers.** Allocator ownership comes from `docs/abi/allocators.md`, which traces each container across the boundary to the allocator calls on both sides and was confirmed in game. The `SC4Vector` and `SC4HashSet` source issues are read from the SDK headers themselves.
- **Limits.** One build (Windows 1.1.641) only. Types of arguments whose size matches (for example `uint32_t` vs a pointer) cannot be told apart by the ABI tables and are reported only where a spec establishes them. Methods the game never calls are reported from the implementation alone.
