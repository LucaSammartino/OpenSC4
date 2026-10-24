# Audio: the sound-event system, the sound managers and the sound data (behaviour spec)

Owner-audit supplement: [§§17–21](audio-owner-contracts.md) inventory every named `cSC4Audio*` owner, describe the
listener, scape, SEM and preferences boundaries, and distinguish settled contracts from open work.

Status: M6 behaviour spec, written 2026-10-16 (re-specs20 lane). Every fact was checked on the Windows
executable `bin/SimCity4.exe` (1.1.641, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`); every address is a Windows address.
Numeric details were checked against the instruction bytes with `tools/x86dis.py` where the decompiler output
was ambiguous or failed. Implementers work from this spec, the public LGPL gzcom-dll SDK headers and general
knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags:

- *[compat]*: observable; a replacement must match it.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: consistent with all the evidence, but not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.
- *[unsettled]*: open; see §15.

Scope. The audio layer of the game (INVENTORY-M6 §5, part **A1**). This spec covers, in order of what an
implementer needs first:

1. **The sound-event system** — `cGZSndEventStd`, the pooled sound-event object, and the pool it is allocated
   from (§4, §5).
2. **How a sound is requested and stopped** — the automaton SFX slots, the messages they post, and the handler
   that receives them (§6, §7).
3. **The SC4 sound manager and the audio event handler** — the objects those messages are delivered to (§8).
4. **The effects system's sound component** (§9).
5. **The sound data** — where sound events and instances live in the shipped packages (§10).
6. **The class inventory** — the 92 classes the audio director registers (§11).

Out of scope and covered elsewhere: the simulation side of automata states ([automata.md](automata.md) §6.2),
the effects component record ([effects.md](effects.md) §2), message delivery mechanics
([foundations.md](foundations.md) §5), and the city save ([save-records.md](save-records.md)). This spec does
**not** cover the EA sound library internals (SNDSYS / AEMS / the mixer at `0x00864000`–`0x00879000`,
INVENTORY-M6 §0.1), the Hit sequenced-music engine, the radio player or the freshness score — those are named
in §2 and §11 so an implementer can see the boundary, and are left to a later spec (§15, **AU8**).

---

## 1. Identity

### 1.1 Directors

The audio layer is five COM directors. `cSC4AudioCOMDirector` is the one `cSC4App::AddCOMDirectors`
(`0x004493B0`) adds; it adds the other four from its `OnStart`.

| Director | Director ID | Constructor | Registers | Notes |
|---|---|---|---|---|
| `cSC4AudioCOMDirector` | `0xFDB5CA17` | `0x004EC710` | 92 classes | GetDirectorID `0x004EB1A0` returns `0xFDB5CA17`; OnStart `0x004EB1B0` |
| `cGZSoundCOMDirector` | `0xE9D3A40B` | `0x00966498` | 8 classes | GetDirectorID `0x00966539`; OnStart `0x0096655A`; PreAppShutdown `0x0096653F` |
| `cHitManCOMDirector` | `0x9B2D3D2F` | `0x00935F28` | 10 classes | GetDirectorID `0x00935FEB`; OnStart `0x00935FF1`; PreAppInit `0x00936090` |
| `cFreshnessCOMDirector` | `0xFD53FF17` | `0x009456EC` | 5 classes | GetDirectorID `0x00945630` |
| `cGZSndSysMacCOMDirector` | — | — | — | added by `cSC4AudioCOMDirector::OnStart` (`0x004EB1B0`); also `cGZToolsCOMDirector` |

`cSC4AudioCOMDirector::OnStart` (`0x004EB1B0`) registers the sub-directors **in this order**, calling each one's
`OnStart` (slot 4, `+0x10`) immediately after adding it: `GetGZSoundCOMDirector()`, `GetGZSndSysMacCOMDirector()`,
`cGZToolsCOMDirector::Get()`, `GetHitManCOMDirector()`, `cFreshnessCOMDirector::GetDirector()` *[compat]*.

`cSC4AudioCOMDirector::PostAppInit` (`0x004EBF40`) then brings up the SC4 audio services: it loads the AEMS
banks under group `0x8A5971C5` (with a second key `0xEA8A1115`) and creates the system services listed in §1.3.

### 1.2 Class IDs

Measured from the `AddCls` calls in the four constructors (`cRZCOMDllDirector::AddCls`, `0x0090E133` in the
SC4 director's case) and from the game's own ID→name table (`re/class_id_table.tsv`).

| CLSID | Class | Factory | Director |
|---|---|---|---|
| `0xE41CE9FD` | `cGZSndSys` | `cGZSndSys::Create` | sound |
| `0x7AF85B98` | `cGZSndMgr` | `cGZSndMgr::Create` | sound |
| `0xDC1F73D6` | unnamed | `FUN_00966411` | sound |
| `0x1D07EB4B` | unnamed | `FUN_0096647F` | sound |
| `0xBB7051F5` | unnamed | `FUN_00966469` | sound |
| `0x9D4C6B95` | unnamed | `FUN_00966427` | sound |
| `0x7DC495CC` | unnamed | `FUN_00966453` | sound |
| `0xBDF613B4` | unnamed | `FUN_0096643D` | sound |
| `0x7B1BD993` | `cHitMan` | `CreatecHitMan` | Hit |
| `0x3BC9E6E0` | `cHitDebugInfo` | `CreatecHitDebugInfo` | Hit |
| `0x5B945038` | unnamed | `FUN_009358C0` | Hit |
| `0xDC45DCB1` | unnamed | `FUN_00935DC5` | Hit |
| `0x2B2EB869` | `cHitSegmentDataSourceChannelCreator` | `CreatecHitSegmentDataSourceChannelCreator` | Hit |
| `0xA3CD96CF` | unnamed | `FUN_00935886` | Hit |
| `0x3B1EA5BE` | `cHitChannel` | `CreatecHitChannel` | Hit |
| `0x3D802D67` | `cHitIniFile` | `cHitIniFile::Create` | Hit |
| `0xABA06EF4` | unnamed | `FUN_00935EBD` | Hit |
| `0x7B1ACFCD` | unnamed (Hit lists) | `FUN_009358D6` | Hit |
| `0x3C1CF59E` | `cFreshPlayer` | `cFreshPlayer::Create` | freshness |
| `0x5C1CF5A3` | `cFreshScore` | `cFreshScore::Create` | freshness |
| `0xBDDA72DD` | unnamed | `FUN_009456C8` | freshness |
| `0x7DD9C9F2` | `cFreshCell` | `cFreshCell::Create` | freshness |
| `0xBDD9D7A5` | unnamed | `FUN_0094568E` | freshness |
| `0xAA355871` | `cSC4AudioSemManager` | `0x004ED700` | SC4 audio |
| `0x29E2797F` | `cSC4AudioScapeManager` | `0x004EB120` | SC4 audio |
| `0xBDB6171E` | `cSC4AudioEventHandler` | `0x004EB0A0` | SC4 audio |
| `0x69E500BC` | `cSC4AudioEventHandlerDisasterFire` | `0x004EB100` | SC4 audio |
| `0x6A12F51C` | `cSC4AudioAmbienceManager` | `0x004EB0C0` | SC4 audio |
| `0x4A47DC0D` | `cSC4AudioSoundManager` | `0x004EB080` | SC4 audio |
| `0xCA4A66B9` | `cSC4AudioListener` | `0x004EB0E0` | SC4 audio |
| `0x5CFBDBBF` | `cRZRadio` | `0x004EB150` | SC4 audio |
| `0x5CFBDBD5` | `cRZDiskRadioStation` | `0x004EB180` | SC4 audio |

The remaining 83 classes registered by `cSC4AudioCOMDirector`'s constructor are covered in §11.

**Corrects:** INVENTORY-M6 §5 lists the SC4 audio managers but omits the class IDs for the scape and sem
managers. Those are `0x29E2797F` and `0xAA355871` (measured from `cSC4AudioCOMDirector`'s constructor
`0x004EC710`, and both appear in `re/class_id_table.tsv`). **MATCHES** for every ID M6 does give.

### 1.3 Service IDs and interface IDs

`re/class_id_table.tsv` names these; the values are used at the call sites cited.

| ID | Name | Used at |
|---|---|---|
| `0xAA12F911` | `kGZGUID_AudioAmbienceManagerSystemServiceId` | service registry |
| `0x49E50BA1` | `kAudioScapeManagerSystemServiceID` | service registry |
| `0xAC1FAE31` | `GZGUID_AEMSEventsSystemServiceId` | service registry |
| `0x1D07B39F` | `kRZRadioDefaultServiceId` | service registry |
| `0x287FB697` | (service IID) | `cGZSoundCOMDirector::OnStart` `0x0096655A` creates `cGZSndSys` (`0xE41CE9FD`) and `cGZSndMgr` (`0x7AF85B98`) under this IID |
| `0xBB523BB2` | (service IID) | `cGZSoundCOMDirector::OnStart` creates `0x7DC495CC` under it; `cSC4AudioCOMDirector::PostAppInit` `0x004EBF40` creates `cSC4AudioEventHandler` (`0xBDB6171E`) and `cSC4AudioEventHandlerDisasterFire` (`0x69E500BC`) under it |
| `0x3AF85BB7` | `cIGZSndMgr` IID | `cGZSndMgr::QueryInterface` `0x00968A9D` answers it, returning `this+0x1C` |
| `0x5B523BA4` | sound-event IID | `cGZSndEventStd::QueryInterface` `0x0096BA2B` answers it, returning `this+0x2C` |
| `0x89E51472` | (audio interface) | queried in `cSC4AudioCOMDirector::PostAppInit` `0x004EBF40` |
| `0xA9B40F05` | (position interface) | queried by `cSC4AudioEventHandler::HandleAutomataMessages` `0x004EE560` |
| `0x65297976` | (message payload interface) | queried on the message by `HandleAutomataMessages` |
| `0xDB53383B` | `kSndEventMsgTypeStd` | the message type a `cGZSndEventStd` carries (`cGZSndEventStd` ctor `0x0096BA6D`) |
| `0x2A3AD653` | `kMessageSetRadioStation` | posted by `cSC4AudioEventHandler::DoMessage` `0x004EF810` on message `0x26D31EC5` |
| `0x8A4A69C7` | `kMessageListenerPosChanged` | the listener follows the camera |

---

## 2. The layers at a glance

INVENTORY-M6 §5's five-layer table stands (**MATCHES**). For orientation only; the layers this spec details are
marked.

| Layer | Classes | Detailed here |
|---|---|---|
| framework sound system | `cGZSndSys` (`0xE41CE9FD`), `cGZSndMgr` (`0x7AF85B98`), WAV and "UTalk" data sources | §3.2, §8.3 |
| EA sound library (SNDSYS / AEMS) | the unnamed range `0x00864000`–`0x00879000`, `cSC4AudioAEMSEventSystem` | §10.2 |
| Hit (sequenced music and tracks) | `cHitMan` (`0x7B1BD993`), `cHitTrackPlayer` | §11 (boundary only) |
| freshness (adaptive score) | `cFreshPlayer`, `cFreshScore`, `cFreshCell` | §11 (boundary only) |
| radio | `cRZRadio` (`0x5CFBDBBF`), `cRZDiskRadioStation` (`0x5CFBDBD5`) | §11 (boundary only) |
| SC4 audio managers | `cSC4AudioSoundManager` and the other §1.2 classes | **§5–§9** |

---

## 3. State

### 3.1 `cGZSndEventStd` — the sound-event object

A sound event is a **message object**. It derives from `cRZMessage2Standard` and is allocated from a dedicated
pool. Constructor `0x0096BA6D`, default constructor `0x0096BB0E`, `QueryInterface` `0x0096BA2B`,
`operator new` `0x0096BAD1`.

Object size is **`0x38` (56 bytes)** — measured: `cGZSndEventStd::operator.new` (`0x0096BAD1`) compares the
requested size against `0x38` and only takes the pool when it matches.

| Offset | Type | Set by | Meaning |
|---|---|---|---|
| `+0x00` | vtable | ctor `0x0096BA6D` | `0x00AD75A0` |
| `+0x04`…`+0x2B` | `cRZMessage2Standard` base | `cRZMessage2Standard::cRZMessage2Standard` | the message payload; see §3.2 |
| `+0x2C` | vtable | ctor | `0x00AD7580` (`ZThn44_cGZSndEventStd_QueryInterface`); a **second interface at offset `+0x2C`** |
| `+0x30` | `u32` | ctor argument 2 | the event's payload word |
| `+0x34` | `u32` | ctor `0x0096BA6D` sets it to **1**; the default ctor `0x0096BB0E` sets it to **0** | a flag / count |

`QueryInterface` (`0x0096BA2B`) answers IID `0x5B523BA4` by returning `this+0x2C` and calling `AddRef` (slot 1);
every other IID falls through to `cRZMessage2Standard::QueryInterface`. The `+0x2C` interface is therefore a
**secondary base whose vptr lives inside the object**, not at `+0x00` *[compat]*.

**The secondary interface at `+0x2C`** is a payload accessor. Its vtable is `0x00AD7580`, immediately before
the primary `0x00AD75A0`, and has **seven** methods (slot 7 is a null word, not a method):

| Slot | Offset | Impl | Body | Meaning |
|---|---|---|---|---|
| 0 | `+0x00` | `0x0096BA55` | `sub ecx, 0x2C; jmp 0x0096BA2B` | QueryInterface (this-adjusting thunk) |
| 1 | `+0x04` | `0x0096BA5D` | `sub ecx, 0x2C; jmp 0x00940508` | AddRef (thunk) |
| 2 | `+0x08` | `0x0096BA65` | `sub ecx, 0x2C; jmp 0x0096BA22` | Release (thunk → `0x009132C1`) |
| 3 | `+0x0C` | `0x00891D96` | `mov al, [ecx+8]; ret` | **GetFlag** → the outer `+0x34` byte |
| 4 | `+0x10` | `0x009D7E63` | `mov eax, [ecx+4]; ret` | **GetValue** → the outer `+0x30` dword |
| 5 | `+0x14` | `0x0096BA02` | `[ecx+4] = arg; [ecx+8] = 1; ret 4` | **SetValue(u32)** and mark it set |
| 6 | `+0x18` | `0x0096BA27` | `lea eax, [ecx-0x2C]; ret` | **GetOuter** → `this - 0x2C` |

So the sound event carries one `u32` payload (`+0x30`) with an accompanying "was set" byte (`+0x34`), and
slot 5 is the setter that also sets that byte. The two constructors agree: the five-argument one writes its
argument 2 into `+0x30` and `1` into `+0x34`; the default one zeroes both *[compat]*.

The five-argument constructor `0x0096BA6D` sets the message type to `kSndEventMsgTypeStd`
**`0xDB53383B`** (`FUN_009132db(0xdb53383b)` inside it), then stores its arguments through the message
setters `0x00913322` (data1), `0x00913330` (data2), `0x0091333E` (data3) and `0x0091334C` (data4).

### 3.2 `cRZMessage2Standard` — the message payload

Vtable `0x00ACEC00`, object size **`0x2C` (44 bytes)** (measured: every sound message built in
`cSC4Automaton::StartSFX`/`StopSFX` and in `cSC4AudioEventHandler::DoMessage` is `operator_new(0x2c)`).

The setters each OR one bit into a presence-flag byte at `+0x08`. This is the compatibility surface a
replacement must reproduce *[compat]*.

| Offset | Type | Presence bit | Setter slot | Setter impl | Meaning |
|---|---|---|---|---|---|
| `+0x00` | vtable | — | — | — | `0x00ACEC00` |
| `+0x04` | `u32` | — | — | — | refcount (`cRZRefCount`) |
| `+0x08` | `u8` | — | — | — | **presence flags** (see below) |
| `+0x0C` | `u32` | bit 0 (`0x01`) | 5 (`+0x14`) | `0x009132DB` | **type** |
| `+0x10` | `u32` | bit 1 (`0x02`) | 11 (`+0x2C`) | `0x00913322` | **data1** |
| `+0x14` | `u32` | bit 2 (`0x04`) | 16 (`+0x40`) | `0x00913330` | **data2** |
| `+0x18` | `u32` | bit 3 (`0x08`) | 21 (`+0x54`) | `0x0091333E` | **data3** |
| `+0x1C` | `u32` | bit 4 (`0x10`) *[inferred]* | 26 (`+0x68`) | `0x0091334C` | **data4** |
| `+0x24` | `void*` | bit 7 (`0x80`) | 33 (`+0x84`) | `0x0091346B` | **void1 / target** |

Measured setter bodies (all `__stdcall`, `ret 4`):

- `0x009132DB` `SetType(u32 t)`: `[this+0x0C] = t`; if `t == 0` clear bit 0 else set it. **Type 0 is treated as
  "no type"** and does not set the presence bit *[quirk]*.
- `0x00913322` `SetData1(u32 v)`: `[this+0x10] = v`; `flags |= 0x02`.
- `0x00913330` `SetData2(u32 v)`: `[this+0x14] = v`; `flags |= 0x04`.
- `0x0091333E` `SetData3(u32 v)`: `[this+0x18] = v`; `flags |= 0x08`.
- `0x0091334C` `SetData4(u32 v)`: `[this+0x1C] = v`; `flags |= 0x10` *[inferred bit]*.
- `0x0091346B` `SetVoid1(void* p)`: if `p != 0` set bit 7 else clear it, then write at `this+0x24`.

Slots 12, 17, 22, 27 of the same vtable are the **getter twins** of 11, 16, 21, 26 — they share the same
implementation address (`0x00913322`, `0x00913330`, `0x0091333E`, `0x0091334C`) in the exported vtable rows
(`docs/abi/interfaces.tsv`); the pairing is *[inferred]* to be set/get over the same slot, and the argument
direction is what distinguishes them. Full row set: `docs/abi/interfaces.tsv` lines for `cIGZMessage2Standard`
vtable `00ACEC00`, slots 0–37.

### 3.3 The automaton's SFX fields

Two fields of `cSC4Automaton` carry the sound state. Both are **unsaved** (§12).

| Offset | Type | Written by | Read by | Meaning |
|---|---|---|---|---|
| `+0x8C` | `u32` | `StartSFX` `0x00509950` | `StopSFX` `0x00505B50` | the **sound instance id** returned by `cSC4AudioSoundManager::GenerateNewInstanceId` (`0x004F7BB0`) |
| `+0x84` | `u32` | — | `HandleEnterState` `0x00511450` | the automaton's flag word; copied into `state+0x20` on state entry |

The state base record's `+0x18` is the **SFX id** *[see §6.1]*.

### 3.4 Emanation models

`cSC4AudioEmanationModel2D` (ctor `0x004ED8A0`), `cSC4AudioEmanationModel3D` (ctor `0x004ED9A0`) and
`cSC4AudioEmanationModel3DEffects` (ctor `0x004EDA00`) share a layout; the constructors differ only in the
name string and the constant at `+0x20`.

| Offset | Type | 2D | 3D | 3DEffects | Meaning |
|---|---|---|---|---|---|
| `+0x00` | vtable | `0x00A94068` | `0x00A940A4` | `0x00A940E4` | primary |
| `+0x04` | vtable | `0x00A94054` | `0x00A94090` | `0x00A940D0` | secondary base |
| `+0x08` | `u32` | 0 | 0 | 0 | |
| `+0x0C` | vtable | `0x00A80810` | — | `0x00A80810` | a third base (`QueryInterface`) |
| `+0x10`, `+0x14`, `+0x18` | `u32` | 0 | 0 | 0 | |
| `+0x1C` | `u32` | 0 | 0 | 0 | |
| `+0x20` | `u32` | **`0x7DCDED2D`** | **`0x7DCDED3D`** | **`0x0A63A543`** | a constant id |
| `+0x24` | `u32` | 0 | 0 | 0 | |

Each constructor builds a name string with `FUN_00407FA0(begin, end, out)` (a `std::string` range construct).
The names, read from the binary: **`"2D SC4K"`** (bytes `0x00A9404C`–`0x00A94053`), **`"3D SC4K"`**
(`0x00A94088`–`0x00A9408F`), **`"3DEffect"`** (Ghidra resolves the literal at the 3DEffects constructor
`0x004EDA00`). *[compat]*: these three strings are the identity of the emanation model, not display text.

---

## 4. The sound-event pool

This is the pool named in `docs/runs/run-9.md` ("crash at month end in the game's sound-event pool
(`exe+0x1E16F5`)").

**Corrects:** that note is imprecise about the address. `exe+0x1E16F5` is a module-relative offset; the virtual
address is `0x00400000 + 0x1E16F5` = **`0x005E16F5`**, which is `FUN_005E16E0+0x15`. The note's diagnosis
("heap corruption symptom") **MATCHES** — see §4.3.

### 4.1 The pool object

`cGZSndEventStd::operator.new` (`0x0096BAD1`) is the only entry into the pool for sound events:

```
cmp   dword ptr [ebp+8], 0x38      ; only the 56-byte size uses the pool
jne   0x0096BB03                   ;   otherwise ordinary operator new
push  0x00B63618                   ; lock object
lea   ecx, [ebp-8]
call  0x0048C9F0                   ; cRZLock::cRZLock(&lock)  — RAII
mov   ecx, 0x00B6360C              ; THE POOL
call  0x005E16E0                   ; allocate
...
call  dword ptr [eax+8]            ; release the lock
```

- **Pool object: global `DAT_00B6360C`.**
- **Pool lock: global `DAT_00B63618`** (`cRZLock`, taken around every allocation and release).
- Allocations of any size other than `0x38` bypass the pool entirely and go to the ordinary heap
  (`operator new` at `0x009133DA`) *[compat]*.

### 4.2 Pages

The pool hands out objects from **16 KB-aligned pages**. A page carries a small header and a LIFO free list of
objects; each free object stores the next pointer in its first dword.

| Page offset | Type | Written by | Meaning |
|---|---|---|---|
| `+0x00` | `u32` | `FUN_0090CF9A` `0x0090CF9A` | ring link (prev) |
| `+0x04` | `u32` | `FUN_0090CF9A` | ring link (next) |
| `+0x08` | `void*` | `FUN_005E16E0`, `FUN_005E1780` | **free-list head** |
| `+0x0C` | `u32` | `FUN_0090CF9A`, `FUN_005E16E0`, `FUN_005E1780` | **count / state word** |

A free object's `+0x00` holds its successor. The page a pointer belongs to is recovered as
**`p & 0xFFFFC000`** (16 KB alignment) — measured in `FUN_005E1780` (`and eax, 0xffffc000`).

**Allocate `FUN_005E16E0` (`0x005E16E0`, `__thiscall`, ECX = pool):**

```
mov  eax, [esi+4]        ; pool+4 -> current page
mov  eax, [eax+8]        ; free-list head
test eax, eax
jne  have_node
call 0x0090CF9A          ; grow: take a page and return a node
have_node:
mov  ecx, [esi+4]
mov  edx, [eax]          ; *** 0x005E16F5 — the run-9 crash site ***
mov  [ecx+8], edx        ; free-list head = node->next
mov  esi, [esi+4]
inc  dword ptr [esi+0xc] ; page count ++
```

**Release `FUN_005E1780` (`0x005E1780`, `__stdcall`, `ret 4`):**

```
mov  edx, [esp+4]        ; p
test edx, edx
je   done                ; null is a no-op
mov  eax, edx
and  eax, 0xffffc000     ; page = p & ~0x3FFF
mov  esi, [eax+8]
mov  [edx], esi          ; p->next = head
mov  [eax+8], edx        ; head = p
mov  edx, [eax+0xc]
dec  edx
mov  [eax+0xc], edx
jne  check_canary        ; not zero
mov  [esp+4], eax
jmp  0x0090D031          ; count hit 0: give the page back (pool vtable slot 3)
check_canary:
cmp  edx, 0x40000045
jne  done
mov  [esp+4], eax
jmp  0x0090CFF9          ; special path: clear the 0x40000000 marker and unlink
done:
ret  4
```

**Grow `FUN_0090CF9A` (`0x0090CF9A`)** walks a doubly-linked ring of pages (`pool+4` is the cursor). If the
cursor's page is its own successor the ring is exhausted and it calls the pool's **vtable slot 2 (`+0x08`)**
to obtain a new page, which it links into the ring. It then unlinks the chosen page and does
**`add dword ptr [page+0x0C], 0x40000000`** — the page's state word acquires the `0x40000000` marker when it
comes into service *[compat]*.

**Release paths.**

- `FUN_0090D031` (`0x0090D031`): unlinks the page from the pool's ring and calls the pool's **vtable slot 3
  (`+0x0C`)** with it — the page goes back to the heap. It is a no-op when the page is already a self-loop.
- `FUN_0090CFF9` (`0x0090CFF9`): does **`add dword ptr [page+0x0C], 0xC0000000`** (i.e. clears the
  `0x40000000` marker), unlinks the page from the ring, and moves it onto a second list at `pool+4`. This is
  the "page leaves active service but is not yet returned" path.

**What the `0x40000045` threshold means is *[unsettled]*.** Measured: it is compared against the post-`dec`
value of `page+0x0C`, and it is `0x40000000 | 0x45`. *[inferred]* it is a watermark meaning "the page has
fallen to `0x45` live objects while still marked in service, retire it early". What would settle it: reading
the pool's vtable slot 2 implementation to see the initial value written to `page+0x0C`, or logging the
field over a session.

### 4.3 Why run 9 crashed

The crash address `0x005E16F5` is the instruction `mov edx, dword ptr [eax]` in `FUN_005E16E0`, which
dereferences the free-list node just popped from `page+0x08`. If `page+0x08` holds a stale or overwritten
pointer — the classic signature of a heap overflow into the page header or of a double free that rewrote a
node's successor — this load faults. The pool itself is sound; the corruption is upstream *[inferred]*.

A replacement that keeps the same allocation shape must therefore reproduce:

1. the 16 KB page alignment of the free list (`p & 0xFFFFC000`),
2. the LIFO order of the free list (a freed object becomes the new head),
3. the null-free in `FUN_005E1780`,
4. the two thresholds on the state word (§4.2).

Only (2) and (3) are observable through the sound API; (1) and (4) are observable through the crash
behaviour *[compat]*.

---

## 5. The sound-event system

`cGZSndEventStd` is the payload of the sound-event message type **`kSndEventMsgTypeStd` `0xDB53383B`**. Its
five-argument constructor stamps that type and fills data1..data4 (§3.1). Nothing in the decompiled audio
layer constructs a `cGZSndEventStd` outside its own file; the class is created through the COM factory
mechanism and consumed as a `cRZMessage2Standard`-shaped message *[inferred from the absence of direct
callers; what would settle it: a complete caller list of `FUN_005E16E0` beyond `decomp/cGZSndEventStd.c` and
`thunk_FUN_005E16E0` at `0x004F5660`]*.

`cGZSndEventStd::operator.new` is also referenced through vtable thunks — `FUN_005E58F0` (`0x005E58F0`)
installs `PTR_thunk_FUN_005E16E0_*` into eight consecutive 12-byte slots of a `cRZCriticalSection`-shaped
object, alongside `FUN_0090D056` calls, and `PTR_PTR_00B098B4` is set to two of those thunks from
`0x00A70000`-region code. The eight-slot object is a **pool-owner array**; the thunks are the pool's
allocate methods bound to each pool *[inferred]*.

---

## 6. How a sound is requested and stopped

This is the concrete caller the automata layer already records.

### 6.1 The SFX id

The automaton state base record's `+0x18` is a `u32`. It is the **SFX id** *[compat]*:

- `cSC4Automaton::HandleEnterState` (`0x00511450`) does, when the state pointer is non-null and
  `state+0x18 != 0`:
  `this->vtable[0x3D](*(u32*)(state + 0x18))` — slot index `0x3D` = **61**, offset **`+0xF4`**, i.e.
  `StartSFX(state+0x18)`.
- `cSC4Automaton::HandleExitState` (`0x0050E3F0`) does, when `param_2[6] != 0` (that is `*(u32*)(state+0x18)`):
  `this->vtable[0x3E](param_2[6])` — slot index `0x3E` = **62**, offset **`+0xF8`**, i.e. `StopSFX`.

Both are guarded on the field being non-zero, so **SFX id 0 means "this state makes no sound"** and neither
slot is called *[compat]*.

**Corrects:** [automata.md](automata.md) §6.2 lists `+0x18` as a bare `u32` with no name and
[automata.md](automata.md) §11 names slots 61–62 `Start/StopSFX` without saying where the id comes from. The
field **is** the SFX id and the two calls above are the only users of it. **MATCHES** on the slot names and
offsets; this spec adds the field's meaning.

### 6.2 `StartSFX` (slot 61, `+0xF4`, `0x00509950`)

`__thiscall`, **`ret 4`** (one `u32` argument), returns `1` on success and `0` when the id is 0.

| Step | Address | What happens |
|---|---|---|
| 1 | `0x00509953` | read the argument (the SFX id) |
| 2 | `0x0050995B` | if it is 0, return `0` immediately |
| 3 | `0x00509968` | `call 0x004ECD60` (a method on the automaton; its return value only selects a dead branch, see the quirk below) |
| 4 | `0x0050998D` | `call 0x004F7BB0` = **`cSC4AudioSoundManager::GenerateNewInstanceId()`**; store the result at `this+0x8C` |
| 5 | `0x00509992` | `operator_new(0x2C)` and `cRZMessage2Standard::cRZMessage2Standard` (`0x009134D6`) — build a message |
| 6 | `0x005099B3` | `msg->slot1()` = `AddRef` |
| 7 | `0x005099BC` | `msg->slot5(0x8A385DD8)` = `SetType(AutomataPlaySFX)` |
| 8 | `0x005099CC` | `this->vtable[5]()`, the automaton's **AsOccupant**, passed to `msg->slot33` (`+0x84`) = `SetVoid1` |
| 9 | `0x005099DA` | `msg->slot11(sfxId)` = `SetData1` |
| 10 | `0x005099E0` | `msg->slot16(*this+0x8C)` = `SetData2` (the instance id) |
| 11 | `0x005099EE` | `(*DAT_00B43CCC)->slot4(msg, 0)` = **message-server `MessagePost(msg, /*highPriority*/ false)`** |
| 12 | `0x005099FC` | `msg->slot2()` = `Release` |
| 13 | `0x00509A03` | return `1` |

### 6.3 `StopSFX` (slot 62, `+0xF8`, `0x00505B50`)

`__thiscall`, **`ret 4`** (one argument), returns `1` always.

| Step | Address | What happens |
|---|---|---|
| 1 | `0x00505B53` | read `this+0x8C`; if it is 0, skip to return `1` |
| 2 | `0x00505B5F` | `operator_new(0x2C)` + `cRZMessage2Standard` ctor |
| 3 | `0x00505B7A` | `msg->slot1()` = `AddRef` |
| 4 | `0x00505B83` | `msg->slot5(0xAA396960)` = `SetType(AutomataStopSFX)` |
| 5 | `0x00505B93` | `this->vtable[5]()` (AsOccupant) → `msg->slot33` = `SetVoid1` |
| 6 | `0x00505B9F` | `msg->slot11(arg1)` = `SetData1` |
| 7 | `0x00505BAB` | `msg->slot16(*this+0x8C)` = `SetData2` (the instance id) |
| 8 | `0x00505BB9` | `(*DAT_00B43CCC)->slot4(msg, 0)` = `MessagePost(msg, false)` |
| 9 | `0x00505BC7` | `msg->slot2()` = `Release` |
| 10 | `0x00505BD0` | return `1` |

### 6.4 The message contract

Both messages are posted **queued, at normal priority** through the message server global **`DAT_00B43CCC`**,
slot 4 (`+0x10`), which foundations §5 identifies as `MessagePost(m, highPriority)` (implementation
`0x004AB778`). Delivery is therefore asynchronous: `StartSFX` and `StopSFX` return before any sound is
started or stopped *[compat]*.

| Field | `AutomataPlaySFX` `0x8A385DD8` | `AutomataStopSFX` `0xAA396960` |
|---|---|---|
| type (`+0x0C`) | `0x8A385DD8` | `0xAA396960` |
| data1 (`+0x10`) | the SFX id | the SFX id (same argument the caller passed) |
| data2 (`+0x14`) | the instance id (`automaton+0x8C`) | the instance id (`automaton+0x8C`) |
| void1 (`+0x24`) | the automaton's occupant | the automaton's occupant |
| flags (`+0x08`) | `0x01\|0x02\|0x04\|0x80` = `0x87` | `0x87` |

`cSC4AudioEventHandler::DoMessage` (`0x004EF810`) routes both types (and `0xA9E3B6E3` and `0xEA3ADA33`) to
`HandleAutomataMessages` (`0x004EE560`) — see §7.

---

## 7. The audio event handler

`cSC4AudioEventHandler`, CLSID `0xBDB6171E`, factory `0x004EB0A0`, **object size `0x50`** (ctor `0x004EF6F0`).
`DoMessage` `0x004EF810`, `HandleAutomataMessages` `0x004EE560`, `Init` `0x004EFE80`, `Shutdown` `0x004EE000`,
destructor `0x004EFC60`, `DoEvent` `0x004EF490`.

### 7.1 Object layout

| Off | Type | Meaning | Evidence |
|---|---|---|---|
| `+0x00` | vtable | `0x00A94164` | ctor `0x004EF70A` |
| `+0x04` | vtable | `0x00A94154` — **modal** message target | `0x004EF710` |
| `+0x08` | vtable | `0x00A94144` — **main** message target | `0x004EF717` |
| `+0x0C` | vtable | `0x00A94130` — service (`Init` / `Shutdown`) | `0x004EF71E` |
| `+0x10` | `*` | `GetService(0x1D07B39F, 0xE0510260)` — **`kRZRadioDefaultServiceId`** | `0x004EF737` |
| `+0x14` | `cIGZSndSys*` | `GetService(0x23CECBA0, 0x106077D2)` — the sound system | `0x004EF757` |
| `+0x18` | refcounted `*` | AudioScape manager, service `0x49E50BA1` / IID `0x29E39DB4`; initially null, acquired by `Init` | `0x004EF767`, `0x004EFF10` |
| `+0x1C` | `u8` | **initialized flag** | `0x004EF76A` |
| `+0x20` | `u32` | refcount | `0x004EDE60`, AddRef `0x008AE800` |
| `+0x24`, `+0x28` | `u32` | constructor writes zero; no handler operation below assigns them | `0x004EF770`–`0x004EF773` |
| `+0x2C` | list node `*` | `std::list` sentinel (12 bytes) | `0x004EF776` |
| `+0x30` | `cRZTimer` | the rate limiter of section 7.3 | `0x004EF78A` |
| `+0x48` | `u32` | rate-limit window (ms) | `0x004EF796` |

Four sub-object vtables; each is QueryInterface / AddRef / Release plus one or two real slots:

| Vtable | At | Slot 3 | Slot 4 |
|---|---|---|---|
| `0x00A94164` | `+0x00` | `DoEvent` `0x004EF490` | deleting dtor `0x004EFC60` |
| `0x00A94154` | `+0x04` | `DoMessage` (modal) `0x004EFE50` | — |
| `0x00A94144` | `+0x08` | `DoMessage` (main) `0x004EF810` | — |
| `0x00A94130` | `+0x0C` | `Init` `0x004EFE80` | `Shutdown` `0x004EE000` |

**Corrects (re-specs20):** an earlier reading of `DoMessage` treated the object it reaches as "a sound manager
at `this+0x08`". That is a Ghidra `this`-base mistranslation: `DoMessage` runs with `this = base+8` (it is
slot 3 of the vtable at `[base+8]`), so the loads Ghidra names `field2_0x8` are object **`+0x10`**, which is
the **`kRZRadioDefaultServiceId` service** (`0x1D07B39F`), not a sound manager. The sound system is at
object `+0x14` (`cIGZSndSys`). **DIFFERS** from that reading.

#### 7.1.1 Construction and service lifetime (AU14)

The constructor leaves the initialized flag, reference count, `+0x18`, `+0x24`, `+0x28` and rate-limit
window at zero. The modal-window collection is empty, with a heap-allocated 12-byte circular sentinel.
The timer is constructed with clock selector **4**, stopped, with its four timing words zero; its
`+0x04` word is not initialized by this constructor (`0x004EF767`–`0x004EF796`, timer constructor
`0x0088FEDF`). Padding at handler `+0x1D`–`+0x1F` and `+0x4C`–`+0x4F` has no assigned value.
The two constructor service lookups in the layout table are independent: a missing framework or failed
lookup leaves that service null and construction continues. Neither is retried into the member by `Init`.
Allocation failure is not a clean false-return path: the sentinel is immediately dereferenced. *[compat]*

`Init` and `Shutdown` receive the service subobject (`base+0x0C`); their `this+0x10` flag is therefore
the **outer `+0x1C`**, not the radio pointer. `Init` returns true without effects if already initialized.
Otherwise its externally visible contract is:

| Condition/stage | Outcome |
|---|---|
| Fresh temporary sound-system lookup fails | False; no registration, no member changes. |
| Sound system available | Register outer event target through sound-system slot 15 (`+0x3C`); ignore its result. |
| Following AudioScape lookup fails | False, release temporary references; **do not unregister** the preceding registration. Flag remains false. A retry repeats registration. |
| AudioScape available | Retain it into outer `+0x18`, replacing/releasing any previous value. This is a retained reference in addition to the temporary lookup reference. |
| Filesystem service `0x2073215D` / IID `0xE0203660` unavailable | Skip radio-path configuration and continue. |
| Filesystem available and member radio non-null | Obtain filesystem slot 7 path; append the runtime string fragment described by globals `0x00B21A0C`/`0x00B21A10`; radio slots 29(path), 23(false), 16(0), in that order. |
| Subscriptions | Install the modal pair then the 13 main subscriptions listed in §7.2. All registration/subscription return values are ignored. |
| Preferences | Run audio preference loading (`0x004F2D40`), apply the default/list preferences, and, if radio exists, radio slots 30(true), 23(music-enabled != 0), 16(music volume). Then set initialized and return true. |

The preference work uses the foreign application audio-preference service, not new handler fields.
On first/default preferences, the observed defaults are slider values `0x2D0` for music and `0x400` for
SFX, reset the default music lists, and update both exclusion lists (`0x004F017D`–`0x004F01CB`).
The conversion and list operations remain preference-service calls. *[compat]*

There is **no** recoverable-failure test for a null global modal message server or the final message-server
lookup: these are required integration services and are dereferenced. Do not invent a successful or false
return, subscription rollback, or partial-subscription retry for their absence. A false result returned by
a present service does not abort initialization (`0x004EFEC8`, `0x004EFFAB`–`0x004F0167`, `0x004F0205`).

`Shutdown` returns true without effects when the flag is false. With the flag true it obtains a fresh
temporary sound system. Failure returns false **without clearing the flag or any members**. Success
unregisters the outer event target (result ignored), clears/releases `+0x18`, calls radio slot 23(false)
if non-null, then releases and nulls the member radio pointer. It removes the 13 main subscriptions and
the modal pair, clears the flag, releases temporary services and returns true. The message servers are
again required, and unsubscribe results are ignored. It does not reset the timer/window, empty the modal
list, clear the member sound-system pointer, or remove the city-only subscriptions. A second successful
shutdown is a no-op. A later Init can reacquire AudioScape but **does not reacquire the now-null member
radio**. Evidence: `0x004EE007`–`0x004EE082`, `0x004EE0B3`–`0x004EE25A`. *[compat]*

### 7.2 Message dispatch (`DoMessage` `0x004EF810`)

A binary search on the message type. A null message returns 0. Ordinary routes return 1 even when their
sub-handler rejects the payload; the mouse route forwards its own boolean (§7.2.4). *[compat]*

| Type | Name | Handler |
|---|---|---|
| `0x89F10866` | | `HandleConstructionMessages` `0x004EEC10` |
| `0xA9F10E45` | | `HandleConstructionMessages` `0x004EEC10` |
| `0x2A611B51` | | `+0x10` service slot 31 (`+0x7C`) |
| `0x26D31EC1` | `kSC4MessagePostCityInit` | `HandleCityInitShutdownMessages` `0x004EE970` (**Add**) |
| `0x26D31EC2` | `kSC4MessagePreCityShutdown` | `HandleCityInitShutdownMessages` (**Remove**) |
| `0x26D31EC4` | | builds a message of type `0x2A3AD653` with data1 = 0 and posts it |
| `0x26D31EC5` | | builds a message of type `0x2A3AD653` with data1 = 1 and posts it |
| `0x2A3AD653` | `kMessageSetRadioStation` | `HandleRadioMessages` `0x004EF010` |
| `0x8A385DD8` | AutomataPlaySFX | `HandleAutomataMessages` `0x004EE560` |
| `0xA9E3B6E3` | | `HandleAutomataMessages` `0x004EE560` |
| `0xAA396960` | AutomataStopSFX | `HandleAutomataMessages` `0x004EE560` |
| `0xEA3ADA33` | | `HandleAutomataMessages` `0x004EE560` |
| `0xAA611B6E` | | obtain `msg->slot10()` interface; pass it to radio slot 7 if radio exists, then release that interface |
| `0xAA650861` | | `msg->slot9()` then the `+0x10` service slot 30 (`+0x78`) as a bool |
| `0xC9F1011A`, `0xE94671EB` | | `LAB_004EFA91` |
| `0xFA0C6390` | `kMouseMessageType` | `HandleMouseMessagesForModalWindow` `0x004EF5D0` |
| `0x49F10111`, `0x894671F6` | | `HandleDispatchManagerMessages` `0x004EE300` |
| `0x49E64801` | | `HandleAutomataMessages` (accepted but no matching inner operation) |
| `0x0A611B34`, `0xAA650860`, `0x4348B111`, `0x4A7FB7E9` | | radio/settings routes described below |

Types absent from this **completed** table have no effects and return 1. In particular the subscribed
`0xC53D10AA` and city-only `0xE94671ED` are ignored; subscription alone does not imply dispatch.
`LAB_004EFA91` is the call to `HandleDispatchManagerMessages`, not an additional algorithm.

#### 7.2.1 Radio and settings messages (AU13)

Radio service absence makes each radio operation a no-op. Payload/interface references are still balanced.
`0x26D31EC4` and `0x26D31EC5` each allocate a standard message, retain it, set type `0x2A3AD653`, set data1
to 0 or 1 respectively, post through global `0x00B43CCC` slot 4 with priority 0, then release their local
reference. Data2 is absent. They queue station selection, rather than selecting immediately. Allocation
failure and a missing post service are unchecked, not false-return cases (`0x004EF87D`–`0x004EF932`).

`HandleRadioMessages` accepts only a non-null message whose QI `0x65297976` succeeds and whose payload type
is `0x2A3AD653`; otherwise it returns false, releasing any acquired interface. Data1 selects **0 → `kmay`**,
**1 → `krgn`**; other values return true without selection. Fade duration is data2 when present, otherwise
**4000** (global `0x00B0988C`). For either recognized station, the selection helper first obtains the
current station through radio slot 10(out), calls radio slot 30(false), resolves the current station through
slot 12(current-name), and compares that result with slot 11(requested-name). If the current result is
non-null and identical, there is no change call. Otherwise call slot 13(requested-name, transition), where
transition is a two-word record **`(0, duration)`**. The helper also supports a null transition when its
third argument is false, but this message route always supplies true. The helper is a no-op for a null
radio. Successful recognition returns true regardless of radio call results (`0x004EF010`–`0x004EF0BA`,
`0x004EE270`–`0x004EE2F6`). The station strings are referenced by `0x00B09850`/`0x00B0984C`. *[compat]*

Other main routes use the message slots directly, without payload QI:

| Type | Observable operations |
|---|---|
| `0x2A611B51` | Radio slot 31(), if radio exists. |
| `0xAA611B6E` | Message slot 10 returns its data1 word interpreted as an interface pointer. If non-null, call radio slot 7(object) when radio exists, then object slot 2 once even if radio was absent. No message release and no AddRef/QI in this route. |
| `0xAA650861` | Read data1 (slot 9); radio slot 30(data1 != 0) if available. |
| `0x0A611B34` | Name is `kmay` for data1 0, `krgn` for 1, null otherwise. Read data2 via slot 15. Read data3 through slot 19 only if present (slot 18), otherwise use 0. If radio slot 11(name) returns an object, call its slot 14(data2), then slot 12(2 if data3 == 0, otherwise 1). No call for null radio/object. |
| `0xAA650860` | With radio present, if data2 present call radio slot 16(data2). If data1 present, desired enabled is data1 == 1; compare radio slot 24(). Only on a change call slot 23(desired), then, when enabling, slot 8(0). |
| `0x4348B111` | Only when application audio-preference word at preference-object `+0x10` is nonzero, QI payload `0x65297976`; if successful and radio exists, slot 23(data1 != 0). Release acquired payload. |
| `0x4A7FB7E9` | Read message data1; `RZ::SS` slot 7(`0x0BAAB7A4`, data1, 0, 0, 0). This is a sound-service request, not direct radio selection. |

The standard message slot 10 is a raw load of `+0x10` (`0x0099BE66`–`0x0099BE69`), **not an acquiring
getter**: it neither AddRefs nor QIs anything. Thus `0xAA611B6E` consumes a reference supplied in data1;
do not release the message itself or add an invented acquisition. Slot 15 similarly loads data2 at
`+0x14` (`0x009D4714`). Slot 32 loads borrowed void1 at `+0x24` (`0x009D858F`).
Evidence for dispatch: `0x004EF891`–`0x004EF903`, `0x004EFA3A`–`0x004EFA8F`,
`0x004EFAA3`–`0x004EFAFE`, `0x004EFB21`–`0x004EFC5B`. *[compat]*

#### 7.2.2 Dispatch-manager messages (AU13)

The sub-handler requires a non-null message, non-null QI `0x65297976` payload, member sound system and
data1 presence (payload slot 8). Missing any returns false, releasing acquired payload. Data1 is the
dispatch category; void1 (slot 32) is the sound instance key for the category routes:

| Type | Accepted categories | Sound ID |
|---|---|---|
| `0x894671F6` | 0, 1, 5, 6 | `0x09C66951` |
| `0xE94671EB` | 0, 1, 5, 6 | `0x29D99E9E` |

For an accepted category and nonzero key, request a sound-event payload interface IID `0x5B523BA4`
through sound-system slot 13: event `0xEA451903`, output interface, and four words
**`(sound ID, void1 key, 0, 0)`**. Pass the returned interface to sound-system slot 12, then release it.
The output is assumed valid; no allocation/QI failure branch is supplied. Zero key and other categories
are successful no-ops. Evidence: `0x004EE356`–`0x004EE3AE`, `0x004EE4B8`–`0x004EE51E`.

`0xC9F1011A` and `0x49F10111` instead retain the void1 object and QI it for `0xC9B6D7D6`. Missing void1 or
failed QI returns false with balanced references. Successful QI is merely a capability check: release it
and the retained object, emit **no sound**, return true. No sound IDs should be guessed for those two
messages (`0x004EE3B8`–`0x004EE4B6`). All paths release the outer acquired payload. *[compat]*

#### 7.2.3 Modal notifications (AU13)

The modal target receives a raw notice record: type at byte 0, window pointer at byte 8. It always returns
true (`0x004EFE50`), and notices other than `0x62BA7B42`/`0x62BA7B43` do nothing. Begin notices retain
the window and push it to the front of the handler's modal stack; nested begins are allowed. End notices
pop/release the front window, **ignoring the notice's own window field**. A matching begin is a precondition
of end; empty-stack end is unchecked. There is no implicit list clear at city shutdown. *[compat]*

On empty→nonempty and nonempty→empty transitions only, obtain `RZ::GS` slot 14's window, retain it, QI
`0x3AC7A210`, and obtain that interface's slot 3 message server. If both are available, add (slot 5) or
remove (slot 6) the main target `base+8` subscription to `0xFA0C6390`. Release the temporary QI/window
references. A missing window, failed QI or null returned server skips the subscription change **without
preventing the push/pop**; subscription results are ignored. The separately retained global message
service `0x00B43CCC` is released after processing. Evidence: `0x004EFCC0`–`0x004EFE44`.

#### 7.2.4 Mouse outside a modal window (AU13)

Use the front modal window, retained for the call. QI the message for `0x1A0C615F`. Failed QI or a subtype
other than **7, 9, 8** returns true without sound. For these subtypes obtain the window rectangle through
slot 47 and the message point through slot 10. Bounds are signed and **inclusive** on all four edges.
Only an outside point calls global sound service `0x00B43CB8` slot 11 with
**`(1, 0xAA62C9D5, 0, 0, 0)`**, temporarily retaining/releasing the service. If that service is null,
return false; otherwise return true, ignoring its result. Release the mouse QI and window reference on
either return. This false result is forwarded by main `DoMessage`. A valid front window is required;
there is no safe empty-list mouse route (`0x004EF5D0`–`0x004EF6EA`). *[compat]*

**Subscriptions.** `Init` (`0x004EFE80`) fetches four services — `0x23CECBA0`/`0x106077D2` (`cIGZSndSys`),
`0x49E50BA1`/`0x29E39DB4`, `0x2073215D`/`0xE0203660`, `0x4FA845B`/`0x652294C7` (`cIGZMessageServer2`) —
registers itself with the sound system (`SndSys->vt[0x3C]` at `0x004EFEC8`; the matching `vt[0x40]`
unregister is at `0x004EE056`), and subscribes:

- target `base+0x04` (modal `DoMessage`) to `0x62BA7B42` and `0x62BA7B43` (the modal begin/end notices),
  through the global `DAT_00B43CC8`;
- target `base+0x08` (main `DoMessage`) to `0x26D31EC1`, `0x26D31EC2`, `0xC53D10AA`, `0x26D31EC4`,
  `0x26D31EC5`, `0x4A7FB7E9`, `0x2A3AD653`, `0x0A611B34`, `0x2A611B51`, `0xAA611B6E`, `0xAA650860`,
  `0xAA650861`, `0x4348B111`.

A **second** set of 11 types is subscribed only while a city is loaded, by `HandleCityInitShutdownMessages`
(`0x004EE970`): `0x26D31EC1` **adds** and `0x26D31EC2` **removes** `0xE94671EB`, `0xE94671ED`, `0x894671F6`,
`0xA9E3B6E3`, **`0x8A385DD8`**, **`0xAA396960`**, `0x49F10111`, `0xC9F1011A`, `0xEA3ADA33`, `0x89F10866`,
`0xA9F10E45` on target `base+0x08`. **So the automata SFX messages are only heard while a city is loaded**
*[compat]*.

`Shutdown` (`0x004EE000`) reverses this: unregister from the sound system, release `+0x18`, stop the `+0x10`
service, unsubscribe the `Init` list from `base+0x08`, unsubscribe `0x62BA7B42`/`0x62BA7B43` from `base+0x04`,
clear `+0x1C`. The city-init set is **not** torn down here — it is removed by the `0x26D31EC2` path.

### 7.3 `HandleAutomataMessages` (`0x004EE560`)

Entry: `+0x18` and message must be non-null, and QI `0x65297976` must return a non-null payload;
otherwise return false without an unowned release. If `+0x14` (`cIGZSndSys`) is null, release the acquired
payload and return false. Every later exit also releases that payload once (`0x004EE560`–`0x004EE5B5`).

For type **`0x8A385DD8` (play)** — `0x004EE707`:

1. `payload->slot9()` is compared against **`0xA9F14C47`**. When it matches, a **rate limiter** on the
   handler runs (timer at `+0x30`): if the timer is running and `GetElapsedTime` is **< `+0x48`**, the play
   is **dropped** and the handler returns 1. Otherwise the timer restarts and `+0x48` is set to
   `1000 + rand()*3000/0x7FFF` — a uniform random **1000…4000 ms** (`0x004EE73C`–`0x004EE75F`) *[compat]*.
2. Obtain borrowed void1 through payload slot 32 and QI it for **`0xA9B40F05`**. Failed QI releases any
   returned interface, releases the payload and returns false **without posting**. A null void1 is
   unchecked, not a seeded-position success case. After successful QI, seed three float words from
   `DAT_00B21A04` (image default zero) and let position-interface slot 7 fill `(x,y,z)`.
3. Two events are posted to the sound system (`+0x14`) through its `vt[0x2C]`:
   - **`0xEA451903`** with four words **`(payload data1, payload data2, 1, 0)`** — start sound;
   - **`0xCA45190F`** with four words **`(payload data2, trunc(x), trunc(y), trunc(z))`** — set position.
   The key is read from payload slot 14 (data2) separately for each event. Coordinates are loaded as
   binary32 and converted by `_ftol2`, truncating toward zero to signed integers; these are integer words,
   not float bit patterns or a vector pointer. For `(1.9,-2.9,3.1)` and key 77 the position words are
   `(77,1,-2,3)`. The four words populate standard event data1–data4 (§3.2). Release the position QI after
   both posts. Evidence: `0x004EE78D`–`0x004EE839`. *[compat]*

For type **`0xAA396960` (stop)** — `0x004EE83E`: post **`0x6A451908`** with
**`(payload data2, 0, 0, 0)`**. Slot 14 is a no-argument getter. The earlier `slot14(1,0)` and
`slot14(0,0,0)` notation misattributed the sound-system call's stacked arguments to that getter. *[compat]*

`0xEA451903`, `0x6A451908` and `0xCA45190F` are therefore **sound-system event ids, not message types**.
They are dispatched by `cSC4AudioSoundManager::DoEvent` (section 8.3). **Corrects:** an earlier reading
called these "command identifiers sent to `field8_0x14`" and guessed that field was the Lua script server; it
is `cIGZSndSys` and they are events. **DIFFERS** on the target; the ids themselves **MATCH**.

#### 7.3.1 Alternate automata requests (AU15)

`0xA9E3B6E3` requires non-null void1 and presence of both data1 and data2 (payload slots 8 and 13).
Failure returns false. Signed data1 chooses the AudioScape operation:

| data1 | Calls on `+0x18` |
|---|---|
| 0 | Slot 14(`0x49DE8A94`, void1), then slot 14(`0xA9DE8B59`, void1). Void1 is fetched for each call; no position QI. |
| > 0 | QI void1 `0x26BB767E`; obtain position through its slot 8; slot 13(`0x49DE8A94`, void1, position, **50.0f**). |
| < 0 | Same position path, sound ID **`0xA9DE8B59`**. |

Failed position QI returns false with QI/payload cleanup. Success releases position QI and returns true;
the posted instance is the void1 pointer, **not data2**. The `50.0f` word is `0x42480000`.
Evidence: `0x004EE5E5`–`0x004EE702`. *[compat]*

`0xEA3ADA33` uses **the original message's** void1 (slot 32) and QIs `0x26BB767E`. On success its slot 8
fills position and the QI is released. On failure execution continues, but position has no initialized
value: there is no specified zero/seed fallback or clean false return. Null void1 is unchecked. Message
data1, if absent, defaults to `0xFFFFFFFF`; data2, if absent, also defaults to `0xFFFFFFFF`. The boolean
selector is data1 != 0, and signed/unsigned bit-identical data2 selects:

| data2 | data1 == 0 | data1 != 0 |
|---|---|---|
| 1 | `0x4A5E7FFC` | `0x0A5E802C` |
| 2 | `0x2A5E7F2C` | `0x0A5E7F51` |
| 3 | `0x2A5E8067` | `0x6A5E8088` |
| any other | `0x69F01280` | `0x69F01280` |

Call `SFX::SoundManager` (`0x004BB9E0`) for its service access side effects, but do not call a method on
its return value. Call **no-argument**, global `GenerateNewInstanceId` once (§8.4); then AudioScape slot 13
with **`(selected sound, generated ID, position, 50.0f)`**. Return true and release the acquired outer
payload. The generator neither consumes nor produces a vector and does not belong to this handler's
state. Evidence: `0x004EE862`–`0x004EE94E`, selector `0x004EDE20`–`0x004EDE5B`. *[compat]*

#### 7.3.2 Construction requests (AU13)

Both main construction routes require member AudioScape, a non-null message, and a non-null QI
`0x65297976` payload; failure returns false. Data1 is the instance key. Data3 mask **`0x2`** selects sound
**`0xEA62C800`** when set, **`0xAA62C82A`** otherwise (`0x004EECC4`–`0x004EECC9`).

For `0x89F10866`, positional mode means data2 is present **and nonzero**. Non-positional mode calls
AudioScape slot 14(sound, data1). Positional mode QIs borrowed void1 for `0xA821EF94`; when a non-null
interface is returned, obtain its slot 48 object, let that object's slot 8 fill position, and call
AudioScape slot 13(sound, data1, position, 0.0f). Release that QI. A null QI result emits nothing but still
returns true. Unlike the alternate route above, the QI boolean itself is not tested here.

For `0xA9F10E45`, a null void1 returns false. Otherwise use the same `0xA821EF94` → slot 48 → slot 8
position path, but call **AudioScape slot 15**(sound, data1, position, 0.0f). Null QI result is a successful
no-op. Release QI and payload. Neither route checks a null intermediate slot-48 object. These calls do
not use the automata rate limiter or generate an instance ID. Evidence: `0x004EEC10`–`0x004EED73`,
`0x004EEE23`–`0x004EEEE9`, `0x004EEFF2`. *[compat]*

### 7.4 `cSC4AudioEventHandlerDisasterFire`

CLSID `0x69E500BC`, factory `0x004EB100`, object size **`0x34`**. Registered alongside the handler under
service IID `0xBB523BB2` by `cSC4AudioCOMDirector::PostAppInit` (`0x004EBF40`). **Correction (AU16):**
`HandleTorchControlMessages` (`0x004EF170`), `_StartTorchSound` (`0x004EF0C0`) and `_StopTorchSound`
(`0x004EF100`) belong to **`cSC4AudioEventHandler`**, whose `DoEvent` calls them with the outer base
and whose sound-system member they read at `+0x14`. They do not belong to DisasterFire.

#### 7.4.1 Fire-handler owner contract (AU44)

Use the outer message-target base B for all offsets here; Init/Shutdown receive B+4.
Factory `0x004EB100` allocates 52 bytes, constructs at `0x004F0510`, and returns B+4;
allocation failure returns null. Construction initializes outer refcount B+0x1C and embedded
filter count B+0x10 to zero, initialized byte B+0x18 false, scape B+0x20 null, and four
words B+0x24 through B+0x30 zero. There is no initial scape registration or occupant scan.

| View / table | Slots and behavior |
|---|---|
| B / `0x00A941E8` | 0 QI `0x004F0270`; 1 AddRef `0x004F0240`; 2 Release `0x004F0250`; 3 DoMessage `0x004F05D0`. |
| B+4 / `0x00A941D4` | 0–2 adjust to B and share QI/refcount; 3 Init `0x004F0740`; 4 Shutdown `0x004F03A0`. |
| Embedded filter B+8 / `0x00A941B8` | 0–2 adjust to B for QI/refcount; 3 fire predicate `0x004F0440`; 4/5 return true without inspecting their one argument (`0x004FC900`); 6 deleting destructor adjusts to B (`0x004F0330`). |
| Filter persistence B+0x0C / `0x00A941A0` | 0–2 adjust to B; 3/4 inherited Write/Read `0x005E97C0`/`0x005E97E0`; 5 inherited class ID getter `0x005E95B0`, returns `0xA9C74DC9`. |
| Service B+0x14 / `0x00A94190` | 0–2 adjust to B; 3 returns false without work (`0x005DEB40`). |

QI accepts `1` and lifecycle IID `0x89E51472` as B+4, message-target IID `0x452294AA`
as B, and service IID `0xBB523BB2` as B+0x14. Each success increments the same outer
refcount. Other IDs return false without clearing the output. In particular, filter views are
embedded addresses used directly, not additional accepted QI IDs. AddRef increments a 32-bit
word; Release tests the old count as signed: >1 decrements, otherwise invokes the deleting
destructor through embedded-filter slot 6 and returns zero. Destruction releases a non-null
scape and destroys the embedded filter; it does not call Shutdown or remove subscriptions/groups.

The inherited persistence view writes a 16-bit version 3 through stream slot 7 and returns
the inverse of stream slot 18's error byte. Read obtains a 16-bit version through stream
slot 7, rejects versions other than 3, then returns the inverse error byte. Neither serializes
the handler's state. Stream pointers are unchecked.

Init is idempotent and returns true when already initialized. Otherwise obtain the message
server `(0x04FA845B,0x652294C7)`; missing server returns false with no subscriptions.
Subscribe B to occupant insertion `0x99EF1142`, removal `0x99EF1143`, then city init
`0x26D31EC1`, ignoring subscription results. Obtain scape `(0x49E50BA1,0x29E39DB4)`
and retain it at B+0x20 with normal AddRef-before-old-Release assignment. Missing scape returns
false **without removing the subscriptions**. With scape present, call slot 5
`("Fires",0x49E4CE85,0,100.0f)`. A false return also leaves subscriptions and retained scape
in place, with initialized still false. A successful registration is followed by these three
slot 16 calls, whose results are ignored (`0x004F082E`–`0x004F08CA`):

| Group | Sound | Remaining arguments in order |
|---|---|---|
| `0x49E4CE85` | `0x29DC19EF` | 1, 0, 1, 9 |
| `0x49E4CE85` | `0x09DC19A2` | 1, 0, 10, 49 |
| `0x49E4CE85` | `0x09DC1982` | 1, 0, 50, `0x7FFFFFFF` |

Then set initialized true, release temporary services and return true. There is no scan during
Init itself and no rollback after partial failure. A retry can subscribe again and encounter an
already registered group. Shutdown when uninitialized is a true-return no-op, even after failed
Init. When initialized, reacquire message server and, if available, unsubscribe city init,
insertion, removal in that order. Regardless of server availability, clear/release retained scape,
clear initialized and return true. It does **not** call a scape group-removal method, clear existing
fire instances or subscribe/unsubscribe city shutdown (`0x004F03A0`–`0x004F0436`). *[quirk]*

DoMessage always returns true, including null input, QI failure, missing occupant, non-fire
occupant and unrelated types (`0x004F0734`). For non-null input QI `0x65297976` is required
before dispatch; a false QI skips dispatch and releases any non-null returned view. On success
get payload void1 (slot 32) and temporarily AddRef it if non-null, **even for unrelated types**.
Read type from the original message. Dispatch is:

| Type | Effect |
|---|---|
| `0x99EF1142`, `0x99EF1143` | Non-null void1 must QI `0x26BB767E`. Test that position interface through embedded filter slot 3: QI `0x69D25C60` must succeed (the fire predicate); release its returned view. If accepted, get position through position slot 8 even on removal. Insertion calls scape slot 13(`0x49E4CE85`, position-interface pointer, &position, 0.0f); removal calls slot 14(`0x49E4CE85`, position-interface pointer). Release position QI afterward. |
| `0x26D31EC1` | Call global renderer slot 44 (`+0xB0`) with `(callback 0x004F04D0, B, 0, 0, B+8)`, supplying the embedded fire filter. No renderer null gate. |
| other | No owner action. |

Finally release temporary void1 and payload. No initialized/scape gate is checked before scape
calls. The insertion/removal key is the acquired position interface, not the payload's raw void1.
The fire predicate accepts solely on successful fire IID QI, releasing any non-null QI output
even on failure; it does not inspect a burning flag (`0x004F0440`).

The enumeration callback (`0x004F04D0`) returns false if occupant or B is null. Otherwise
AddRef B, call `IterateFireOccupantCallback`, Release B and return true. That member helper
ignores null occupant, obtains position via occupant slot 8 and calls scape slot 13 with
`(0x49E4CE85, occupant pointer, &position, 0.0f)`. It does not repeat the fire filter or acquire
an occupant reference; renderer enumeration supplies filtering. There is no bulk removal callback.
The position scratch vector is initially zero (global `0x00B21A1C`); the getter result is ignored.

#### AU44 conformance vectors

| ID | Input/action | Expected |
|---|---|---|
| AU44-01 | Construct; QI 1, message-target, lifecycle, service and an unrelated IID | Zero counts/flags/dependencies initially; successful views B+4, B, B+4, B+0x14 increment outer count; failure preserves output. Factory returns B+4. |
| AU44-02 | Init with server absent, then with server present/scape absent | First false with no subscriptions; second false after three subscriptions. Shutdown afterward is a no-op because initialized remains false. |
| AU44-03 | Group registration returns false | Retained scape and subscriptions remain; no slot 16 calls; false result and initialized false. |
| AU44-04 | Successful Init, repeated Init, Shutdown | One group registration, three literal slot 16 calls, no initial scan; repeated Init adds nothing. Shutdown unsubscribes city/insertion/removal, releases scape, never removes group. |
| AU44-05 | Insert then remove fire with distinct raw occupant O and position view P | Fire QI checked both times; position fetched both times; slot 13 uses key P and radius 0, slot 14 removes key P. Both return true regardless of scape result. |
| AU44-06 | Null message, failed payload QI, null void1, rejected fire predicate | Always true; no scape call. Acquired views released in each applicable case. |
| AU44-07 | City init with valid payload; renderer visits occupant O | Enumerate with filter B+8; callback brackets owner with AddRef/Release; slot 13 uses O directly. Null callback occupant/owner returns false. |
| AU44-08 | Destroy initialized handler without Shutdown | Release scape and embedded filter only; no unsubscribe or scape group-removal call. |

### 7.5 Sound-system callbacks and torch state (AU16)

`DoEvent` (`0x004EF490`) returns **true immediately** if member sound system is null. With it present,
obtain a temporary service `0x1AF85BFB` / IID `0x3AF85BB7` and QI the input event for `0x65297976`;
release both acquired references at exit. QI failure does not reject or change dispatch; the QI payload
is not used. Dispatch reads the original event's slot 4 type. Null event is unchecked. After any
dispatched/ignored event the return is **false**, even if sound requests succeeded (`0x004EF58B`).

| Event type | Effect |
|---|---|
| `0x2A106113`, `0x8A10611A`, `0x0A106122`, `0x4A106126`, `0x8A10611D`, `0xAA96FE8F` | Torch table below. |
| `0xAA36D9B6` | Sound-system slot 32(callback `0x004EF120`, 0, 0). For each object visited, callback QIs `0xFD584467`; on success read interface slot 4(`0x20`,0), then slot 3(`0x20`,read value); release QI if non-null; callback returns true. |
| `0x0BAAB7A4` | Read event slot 6's payload, then payload slot 9 value. Obtain an interface through `RZ::SS` slot 43(out); call that interface's slot 11(`0x7F`, value), then release it. Payload/output validity is assumed. |
| other | No effects apart from temporary acquisition/release. |

Torch processing requires member AudioScape and non-null `RZ::SS`; otherwise it returns false without
changing torch state. A null event also returns false, balancing the temporary sound-service reference.
All normal torch calls return false as well, irrespective of sound results. Their event payload is obtained
through event slot 6, not the temporary QI above; it supplies signed x through slot 9, signed y through
slot 14, and a wrapping `u32` timestamp through slot 19. Valid payload and member sound system are
preconditions on sound-producing branches. *[compat]*

Torch state is **process-global**, initially zero: mode (`0x00B219F0`), movement-volume enabled
(`0x00B219F1`), previous smoothed speed (`f32`, `0x00B219F4`), previous y/x (signed words at
`0x00B219F8`/`0x00B219FC`), and previous timestamp (`0x00B21A00`). It is not reset by handler Init,
Shutdown or destruction. Every valid torch event, including an unrecognized direct call, updates the
speed/history after its type-specific action.

Elapsed time is wrapping subtraction of timestamps, interpreted unsigned (the instruction sequence
converts a negative signed difference by adding **4294967296.0f**). For zero elapsed time speed is zero;
otherwise speed is Euclidean distance between the two signed integer points divided by
`elapsed * binary32(0.01)`. The x difference is stored to binary32 before squaring; previous y is first
converted/stored to binary32, and its subtraction/square stays in x87 precision. Add the squared terms,
take square root, divide, then store speed to binary32. Cap at **60.0f**. Smoothed speed is
`(previous smoothed speed + capped speed) * 0.5`, stored to binary32 for the next event. For volume V,
continue the same x87 result (before rounding that history store), multiply first by **424.0f**, then
by binary32 **`0x3C888889`** (approximately 1/60), then truncate toward zero. Do not replace this with
a single rounded `424/60` factor. Evidence: `0x004EF1E6`–`0x004EF2A8`; constants at `0x00A80AA8`,
`0x00A823A4`, `0x00A84D2C`, `0x00A94124`–`0x00A9412C`. *[compat]*

Define Start(sound,key,volume) as two sound-system slot-11 posts, in order:
`0xEA451903` with **(sound,key,0,0)**, then `0x4A451915` with **(key,volume,0,0)**.
The native helper has five arguments (sound,key,unused,volume,unused); callers' 0/1 and `0x200` arguments
are ignored. Stop(key) posts `0x6A451908` with **(key,0,0,0)**. Neither helper tests sound-system presence,
reports post failure or retains keys. Evidence: `0x004EF0C0`–`0x004EF11A`.

| Torch event | Ordered effects, before updating history |
|---|---|
| `0x2A106113` | Start(`0x6A10AC51`, `0x601`, 1024). |
| `0x8A10611A` | Stop `0x601`, `0x602`, `0x603`; **not `0x600`**. Does not reset the two mode flags. |
| `0x0A106122` | Disable movement-volume updates. |
| `0x8A10611D` | Enable movement-volume updates. |
| `0x4A106126` | If movement-volume enabled, post `0x4A451915` for `0x602` then `0x603`, both volume **V+600**, trailing words zero; otherwise no posts. |
| `0xAA96FE8F` with payload data1 != 0 | If mode false: set it true; Start(`0x6A10ABCF`,`0x600`,1024), Start(`0x0A10AD41`,`0x602`,V+600), Start(`0x2AA73C50`,`0x603`,1024−V), then Stop `0x601`. If mode already true, no posts. |
| `0xAA96FE8F` with payload data1 == 0 | If mode true: clear it, Stop `0x602`, Stop `0x603`, Start(`0x6A10AC51`,`0x601`,1024). If already false, no posts. |

Evidence: `0x004EF2B3`–`0x004EF478`. The mode test reads the same data1 used as x for the speed calculation.

### 7.6 Destruction (AU16)

The deleting destructor **does not call Shutdown**, unsubscribe, stop torch sounds, clear global torch
history or unregister the event target. These remain explicit lifetime responsibilities. It destroys
the timer and frees the modal collection's nodes and sentinel, releases any remaining AudioScape member,
then releases the member sound-system and radio references. The raw window pointers in leftover modal
nodes are **not released** by this list destructor; balanced begin/end notices are required to avoid
leaking their retained references. Shutdown itself does not empty the list either. Finally free outer
storage only when deleting-destructor flag bit 0 is set; native return is the original object pointer.
Evidence: `0x004EFC60`–`0x004EFCBF`, node freeing `0x005650A0`–`0x005650DD`. *[compat]*

---

## 8. The SC4 sound manager

`cSC4AudioSoundManager`, CLSID **`0x4A47DC0D`**, factory `0x004EB080`, **object size `0x80`**
(`operator new(0x80)` then ctor `0x004F9830`). It owns the running sound instances.

**The manager never sees `0x8A385DD8` or `0xAA396960`.** Its `DoMessage` (`0x004F9430`) has neither in its
switch and neither in its subscribe list, and a text-section scan finds those immediates only in the
automaton's posters and the event handler. The manager is reached **indirectly**, through sound-system events
(section 8.3) *[compat]*.

### 8.1 Object layout

| Off | Type | Meaning |
|---|---|---|
| `+0x00` | vtable | `0x00A94428` |
| `+0x04` | vtable | `0x00A94418` |
| `+0x08` | vtable | `0x00A94408` |
| `+0x0C` | vtable | `0x00A943F8` — the `cIGZMessageTarget` given to the message server |
| `+0x10` | `u32` | **refcount** (AddRef `0x004F7BA0`) |
| `+0x14` | `u32` | master volume, default `0x400`; `_HandleSetMasterVolume` `0x004FA550` |
| `+0x18` | `u32` | cumulative elapsed time (`Update` adds dt) |
| `+0x1C` | `u32` | update counter |
| `+0x2C` | `u8` | flag |
| `+0x30` | `u32` | focus-restore countdown, set to 50 |
| `+0x34`/`+0x38`/`+0x3C` | hashtable | **instance id to `tSound*`**; buckets begin/end at `+0x38`/`+0x3C` |
| `+0x48` | list node `*` | `std::list` of `tSound*`, sentinel (12-byte node) |
| `+0x4C` | `u32` | list size |
| `+0x54` | `u32` | one-shot counter, compared against `+0x68` |
| `+0x5C` | `cIGZSndSys*` | `GetService(0x23CECBA0, 0x106077D2)` |
| `+0x60` | `cIGZSndMgr*` | `GetService(0x1AF85BFB, 0x3AF85BB7)` |
| `+0x64` | `u32` | tuning `0xAABAA0D3`, default **3** |
| `+0x68` | `u32` | tuning `0x0A946A57`, default **10** |
| `+0x6C` | `u32` | tuning `0x4A946A5D`, default **30** |
| `+0x70` | `cSC4ResponseCurve` | tuning `0x6A946A50` (section 10.8) |

Hashtable node is `{next, u32 key (instance id), tSound*}`; list node is `{next, prev, tSound*}`.

### 8.2 Vtables

`0x00A94428` (primary, `this` = base): 0 `QueryInterface` `0x004F7B70`, 1 `AddRef` `0x004F7BA0`, 2 `Release`
`0x004F8DA0`, 3 `DoEvent` `0x004FA590`.

`0x00A94418` (at `+0x04`, thunks `sub ecx,4`): 3 **`Update(dt)`** `0x004F9CA0` (`ret 4`; adds dt to `+0x18`,
ticks the countdown at `+0x30`, calls `SndSys->vt[0x80](0x004F7FD0, 0, this)` and per-sound `0x004F9690` over
the `+0x48` list).

`0x00A94408` (at `+0x08`, thunks `sub ecx,8`): 3 hashtable lookup by id, reached through QI `0xFD584467`
(`0x004F98F0`).

`0x00A943F8` (at `+0x0C`, thunks `sub ecx,0xC`): 3 **`DoMessage`** `0x004F9430` (`ret 4`, returns `al`).

`QueryInterface` (`0x004F7B70`) answers **`1` and `0x4A47DC0D`** only — the CLSID doubles as the primary IID
(`CreateObject(0x4A47DC0D, 0x4A47DC0D, ...)` at `0x004EC15D`) — and returns the **object base**. `Release`
(`0x004F8DA0`) decrements while > 1, else destroys and returns 0.

**Corrects:** `docs/abi/vtables.tsv` labels slot 1 of `0x00A94428` as `icf::cSC43DRender::AddRef` and slot 1
of the event handler's `0x00A94164` as `icf::cGZLuaScriptServer::AddRef`. Both are **shared-thunk
mis-attributions**: `0x004F7BA0` is `inc [ecx+0x10]` (this class's own refcount) and `0x008AE800` is
`inc [ecx+0x20]`. **DIFFERS.** ABI rows are in `build-runs/pending-abi/re-specs20-rows.tsv`.

### 8.3 Message and event contract

`DoMessage` (`0x004F9430`) handles exactly these, and subscribes to all but one in `Init` (`0x004FA2F0`):

| Type | Name | Action |
|---|---|---|
| `0x28E9397B` | `kSC4MessagePostAppInit` | return 1, no-op |
| `0x28E9397C` | `kSC4MessagePreAppShutdown` | `StopAllSounds(0)` |
| `0x26D31EC2` | `kSC4MessagePreCityShutdown` | `StopAllSounds(0)` |
| `0x4348B111` | `kMessageTypeAppGainLoseFocus` | `_HandleFocusChangeMessage` `0x004F7F00` |
| `0x4A7FB7E9` | `kSC4MessageSimSpeedChange` | `UpdateAllSounds` `0x004F7E20` |
| `0x8A4A69C7` | `kMessageListenerPosChanged` | `UpdateAllSounds` |
| `0xAA7FB7E0` | `kSC4MessageSimPauseChange` | `UpdateAllSounds` |
| `0xC53D10AA` | exemplar changed | `_ReadTuningValues` `0x004F9250` |

`0xC53D10AA` is **handled but never subscribed**; it must arrive by direct post *[inferred]*.

`DoEvent` (`0x004FA590`) is the sound-system event dispatcher:

| Event | Handler | Then |
|---|---|---|
| `0xEA451903` / `1` | `_HandleStartSound` `0x004FA1F0` | `_StartSound` `0x004F9D70` |
| `0x6A451908` / `2` | `FUN_004F9990` | `_StopSound` `0x004F94C0` |
| `0xCA45190F` | `FUN_004FA4C0` | `tSound::SetPosition` |
| `0x4A8C52C8` | `_UpdateAllSounds` `0x004F7E20` | |
| `0x4A4D35E0` | `_HandleResetSound` `0x004F9000` | |
| `0x4A4FE6F6` | `_HandleEnumerateSounds` `0x004F83E0` | |
| `0x4A451915` | `FUN_004F9D20` | |
| `0x6A95CBA2` | `_HandleFocusChangeEvent` `0x004F99F0` | |
| `0xCAA44CA7` | `FUN_004F9610` | used by `StopAllSounds` `0x004F7E90` |

**`_StartSound` (`0x004F9D70`, `ret 0xC` — three stack args).** If the instance id is already live it is
stopped first (`_StopSound(this, id, 1)`); if the caller passed id 0 a fresh one is taken from
`GenerateNewInstanceId`; the sample is resolved through `cIGZSndMgr` (`+0x60`) slot `0x48` with
`kGZIID_cIGZSndDataSourceHolder` `0x206077D2`; a `tSound` is allocated with `+0x08` = instance id, `+0x04` =
sound id, `+0x34 = signedProperty38 * 0.01f * 100.0f`, `+0x38 = x 4`; start-parameter bit 0 sets
instance flag `0x20` and does not supply either radius. It is stored in the `+0x38` hashtable and
the `+0x48` list.

**`_StopSound` (`0x004F94C0`, `ret 8`).** On an **unknown instance id it returns false and does nothing**
(`xor al,al; ret 8` at `0x004F94FF`) — no side effects. The caller `FUN_004F9990` ignores that and still
returns 1. Otherwise it unregisters, turns off completion notification, and takes the soft (`+0x20`) or hard
(`+0x24`) path on the event's second argument; the effect component always sends 0 (soft) *[compat]*.

### 8.3.1 Remaining manager owner contracts (AU46, AU49)

**Defaults and lifetime.** Constructor `0x004F9830` starts refcount, elapsed/update counters,
focus countdown, flag byte `+0x2C`, instance/capacity counters and service holders at zero;
master volume is 1024 and tuning defaults are 3, 10, 30. It constructs the hash table with
initial size argument 100, an empty instance list and the response curve. Init (`0x004FA2F0`)
first acquires SndSys, retains it in `+0x5C`, then acquires SndMgr and retains it in `+0x60`.
Either missing service returns false; missing SndMgr leaves the newly retained SndSys in place.
There is no initialized latch or rollback. After both acquisitions it reads tuning, calls
SndSys `+0x3C(this)` and `+0x78(this+4,20)`, and obtains the message server. If available it
subscribes `this+0x0C` to the seven messages in §8.3, in table order except sim-speed last
(post-app, pre-app, pre-city, listener, focus, pause, speed). Missing message server skips
subscriptions. Registration results do not gate success. It reads preferences, sets master
volume to the configured value if enabled, otherwise zero, updates all instances and returns true.
Temporary service references are balanced independently of retained holders.

Shutdown (`0x004F82E0`) sends StopAllSounds(true), calls SndSys `+0xBC`, then unsubscribes
speed, pause, focus, listener, pre-city, pre-app, post-app if a message server exists. It calls
SndSys `+0x7C(this+4)` and `+0x40(this)`, clears/releases SndSys then SndMgr, and returns true.
Null holders skip their calls. Destructor `0x004F8820` destroys containers, curve and reference
holders; it does not substitute for Shutdown. These calls do not make Init transactional.

**Payload routes.** Event `+0x18` supplies the payload. Reset (`0x004F9000`) and volume
(`0x004F9D20`) reject a null event or null payload. Each retains the payload across processing,
reads the instance ID at payload `+0x24`, balances its reference, and returns true even when
the ID is absent. Reset's inner operation (`0x004F8DD0`) returns false for a missing ID;
otherwise it temporarily retains the instance, QIs its holder for `0xBB1BE367`, calls that
interface's `+0x30`, then QIs it for `0xFD584467`. If that succeeds it reads property `0x15`
with second argument zero and writes the same value back to property `0x15` (third argument
zero). Failed QI skips the dependent calls but does not change the found-ID success result.
All temporary interfaces and the temporary instance reference are released.

Volume first reads payload `+0x38`, then `+0x24`. The inner setter (`0x004F9AF0`) stores the
raw volume word at instance `+0x18`, treats that word as signed i32, multiplies it by the f32
gain at `+0x1C`, truncates toward zero and writes buffer property `0x15(value,0)`. It updates
position when the instance has children or a master, sets instance flag bit 0, and returns
true; a missing ID returns false without these effects. No clamp is applied
(`0x004F9B37`–`0x004F9B57`).

Enumeration (`0x004F83E0`) requires valid event/payload pointers: unlike reset, it does not
null-check them. Payload `+0x80` provides an object QI'd for `0xEA4FC425`. Failed QI returns
false, balancing acquired references. Success walks the instance list in order and calls
callback `+0x0C` with each **instance pointer**, stopping at its first false result. Both
exhaustion and early callback termination return true. The callback object and QI result are
released; a callback is not promised a snapshot independent of list mutation.

**Focus and stop-all.** Focus message `0x4348B111` (`0x004F7F00`) requires SndSys and a
successful message QI for `0x65297976`. It converts QI-view `+0x24` to a boolean and creates
event `0x6A95CBA2`, payload IID `0x5B523BA4`, data `(boolean,0,0,0)` through SndSys `+0x34`.
Successful creation dispatches through `+0x20` and returns true; failure returns false.
The created event and message view are released on both paths as applicable.
The focus-event helper (`0x004F99F0`) rejects missing event, SndSys or payload; otherwise it
retains the payload, reads `+0x24`, and walks every instance. Gain calls Resume; loss increments
its pause count and, only on transition to one for a non-child, disables completion and calls
holder `+0x24`. Loss sets manager volume to zero. Gain restores the configured volume if
enabled, otherwise zero; gain also sets the countdown to 50. It returns true after releasing
the payload. **DoEvent discards this helper's boolean and returns false** for this route.

Public StopAllSounds creates event `0xCAA44CA7` with the same payload IID and four data words
`(argument,0,0,0)`, dispatches/releases it, and returns creation success; missing SndSys returns
false. Its event route directly reads payload `+0x24` without a null gate and normalizes to
boolean. The helper `0x004F9610` repeatedly stops the current first list entry until empty,
using that boolean. DoEvent returns false after these effects. UpdateAllSounds similarly
posts `0x4A8C52C8`; its event helper `0x004FA180` rewrites effective volume, updates linked
positions, applies distance and simulation-pause policies to every instance, and returns true.

**Tick callback.** Update runs on the `this+4` interface. It adds dt modulo 2^32 to manager
elapsed time. While countdown is nonzero it sets it to signed-min(countdown,50) minus one
and enumerates SndSys with `(0x004F7FD0,0,manager)`. That callback always returns true; a
non-null sound successfully QI'd for `0x206077D2` receives `+0x50()` followed by
`+0x54(0)` then `+0x54(saved value)`, and the QI reference is released. Update brackets its
list walk with an increment/decrement of `+0x1C` (`0x004F9CAA`–`0x004F9D0F`).
Per-sound callback `0x004F9690` converts dt as unsigned u32 to f32 and advances the ramp
at instance `+0x40`. Equal current/target means no work; otherwise it moves toward target
by dt times signed rate, clamped to the remaining signed difference, and stores f32 current.
It truncates current to i32 into `+0x18`, multiplies that integer by f32 gain `+0x1C`, truncates
again, and writes property `0x15(value,0)`. Linked instances then update position. This is
two truncations, with no integer-volume clamp (`0x004F7C50`–`0x004F7CAA`,
`0x004F9694`–`0x004F96DF`).

**Start, completion, stop and position.** The word `tSound+0x14` is a retained manager
reference, initially null in constructor `0x004F8790`, assigned before completion registration;
it is **not** a play argument. Holder play receives `(instance ID, manager volume,512,0)`.
Other constructor defaults are zero IDs, null buffers/master, empty child list, zero flags,
pause count/ramp/radii, gain 1.0 and the position sentinel in all six coordinate fields.
Radius computation uses signed property `0x38`, x87 multiplication by f32 0.01 then f32 100;
the inner radius is rounded to f32 by FST, but the still-extended product is multiplied by
f32 4 before storing the outer radius (`0x004F9FCA`–`0x004F9FEB`). It is not computed
from the start parameter's low byte, nor from a reload of the rounded inner radius.

Failed sound resolution or failed buffer QI returns false and releases temporary outputs;
there is no instance insertion. The pool allocator and instance constructor are required to
produce storage: `0x004F9F3E` dereferences the allocation and `0x004F9F61` dereferences the
constructor result without a clean null-return path. A failed holder play returns false
before map/list insertion, dropping local instance and interface references; it does not
restore any old instance stopped for an ID collision. Completion registration's result does
not gate play. EnableCompletionNotification (`0x004F8130`) QIs the holder for `0xE9D00A9A`
and calls its `+0x0C` with manager `+8` when enabling, null when disabling, then releases the
QI result. Failed QI returns false without registration. Callback `0x004F98F0` QIs its input
for `0xFD584467`, reads property `0x34` as instance ID, finds/temporarily retains the instance,
stops it with true, then balances both references. Unknown IDs have no stop effects.

Stop removes the manager's list/hash entry before foreign stop calls. With no children it
disables completion, checks holder `+0x1C()` and calls `+0x24()` for a true hard-stop argument;
for false it calls `+0x20()` only if property `0x33` is nonzero. Thus false stop of a one-shot
does not issue a foreign stop, although it removes the manager record. One-shot counter is
decremented when property `0x33` is zero; otherwise the independent-loop counter is decremented
only for a non-child. With children, the first child becomes master (`0x004F9150`) and inherits
the remaining children and buffer references; removing a child unlinks it from its master
(`0x004F9100`). Instance buffers are cleared/released, then temporary/container references
are dropped (`0x004F94C0`–`0x004F960F`).

SetPosition (`0x004FA260`, despite its exported tSound name, takes the manager base) returns
false for an unknown ID. A found instance is temporarily retained; raw f32 x/y/z are stored
at `+0x20/+0x24/+0x28`, its position and then its master's position (if any) are updated,
distance pause policy runs, flag bit 1 is set and the temporary reference released; return true.
UpdatePosition selects the raw coordinates for a childless instance, averaged coordinates
otherwise, after the mode-specific linked volume/pan update. Buffer `+0x0C` receives property
`0x20(trunc(x),1)`, `0x21(trunc(y),1)`, `0x22(trunc(z),0)` in that order
(`0x004F90B2`–`0x004F90F3`). Foreign property results do not gate the found-ID return value.

**Conformance cases.** Missing SndMgr after successful SndSys acquisition leaves SndSys retained;
Shutdown releases it. Missing payload rejects reset/volume, but an unknown ID with a payload
returns true. Enumeration with three instances and false on callback two visits exactly two
and returns true. Volume -3 with gain 0.5 writes -1, sets bit 0 and retains raw -3. Ramp current
3.9, target 5, rate 0.01, dt 1 stores about 3.91 then uses integer 3 before gain. Focus gain
arms 50; 50 subsequent updates run the restore callback 50 times. Focus-event DoEvent returns
false despite applying gain. Start flags 255 and property 0x38=7 use the property-based radii
and only flag bit 0's meaning. A failed play leaves no new record. Register then unregister
completion with `(manager+8)` then null. Soft-stop one-shot removes the entry with no +0x20
or +0x24 call. SetPosition(unknown) is false; a childless found ID at (-1.8,2.9,3.1) produces
properties (-1,2,3) with flags (1,1,0) and returns true even if those property calls fail.

### 8.4 `GenerateNewInstanceId` (`0x004F7BB0`) — answered

```
mov  ecx, dword ptr [0x00B098C0]
test ecx, ecx
jne  have
mov  ecx, 1
have:
mov  eax, ecx          ; return value = pre-increment
inc  ecx
mov  dword ptr [0x00B098C0], ecx
ret
```

- A single **process-global** counter at **`0x00B098C0`** — **not** per-manager (no `this` is read).
- Returns the **pre-increment** value, so ids are issued 1, 2, 3, ...
- **Skips 0**: a zero counter is forced to 1 first, so 0 is never issued, including after a 32-bit wrap.
- No saturation. `ret` with no stack args.
- The same sequence is **inlined** in `_StartSound` (`0x004F9F0F`-`0x004F9F22`), taken only when the caller
  passed id 0. Those four sites are the only references to `0x00B098C0` in the image.

**Corrects:** the open item **AU2** is now answered. A replacement must keep a single global counter, must
never return 0, and must use the pre-increment value *[compat]*.

### 8.5 Save behaviour

**Nothing.** All four vtables have only four slots and none is `Read`, `Write` or `GetGZCLSID`.
`QueryInterface` answers only `1` and `0x4A47DC0D`, so there is no `cIGZSerializable` interface. A scan of
`.text` for the `mov eax, imm32; ret` form finds **no** `GetGZCLSID` thunk returning `0x4A47DC0D`; all five
occurrences of that constant are the factory registration, the factory table, the QI compare and the ID
table. Sound instances exist only in the `+0x38` hashtable and the `+0x48` list and are destroyed by
`_StopSound` / `StopAllSounds`. The only persistence-adjacent code is the **read-only** tuning of section
10.8 *[compat]*.

### 8.6 `cGZSndMgr` — the framework sound manager

CLSID `0x7AF85B98`, service IID `0x287FB697` (created in `cGZSoundCOMDirector::OnStart` `0x0096655A`). Its
decompilation **fails** in the export (`Low-level Error: Field field5_0x18 does not fit in structure
cGZSndMgr`) for every method except `QueryInterface`; the names and entry addresses are still exact.

| Method | Address | Method | Address |
|---|---|---|---|
| `SetSystemSndVol` | `0x00893D87` | `GetSndDataSourceCreator` | `0x00968FFE` |
| `Create` | `0x009663F8` | `GetSndDataSource` | `0x00969055` |
| `QueryInterface` | `0x00968A9D` | `SystemSndID` | `0x009692FC` |
| `Release` | `0x00968AD1` | `RegisterSndCreator` | `0x00969335` |
| `GetSound` | `0x00968AE7`, `0x009693C0` | `AddSndToAutoPlayList` | `0x00969475` |
| `SndAutoPlay` | `0x00968B1D`, `0x009694A2` | `UnregisterSndDataSource` | `0x0096958D`, `0x009695CB` |
| `SystemSndAutoPlay` | `0x00968B2C`, `0x00969536` | `Shutdown` | `0x0096966C` |
| `SetDefaultSndDataSourcePlayer2DClassId` | `0x00968B67` | `UnregisterSnd` | `0x00969710`, `0x00969752` |
| `GetDefaultSndDataSourcePlayer2DClassId` | `0x00968B76` | `RegisterSndDataSource` | `0x009697B7` |
| `SetDefaultSndDataSourcePlayer3DClassId` | `0x00968B87` | `SetSystemSndID` | `0x0096995C` |
| `GetDefaultSndDataSourcePlayer3DClassId` | `0x00968B96` | `RegisterSnd` | `0x009699BA` |
| `Init` | `0x00968BA7` | `RegisterSndDataSourceCreator` | `0x00969A31` |
| `GetResource` | `0x00968C18` | `~cGZSndMgr` | `0x00969889`, `0x00969AB9` |
| `RemoveSndCreator` | `0x00968D08` | `cGZSndMgr` | `0x00969AF1` |
| `RemoveSndDataSourceCreator` | `0x00968F4F`, `0x00968FB0` | | |

`QueryInterface` (`0x00968A9D`) answers IID **`0x3AF85BB7`** by returning `this+0x1C` and calling `AddRef`;
every other IID falls through to `cRZSystemService::QueryInterface`. `cGZSndMgr` therefore derives from
`cRZSystemService` and exposes `cIGZSndMgr` at `+0x1C` *[compat]*.

`GetSound` at `0x00968AE7` is the one whose bytes were read. `__stdcall`, **`ret 0x14`** (five `u32`
arguments). It calls `arg1->slot5(0)`, passes the result through `FUN_00932F76` (`0x00932F76`), ORs the
result with **`0xFF000000`**, then calls `this->slot18` (`+0x48`) with the transformed value and the
remaining four arguments. The `0xFF000000` OR is a **sound-id tag** *[inferred]*; what would settle it:
whether every sound id the game uses has its top byte clear.

### 8.7 The full play/stop chain

```
cSC4Automaton::HandleEnterState  0x00511450   reads state+0x18 (the SFX id)
  -> cSC4Automaton::StartSFX     0x00509950   GenerateNewInstanceId -> automaton+0x8C
       posts 0x8A385DD8 {data1=SFX id, data2=instance id, void1=occupant}
  -> message server MessagePost  (*DAT_00B43CCC)->slot4(msg, 0)
  -> cSC4AudioEventHandler::DoMessage          0x004EF810
  -> cSC4AudioEventHandler::HandleAutomataMessages  0x004EE560
       rate-limit gate, position QI 0xA9B40F05
  -> cIGZSndSys->vt[0x2C](0xEA451903, ...)     the start-sound event
  -> cSC4AudioSoundManager::DoEvent            0x004FA590
  -> _HandleStartSound                         0x004FA1F0
  -> _StartSound                               0x004F9D70
       resolve the sample via cIGZSndMgr slot 0x48 (IID 0x206077D2), allocate tSound
```

and the stop chain is identical with `0xAA396960` and `0x6A451908` into `_StopSound` (`0x004F94C0`).

---

## 9. The effects system's sound component

The effect component type is **7** ([effects.md](effects.md) section 2). Its description class is
**`cSC4SoundDescription`** (section 10.9); `GetType` is vtable slot 0 = `0x00505070` =
`MOV EAX, 7; RET`, which is what makes it type 7.

### 9.1 The runtime component

Vtable **`0x00AA1118`**, object size **`0x30`** (constructed by `0x005BE370`, allocated by the description
factory `0x005BE6D0`, destroyed by `0x005BE740`).

| Slot | Off | Target | Method | pop |
|---|---|---|---|---|
| 0 | `+0x00` | `0x005BE430` | `Start(mode)` | 4 — **mode is never read** |
| 1 | `+0x04` | `0x005BE4A0` | `Stop(mode)` | 4 — **mode is never read** |
| 2 | `+0x08` | `0x00580B50` | `IsActive` = `mov al,[ecx+0x1C]; ret` | 0 |
| 3 | `+0x0C` | `0x005BE4E0` | `SetTransform(local, parent)` | 8 |
| 4 | `+0x10` | `0x005BE620` | `Update(dt seconds)` | 4 |
| 5 | `+0x14` | `0x005BE2E0` | `SetParameter` | 12 — `xor al,al; ret 0xC`, a no-op |
| 6 | `+0x18` | `0x005BE2E0` | `SetVoidParams` | 12 — the same no-op |
| 7 | `+0x1C` | `0x005BE3E0` | AddRef (`add ecx,4` then `0x00968B60`) | 0 |
| 8 | `+0x20` | `0x005BE3F0` | Release (`add ecx,4` then `0x0090D96B`) | 0 |

| Off | Type | Meaning |
|---|---|---|
| `+0x00` | vtable | `0x00AA1118` |
| `+0x04`/`+0x08` | | `cRZRefCount` sub-object, vtable `0x00AA1104` |
| `+0x0C` | `cSC4SoundDescription*` | the description (AddRef'd; Released in the dtor) |
| `+0x10`, `+0x14`, `+0x18` | `f32` | world position x, y, z |
| `+0x1C` | `u8` | **started** — the `IsActive` byte |
| `+0x20` | `u32` | **instance handle**, set to `this` at `Start` |
| `+0x24` | `f32` | elapsed seconds |
| `+0x28` | `f32` | next position-update time |
| `+0x2C` | `cIGZSndSys*` | cached at `Start`, Released at `Stop` and in the dtor |

**Corrects:** effects.md section 11.2 lists three implementations of the effect-component interface. This is
a **fourth**, with the identical nine-slot shape (slots 5 and 6 share the no-op `0x005BE2E0` with the brush
and game components), and `cSC4CameraEffect` is a **fifth** with a *different* layout (description at `+0x10`,
started byte at `+0x20`). **PARTLY** — the list is incomplete.

### 9.2 Attach, loop, stop

**Attach.** `cSC4EffectsManager::CreateComponentFromDescription` (`0x00591730`) looks the description up in
the type-7 vector — `base = *(manager+0x158) + (7-3)*0x20`, vector at `base+0x14` — and calls the
description's factory (vtable slot 1) to build the component, storing it with `cRZAutoRefCount::operator=`
(`0x0058D8D0`) and setting runtime flag bit 1 (`|= 2`). The effect's per-component tick (`0x005BEA10`) walks
its runtime records and, for each whose `IsActive` is true, calls `Update(dt)`.

**Start (`0x005BE430`).** No-op if `+0x1C != 0`. Caches `*(0x00B43CB8)` (the `cIGZSndSys` service) into
`+0x2C`; zeroes `+0x24` and `+0x28`; sets `+0x20 = this`; then posts
**`0xEA451903` (the description's `+0x10` sound id, `this` as instance, 0, 0)** through the sound system's
slot `0x2C`; finally sets `+0x1C = 1`.

**Update (`0x005BE620`).** Adds dt to `+0x24`. If the description's `+0x14` (location-update period) is > 0
and `+0x24` has passed `+0x28`, it sets `+0x28 = +0x24 + period` and posts
**`0xCA45190F` (`+0x20`, ftol(x), ftol(y), ftol(z))**. If the description's `+0x18` (max length) is > 0 and
`+0x24` has passed it, it calls `Stop(1)`.

**Stop (`0x005BE4A0`).** No-op if `+0x1C == 0`. Sets `+0x1C = 0` **before** the null check on `+0x2C`; posts
**`0x6A451908` (`+0x20`, 0, 0, 0)**; then clears and Releases `+0x2C`. The `mode` argument is unread, so a
hard and a soft stop of a sound component are identical.

**The instance handle is the component's own `this` pointer** — `Start` sets `+0x20 = this` and every later
event carries that value as the key.

### 9.3 The ignore bit

**Where.** The component record (`cSC4EffectDescription::cDescriptionRec`, `0x6C`) at **`+0x10`**, a `u32`
masked `& 3`. The binary reader `operator>>` (`0x005AB690`) does `ReadU8 -> +0x0C` (the type) and
`ReadU32 & 3 -> +0x10`; the text writer `AddAnonDescription` (`0x005A2960`) ORs and ANDs bit 0 there.

**Meaning.** In `CreateComponentFromDescription` (`0x00591730`):
`if (rec+0x10 & 1) runtime+0x40 |= 4; else runtime+0x40 &= ~4;` and `CheckFinished` (`0x005BFD70`) skips any
component whose runtime flag bit 2 is set. So bit 0 means **"this component is not considered when deciding
whether the effect has finished"**. effects.md's claim **MATCHES**.

**The four exceptions** (measured from `SimCity_1.dat`, effect directory `0xEA5118B0/0xEA5118B1/1`, sound
section at offset 633,375 count 305): of 303 type-7 components, 299 have bit 0 set and **4 do not**:

| Effect | Sound id | Period | Length | Why it does not pin the effect |
|---|---|---|---|---|
| `researchsparksfx_slave` | `0x6A92274B` | 0.5 | **50.0** | the only one that counts: `Update` self-stops at t > 50 s |
| `tornado_active` | `0x4A890A7C` | 0.5 | 0 | effect flag bit 1 = "no auto stop", so it never finishes anyway |
| `volcano_active` | `0x4A890A7C` | 0.5 | 0 | same |
| `earthquake_active` | `0x4A890A7C` | 0.5 | 0 | same |

**Corrects:** the "3373 total components" figure that circulated in lane notes is **not in effects.md** and is
not reproducible. effects.md section 7's own `Records` column sums to **3,420**
(1921+406+591+25+64+56+10+303+41+3), which matches the shipped effect directory. The per-type counts and the
299/303 sound split **MATCH**. Also: effects.md section 7's `IsActive` table has no sound row — see the
`**Corrects:**` note there. **A sound with `length == 0` that is *not* ignored pins the effect forever.**

**Quirk.** The text parser derives the bit as `bit0 = (length == 0)` (`cGroupSoundCommand::Parse`
`0x005A2A50` computes it from `+0x18` and passes it to `AddAnonDescription`). That rule **does not hold in
the shipped binary** for three of the four above, all of which have `length == 0` yet the bit clear. Either
those were authored by a path that sets the bit independently of `-length`, or the binary predates the rule
*[unsettled]*.

### 9.4 A distinct field, easy to confuse

The sound **description** has its own 1-bit field at **`+0x0C`** (`Read` `0x005BE790` masks `& 1`). All 305
shipped records have it 0 and nothing in the component's Start/Stop/SetTransform/Update reads it
*[inferred]* unused by the effect path.

### 9.5 The bus

The effect component speaks to **`cIGZSndSys`** (global `0x00B43CB8`, obtained by `SL::OnAppInit1`
`0x0060245B` from `GetService(0x23CECBA0, 0x106077D2)`), **not** to the message server. The same
five-argument event shape appears in `cSC4AutomataController`, `cSC4EmergencyVehicle`,
`cSC4ViewInputControl` and `cSC4AudioEventHandler::_StartTorchSound` / `_StopTorchSound`. **AEMS is not on
this path** (`cSC4AudioAEMSEventSystem::DoEvent` `0x004E14B0` handles only `0x2A5B69C0`, the master volume).

## 10. Sound data

Counts in this section marked **measured** were read out of the shipped packages with `tools/dbpf.py` against
the GOG install (`SimCity_1.dat`, `Sound.dat`); the rest are decompiler-derived and carry their addresses.

### 10.1 `Sound.dat` inventory (measured)

`Sound.dat`: **122,372,241 bytes, 2776 index entries**.

| Type | Count | Compressed | Stored bytes | Role |
|---|---|---|---|---|
| `0x2026960B` | 1680 | 125 | 122,235,432 | samples, `.hit` and `.str` payloads |
| `0x0B8D821A` | 869 | 0 | 72,807 | TKDT track definitions |
| `0x7B1ACFCD` | 214 | 0 | 5,240 | Hit lists |
| `0x9D796DB4` | 12 | 11 | 970 | `.tlo` |
| `0xE86B1EEF` | 1 | 0 | 2,176 | DIR (compression map) |

Payload magics of the 1680 `0x2026960B` records (**measured**): `XA\0\0` ×1512, `vers` ×71, `XAJ\0` ×43,
`XAI\0` ×32, `RIFF` ×21, `HIT!` ×1.

**Corrects:** INVENTORY-M6 §5 says "1,512 of the records surveyed are EA XA ADPCM, a few RIFF WAV; about
120 MB". **PARTLY.** The 1,512 figure is exactly the `XA\0\0` subset; the type holds **1680** records and four
further magics (`vers`, `XAJ\0`, `XAI\0`, `HIT!`) that the survey did not count. The ~120 MB figure
**MATCHES** (122.4 MB). The "12 records of type `0x9D796DB4`" figure **MATCHES**.

### 10.2 `0x2026960B` is context-sensitive

The same type TGI means different things in different DB segments. This is why the type appears both as
"LTEXT" and as "sound effect" in earlier surveys.

| Segment | Directory | Extension | Meaning |
|---|---|---|---|
| HitLists | `%DataDirectory%Sound\HitLists\` | `.str` | LTEXT / strings |
| TLO | `%DataDirectory%Sound\TLO\` | `.hit` | Hit track data |
| every `[SampleDataSegments]` group | — | — | raw sample data |

The binding is **data-driven**, not hard-coded. `cHitIniFile::ProcessDBSegmentDirectoryFilesEntry`
(`0x00936707`) parses each `type:ext` pair of the INI's `[DBSegmentDirectoryFiles]` section, converts the type
with `RZConvertToUint32A`, and registers `(type, extension)` on a directory object obtained by
`QueryInterface(0x856C8CF7)` on the HitMan service, binding the type to the segment group through HitMan
vtable `+0xB8`. **No resource-type constant is hard-coded on this path** *[compat]*.

Related `cHitIniFile` entry points: `InitHitManTypeGroups` `0x009369F6`, `InitTrackDataBlocks` `0x00936D7B`,
`InitDBSegments` `0x00937159`, `InitResourceFactories` `0x009373D6` (which calls
`cIGZPersistResourceManager` vtable `+0x4C` = `RegisterFactory(clsid, type, 0)`),
`ProcessDBSegmentsEntryCreate` `0x009362C9`, `ProcessDBSegmentsEntryInit` `0x0093657D`,
`ProcessTrackDataBlockEntry` `0x00936298`, `ProcessPackedDBSegmentEntry` `0x00936E6D`,
`InitDataSourceCreators` `0x00936B57`, `InitSndCreators` `0x00936BCC`.

### 10.3 The audio configuration record (measured)

The key to the whole audio data model is a text configuration record in `SimCity_1.dat`,
type `0x00000000` / group `0x8A5971C5` / instance `0xEA8A1115` (**measured**, 4012 bytes). `cSC4AudioCOMDirector::PostAppInit` (`0x004EBF40`) loads exactly this pair of keys
(`local_70 = 0x8A5971C5`, `local_6c = 0xEA8A1115`). Its sections:

```ini
[HitManTypeGroups]
0x9dbdbf74=0x7b1acfcd

[DBSegments]
0x9dbdbf74=0xa079ce1b;%DataDirectory%Sound\HitLists\
0xca4d19e3=0xa079ce1b;%DataDirectory%Sound\TLO\

[DBSegmentDirectoryFiles]
0x9dbdbf74=0x7b1acfcd:hls,0x2026960b:str,0x7b1acfcd:hls
0xca4d19e3=0x9d796db4:tlo,0x2026960b:hit

[TrackDataBlocks]
0x5b84113f=Sound\TLO\TracksMain_2026960b_ca4d19e3_ea768228.hit

[ResourceFactories]
; kGZRES_resGZSndPropertySet_Track_OldFormat = HitFactory
0x0b8d821a=0x5b945038

[SndFileDefaultDataSourceClassIds]
1=0x1d07eb4b,XA    2=0x1b6b9806,UTK    3=0xbb7051f5,WAV    4=0x3cec2b47,MP3

[Config]
DefaultSndDataSourcePlayer2DClassId=0xfded7f31 ; cGZSnd
DefaultSndDataSourcePlayer3DClassId=0xfded7f34 ; cGZSnd3d
;Class must implement kGZIID_cIGZSndDataSourceHolder
```

It also lists `[TrackDefSegments]` (`0xAA4D1920`, `0xEA4D192A`, `0xAA4D1930`, `0x2A4D1937`, `0x2A4D193D`,
`0xCA4D1943`, `0xCA4D1948`) and `[SampleDataSegments]` (seven parallel IDs `0x0A4D1926` … `0xAA4D194B`, plus
**35 ambience groups** such as `0xCA88CC85` `Am_Abandoned`, `0x0A5BCDA5` `Am_Construction`, … `0xEA62791B`
`Am_zm3wind`). **Measured:** the seven `0x0B8D821A` groups in `Sound.dat` are exactly `[TrackDefSegments]`,
and the `0x2026960B` groups are exactly `[SampleDataSegments]` ∪ the two directory groups.

The data-source class IDs give the codec set: `0x1D07EB4B` = XA, `0x1B6B9806` = UTK (UTalk),
`0xBB7051F5` = WAV, `0x3CEC2B47` = MP3. All four are registered by `cGZSoundCOMDirector`'s constructor
(`0x00966498`) — §1.2's "unnamed" sound classes are these data-source players *[inferred]*.

### 10.4 Sample records

A `0x2026960B` record in a `[SampleDataSegments]` group is a **0x18-byte header then ADPCM payload**
(**measured**):

| Off | Size | Field | Measured |
|---|---|---|---|
| `0x00` | 4 | magic | `XA\0\0` (1512), `vers` (71), `XAJ\0` (43), `XAI\0` (32), `RIFF` (21), `HIT!` (1) |
| `0x04` | 4 | size-like | ≈3.73× the record size for the XA set *[inferred: decoded PCM byte count]* |
| `0x08` | 2 | | 1 |
| `0x0A` | 2 | | 1 |
| `0x0C` | 4 | | 22050 |
| `0x10` | 4 | | 44100 (88200 on the 4-channel set) |
| `0x14` | 2 | channels | 2 (or 4) |
| `0x16` | 2 | bits | 16 |

XA header histogram (**measured**): `(22050, 44100, 2ch, 16bit)` ×1440, `(22050, 88200, 4ch, 16bit)` ×72.

### 10.5 The resource factory for `0x2026960B`

Registered in `cSC4App::SetupResources` (`0x004572B0`) at **`0x00457EDA`**:

```
00457ed3  MOV ECX,[ESP+0x1c]      ; cIGZPersistResourceManager
00457ed7  MOV EAX,[ECX]
00457ed9  PUSH EBX                ; 0
00457eda  PUSH 0x2026960b         ; resource type
00457edf  PUSH 0x436eb4           ; class id
00457ee4  CALL [EAX+0x4c]         ; RegisterFactory(clsid, type, 0)
```

The factory record lives at **`0x00AD8B28`** (CreateInstance / Write / Read = `FUN_0098419A` `0x0098419A`,
`FUN_009840A7` `0x009840A7`, `FUN_009840F5` `0x009840F5`). `CreateInstance(type, riid, ppv)` returns 0 unless
`type == 0x2026960B`; it then allocates **`0x2C` bytes**, constructs with `FUN_00984146` (`0x00984146`) and
answers `riid` through vtable slot 0.

The resource class is **`cRZStringResource`** (vtable `0x00ADA4F0`, per `re/labels_windows.tsv:790`, paired
with Mac `__ZTV17cRZStringResource`). Its layout:

| Off | Meaning |
|---|---|
| `+0x00` | `cRZStringResource` vptr `0x00ADA4F0` |
| `+0x14` | `cGZPersistResource` sub-object, vtable `0x00ADA564` |

`cRZStringResource::QueryInterface` (`0x0098407D`) answers IID **`0x456B8F1D`** with `this+0x14`; the
destructor `0x009841D0` tears down `cGZPersistResource(this+0x14)` then `cSLString(this)`.

The CLSID literal registered at `0x00457EDF` is **`0x00436EB4`**; it is *not* in `re/class_id_table.tsv`
*[inferred]* it is the GZCOM class id of the anonymous factory whose implementation is the `0x00AD8B28`
record.

### 10.6 Sound instance lookup

`cSC4AudioSoundManager::_StartSound` (`0x004F9D70`) takes `(soundId, instanceId, flags)`:

1. hash-chains on `instanceId` over `field32_0x38`…`field33_0x3C`; a live instance with the same id is stopped
   first (`_StopSound` `0x004F94C0`);
2. resolves the sample through `this+0x60`, calling vtable `+0x48(soundId, 0, 0, 0x206077D2, &buffer)`.
   **`0x206077D2` is `kGZIID_cIGZSndDataSourceHolder`** — corroborated at `0x0080F81F`, where
   `PUSH 0x206077D2; PUSH 0xFDED7F31` (`0xFDED7F31` = `cGZSnd`, the `DefaultSndDataSourcePlayer2DClassId`
   above) are used together as `(clsid, iid)`;
3. `QueryInterface(0xFD584467, …)` on the returned object;
4. stores two `cRZAutoRefCount<cIGZBuffer>` at `tSound+0x0C` and `+0x10`, reads property `0x38` from the
   second buffer and derives the two pause-for-distance radii using the signed conversion,
   multiply order and separate f32 stores in §8.3.1;
5. sets property `0x10` to 3 with third argument 0, then plays the holder via vtable
   `+0x14(instanceId, managerMasterVolume, 0x200, 0)`. The volume is **manager** `+0x14`, not
   instance `+0x14` (which holds the manager reference). See §8.6 for the instruction-checked correction.

Instance lookup in general is the same idiom: hash bucket `id % bucketCount`, chain on `+0x04`
(`tSound::SetPosition` `0x004FA260`).

### 10.7 AEMS banks

Load path:

| Step | Address | What |
|---|---|---|
| `cSC4AudioAEMSEventSystem::LoadSoundBank` | `0x004E1E10` | reads property key `0xEBF44DF7` for the bank instance id, `StopAllSounds`, then builds TGI `{type=0x00000000, group=0x8A5971C5, instance=<prop>}` and calls `LoadBankFromResource` |
| `…::LoadBankFromResource` | `0x004E1DA0` | `new(0x24)` → `cSC4AudioAEMSSoundBank` ctor → `LoadFromResource` → `AddBank` |
| `cSC4AudioAEMSSoundBank::LoadFromResource` | `0x004E24E0` | acquires file, calls `LoadFromFile`, copies TGI only after successful Read, then closes/releases file; final success also requires Close (§10.7.1) |
| `…::LoadFromFile` | `0x004E2490` | `file->GetSize()` twice, `file->Read(bank+0x08, size)` — a raw slurp; **no validation here** |
| `…::AddBank` | `0x004E1BB0` | registrar receives `(raw image,0,0,identity callback 0x004E2410)`; result zero links into `+0x138` (§10.7.1) |
| `SNDAEMSI_addmodulebank` | `0x00867800` | the EA-library bank registrar |
| `_SNDAEMSI_resolvemodulebank` | `0x00864F60` | the pointer-relocation pass (every offset becomes an absolute pointer) |

`cSC4AudioAEMSSoundBank` (ctor `0x004E2420`, size **`0x24`**): `+0x00` vptr `0x00A932EC`, `+0x08` = the raw
bank image buffer, `+0x14/+0x18/+0x1C` = the cached TGI. `cSC4AudioAEMSEventSystem`'s constructor
(`0x004E1C20`) confirms service id `0xAC1FAE31` (`cRZSystemService(..., 0xAC1FAE31, 0x80000000)`).

**Bank header** (type `0x00000000`, group `0x8A5971C5`; **measured** from `SimCity_1.dat` — 43 of the 48
records in that group carry magic `AB`, 41,402,024 bytes in total, all with `version = 9` and
`moduleCount = 1`):

| Off | Size | Field | Measured (bank `0x8C0AE396`, 1,629,888 bytes) |
|---|---|---|---|
| `0x00` | 2 | magic `"AB"` | `41 42` |
| `0x02` | 1 | **format version** | `09`; `<8` → module stride `0x14`, `≥8` → stride `0x1C`; `>4` enables the `+0x34` relocation |
| `0x03` | 1 | pad | `00` |
| `0x04` | 4 | **total image size** | `0x0018DEC0` = 1,629,888 (= record size) |
| `0x08` | 4 | constant | `0x00184945`; identical in all 43 |
| `0x0C` | 4 | **module count** | `1` |
| `0x10` | 4 | **module table offset** | `0x38` (56) in all 43 |
| `0x14` | 2 | **bank id** | `30989` (`0x790D`) |
| `0x16` | 2 | unknown | `0x18` |
| `0x18` | 2 | count of `0xC`-byte records at the `+0x2C` table | `0` |
| `0x1A` | 2 | pad | `0` |
| `0x1C` | 4 | **offset of SNDBANK blob #1** | `0x0E40` |
| `0x20` | 4 | **trailing region #1 size** | `0x0018D080` = 1,626,240 |
| `0x24` | 4 | offset of SNDBANK blob #2 | `0` |
| `0x28` | 4 | trailing region #2 size | `0` |
| `0x2C` | 4 | **offset of the `0xC`-byte record table** | `0x834` |
| `0x30` | 4 | runtime only | 0 in the file |
| `0x34` | 4 | optional extra table, relocated only when `bank[+2] > 4` | 0 |

`size − region2 − region1` = 1,629,888 − 0 − 1,626,240 = **3,648**, the size of the header + module-table
prefix. `SNDAEMSI_addmodulebank` copies that many bytes and appends the decompressed bank blobs at `+3,648`.

**Module record** (stride `0x1C` for version ≥ 8; `0x14` below):

| Off | Size | Field | Measured |
|---|---|---|---|
| `0x00` | 2 | module id | `0x7923` |
| `0x02` | 2 | **instance count** | 1 |
| `0x04` | 2 | **record count** | 49 |
| `0x06` | 2 | update period | 0 |
| `0x08` | 4 | unknown | 0 |
| `0x0C` | 4 | **record table offset** | `0x54` |
| `0x10` | 4 | runtime ptr | 0 |
| `0x14`, `0x18` | 4 | unknown | `0x32`, `0x26` |

**Node records.** `_SNDAEMSI_resolvemodulebank` walks `recordCount` nodes from `mod+0x0C`, advancing by the
size byte of `DAT_00B106D4[type*8]` where `type = *(byte*)(node+4)`. Each node has: `+0x04` node type
(1 byte), `+0x06` count of 8-byte entries at `+0x08`, `+0x08` pointer to those entries (relocated, and each
entry's `+0` is relocated), `+0x0C` pointer relocated for types `04`, `06`, `11`, `1E`, `+0x10` used by types
`04` and `19`, `+0x14` relocated for type `08` when non-zero. The type table at `DAT_00B106D4` has 33 entries
of `(size, stride2, handler)`; handlers are dispatched at `_SNDAEMSI_updatemodule` `0x00865C70`.

`SNDAEMSI_addmodulebank` validates: `DAT_00B50A78 != 0` (sound system up, else returns `-10`); a free slot in
`DAT_00B508A4` (16 slots); registration of SNDBANK blob #1 (`FUN_0086B340`) and #2 if their offsets are
non-zero. **No `"AB"` magic check is visible** in `LoadFromFile` or `addmodulebank` *[inferred]* the magic is
consumed inside the SNDBANK blob (`FUN_0086B340` `0x0086B340` / `_SNDbankadd` `0x0086B35B`).

### 10.7.1 AEMS owner lifetime and IO results (AU48)

Constructor `0x004E1C20` constructs the system-service view at base `+0x0C` with service ID
`0xAC1FAE31` and flags `0x80000000`. QI `0x004E1430` answers `0xBB523BB2` at base,
`0x8C1FAEB7` at base+4, `0x452294AA` at base+8, and otherwise delegates to system-service
QI on base+0x0C. Successful QI AddRefs the service. AddRef/Release (`0x004E19C0/0x004E19D0`)
delegate to the system-service reference implementation; there is no second local counter.
The constructor makes empty storage/work vectors and bank list; gain `+0x110`, effects value
`+0x114`, master factor `+0x13C` and speed factor `+0x140` are 1.0, effects selector `+0x118`
is 37, response curve is constructed, image pointer `+0x12C` and adjacent size/end fields
are zero, and the handle vector contains 21 zero words. The speed-volume vector is initially
empty; tuning populates it. Backend options are obtained by Init, not default-filled here.

Init (`0x004E2210`, this = service view) uses process latch `0x00B21938`. If already set it
returns true without work. First call sets it **before** initialization, zeroes the handle
vector, invokes its own base+4 parameter setters for velocity, throttle and wobble with the
process value at `0x00B2193C`, reads preferences into master factor (signed volume /1024 or
zero when disabled), and reads tuning. It sizes the workspace to 200000 bytes, obtains backend
options, installs callback `0x004E13E0`, sets options, calls backend init with workspace and
constant `0x72B00`, then effects-bus init with `(0,trunc(effects value),effects selector,0,0)`.
The callback simply returns zero and performs no work. Any negative result from get-options,
set-options, backend init or effects-bus init invokes Shutdown and returns false; these
results use signed comparisons (`0x004E2308`–`0x004E2382`). After success it updates volume,
registers the primary event target with SndSys `+0x3C` if that service exists, attempts the
system resource load, and subscribes its base+8 message view to `0xA6B79602` if the global
message server exists. These later results, and ReadTuningValues' result, do not gate true.

Shutdown (`0x004E1AB0`, service view) returns true, including when the latch was clear.
When set, it clears the latch first, stops sounds, zeroes all handles, removes all banks,
enters the backend critical section, removes the event system, leaves critical, restores
the sound system, clears the workspace's logical length, unregisters the primary target
through SndSys `+0x40` when obtainable and unsubscribes base+8 from `0xA6B79602` when possible.
It retains container allocation. Destruction (`0x004E1920`, deleting wrapper `0x004E1D80`)
destroys owned vectors/list/storage and base service; it does **not** call Shutdown.

StopAllSounds (`0x004E1320`) invokes its base+4 EndSound slot for indices
0 through 14, then 17,18,20,19, in that order; it does not iterate all 21 indices.
StartSound (`0x004E16F0`, base+4 view) rejects an already nonzero handle. Otherwise it enters
critical, calls backend start through `0x00B107E4` with the address of the requested index,
leaves critical, and stores backend-result-plus-one when that is signed-positive, otherwise
zero. This attempted-start path returns true even on backend failure. EndSound
(`0x004E1740`) returns false for a zero handle; otherwise it calls `0x00B107EC(handle-1,&index)`
inside critical, clears the handle and returns true regardless of backend result. Neither
method bounds-checks the index. Parameter setters `0x004E1530/0x004E15A0/0x004E1610` clamp
ordinary f32 inputs to [0,1], multiply by f32 127, truncate toward zero, and call
`0x00B107F0(parameter index 0/1/2,value)` inside critical; return backend result >=0.
The instruction comparisons leave NaN on the conversion path, not a finite clamped endpoint.

DoEvent (`0x004E14B0`) handles master-volume event `0x2A5B69C0`: it reads payload `+0x24`
as unsigned u32, divides by 1024, stores f32 master factor, evaluates the response curve
on `(speed factor * tuning gain) * unrounded master factor`, then multiplies its result by
f32 127, truncates and calls SNDmastervol. It always returns false, including after effects.
No event/payload null gate or ownership acquisition occurs here. Ordinary UpdateVolume
instead evaluates `(speed factor * stored master factor) * tuning gain`, rounded to f32
at the curve argument. Message callback `0x004E1680` runs on base+8 and handles only
`0xA6B79602`; it indexes the speed-volume vector using global simulator `+0x0C` without
bounds checks, stores that f32 speed factor, evaluates `(master factor * tuning gain) *
speed factor`, scales/truncates by 127 and calls SNDmastervol, returning true. Other messages
return false. Numeric order is instruction-checked at `0x004E13F0`–`0x004E141A`,
`0x004E14DA`–`0x004E151D`, `0x004E16A8`–`0x004E16DC`; 127 is f32 at `0x00A931C0`.

**Bank and file ownership.** AddBank rejects null or any nonzero registrar result; **zero
means success**. It passes `(bank raw image,0,0,0x004E2410)` to the registrar. That callback
returns its first argument unchanged; it is not a heap allocator despite the earlier name.
On success AddBank stores zero at bank+0x20, retains the bank and appends it to the bank list
(`0x004E1BBC`–`0x004E1C1D`; the shared helper `0x006CE100` is a field setter, not a destructor).
RemoveBank (`0x004E1790`) matches the first bank's cached **instance** ID, calls backend
remove with bank+0x20, unlinks it and drops the list and temporary references; missing ID
returns false. RemoveAllBanks removes from the head, calls backend remove for each and
balances those same references. Backend removal results are ignored. UnloadSoundBank
(`0x004E18B0`, base+4 view) reads property `0xEBF44DF7`, stops sounds, zeroes handles,
then removes that bank. LoadSoundBank uses the same property, stops sounds, then loads its TGI.

A new bank has refcount zero, null raw vector and zero cached TGI/handle. LoadBankFromResource
holds one temporary reference and releases it after load/AddBank success or failure. Allocation
returning null has no safe false-return path: the load method is still invoked on that pointer.
Final bank Release (`0x004E2450`) frees raw vector storage and outer allocation; count >1
merely decrements. LoadFromFile borrows the caller's file without AddRef/Release/Close.
It rejects null, resizes byte storage to the **first** GetSize result and passes the **second**
GetSize result to Read. It returns Read's boolean with no size equality, bytes-read or header
check (`0x004E24A3`–`0x004E24CF`). A shorter second size leaves the rest unvalidated; a larger
one may overrun allocation. There is no specified safe recovery or fabricated false for
that overrun or allocation failure. A failed read leaves the resized storage in the bank.
LoadFromResource owns its acquired file: after a successful acquisition it always calls
file `+0x14` (Close) then releases it. Only a successful Read copies the cached TGI, and
the resource loader's final success is the Close result. This corrects §10.7's abbreviated
claim that the TGI is copied before reading. Failed acquisition releases any returned reference.

The **system** LoadFromFile (`0x004E2120`) uses the same two-size raw-read policy for its
system image. Only a true Read enters critical, removes the old backend system and adds the
new image; add result zero means true. False Read performs neither backend call, even if
some bytes were delivered. Resource wrapper `0x004E21A0` likewise closes/releases the file
and returns Close success only after successful load. Neither reader validates a short
successful read. Tuning acquisition failure logs and returns false with existing values
unchanged. Successful acquisition reads the three scalar keys independently, preserving
their old values when absent, resets the curve to its default pair before applying the
curve property, resets the speed vector to its five defaults before applying its property,
and returns true without requiring every key or validating the resulting vector length
(`0x004E1EA0`–`0x004E2118`). Init ignores a false tuning result.

**Conformance cases.** Repeat Init after success does no IO; negative get-options runs the
full latched shutdown and permits a later retry. Failed final system-resource load does not
change Init's true result. StopAll requests exactly the 19 indices above, leaving 15 and 16
alone. Backend start -1 stores zero but returns true; a second successful start at the same
index is allowed. Parameter 0.5 passes 63; input 2 passes 127. DoEvent updates volume but
returns false. Registrar zero appends/retains a bank; -1 does neither. RemoveBank matches
instance ID alone. File sizes (8,4) allocate 8 and request 4; (4,8) request 8 despite allocation
4, an unsafe original quirk, not a requirement to corrupt replacement memory. False Read
does not cache TGI; true Read plus false Close caches TGI but reports resource-load failure.
Failed system Read does not remove/add the backend system. Missing tuning resource preserves
old tuning, while a successful resource with no speed key restores the five-element default.

### 10.8 Tuning

All three audio tuning readers pull the **same exemplar**: type `0x6534284A`, group `0xCA25875D`,
instance `0x6A359C5C`, through interface `0xA52160F5` (the persist-resource manager at `DAT_00B43CA8`) and
`cSCPropertyHelp::GetPropertyValue` (`0x005FD390` family; the four-int overload is `0x005FD480`).

**Measured exemplar** `0x6534284A/0xCA25875D/0x6A359C5C` ("Audio Settings") in `SimCity_1.dat`, 641 bytes:

| Prop | Type | Value |
|---|---|---|
| `0x00000010` | u32 | 37 |
| `0x00000020` | string | `"Audio Settings"` |
| `0x0A36F410` | f32×5 | 0, 0, 188, 150, 73 |
| `0x0A946A57` | u32 | 10 |
| `0x2A35A25F` | f32 | 1 |
| `0x2A36E3CB` | f32 | 2 |
| `0x2A3FF8CD` | u32×94 | instance ids of the 94 ambience-layer exemplars |
| `0x2AA40CCC` | f32×5 | 10, 4, 2, 1, 0.5 |
| `0x4A946A5D` | u32 | 30 |
| `0x4BFDD955` | f32×8 | 0, 0, 0.5, 0.35, 0.75, 0.6, 1, 1 |
| `0x6A359F22` | f32×5 | 1, 1, 1, 1, 1 |
| `0x6A946A50` | f32×10 | 0, 1024, 2, 950, 3, 900, 4, 850, 5, 800 |
| `0x8C376D4F` | f32×5 | 0.4, 0.5, 0.6, 0.85, 1 |
| `0xAABAA0D3` | u32 | 3 |
| `0xCA359F17` | f32×5 | 1336, 500, 260, 140, 100 |
| `0xCBFB03FA`, `0xCBFB03FF` | f32 | 1, 1 |
| `0xEA4A7C55` | f32×5 | 1, 1, 1, 1, 0 |
| `0xEBFB0404` | u32 | 37 |

`cSC4AudioAEMSEventSystem::ReadTuningValues` (`0x004E1EA0`) reads five keys:

| Key | Destination | Default (ctor `0x004E1C20`) | Measured |
|---|---|---|---|
| `0xCBFB03FA` | `+0x110` | `1.0f` | 1 |
| `0xCBFB03FF` | `+0x114` | `1.0f` | 1 |
| `0xEBFB0404` | `+0x118` | `0x25` (37) | 37 |
| `0x4BFDD955` | `+0x11C` (`cSC4ResponseCurve`) | pre-seeded `(1.0, 1.0)` | `(0,0) (0.5,0.35) (0.75,0.6) (1,1)` |
| `0x8C376D4F` | `+0x144` (5-float vector) | from `DAT_00A931A8` = `(0, 0, 0, 0.5, 1)` | `0.4, 0.5, 0.6, 0.85, 1` |

`cSC4AudioSoundManager::_ReadTuningValues` (`0x004F9250`) reads four:

| Key | Destination | Default | Measured |
|---|---|---|---|
| `0x6A946A50` | `+0x70` response curve | `(0, 1024.0)` | `(0,1024) (2,950) (3,900) (4,850) (5,800)` |
| `0xAABAA0D3` | `+0x64` | 3 | 3 |
| `0x0A946A57` | `+0x68` | 10 | 10 |
| `0x4A946A5D` | `+0x6C` | 30 | 30 |

It is re-read on the **exemplar-changed** message `0xC53D10AA` ([foundations.md](foundations.md) §5).
`+0x64` and `+0x68` are the voice-count gates used by `_StartSound` (`0x004F9D70`).

`cSC4AudioAmbienceLayer::_ReadTuningValues` (`0x004E7A90`) reads a **per-layer** exemplar of the same type and
group, instance = its argument (one of the 94 ids from prop `0x2A3FF8CD`), and sets the layer's name to
`"Noname_0x%08x"` when prop `0x00000020` is absent.

**No clamps** are applied in any of the three readers *[compat]*.

### 10.9 `cSC4SoundDescription` and the `soundEffect` command

`cSC4SoundDescription` (ctor `0x005BE300`, object size **`0x1C`**):

| Off | Type | Init | Meaning |
|---|---|---|---|
| `+0x00` | vptr | `0x00AA10EC` | |
| `+0x04` | `cRZRefCount` | | inner vptr `0x00AA10D8` |
| `+0x0C` | u32 | 0 | unused / flags |
| `+0x10` | u32 | 0 | **sound id** |
| `+0x14` | f32 | `0x3F000000` (0.5) | `1.0 / locationUpdateRate` (0 when rate ≤ 0) |
| `+0x18` | u32 | 0 | **length** |

Produced by `(anonymous_namespace)::cGroupSoundCommand::Parse` (`0x005A2A50`) — the effects-script
`soundEffect` command:

- fatal `"Not in effects block"` outside an effects block;
- fatal `"sound description unimplemented"` when `nSCRes::FindNonSwitchIndices` finds non-switch indices;
- `-name` is uppercased and looked up in `std::map<std::string, unsigned long>` at `cSC4EffectsParser+0xC4`;
  a miss is fatal `"Unknown sound %s"`; no `-name` is fatal
  `"Need at least -name for anonymous soundEffect"`. On a hit, `desc->+0x10 = map[name]` — **this is where a
  sound id is bound to a name**;
- `-locationUpdateRate` → `desc->+0x14 = (rate > 0) ? 1.0f/rate : 0`;
- `-length` → `desc->+0x18 = (int)length`; if 0, `AddAnonDescription` is told to treat it as a one-shot;
- finally `cSC4EffectsParser::AddAnonDescription(...)` and `desc->vtable[5]()` (Release).

### 10.10 The Hit track formats

**TKDT** (`0x0B8D821A`) is **CSV text** wrapped `TKDT,` … `,ETKD`, some records prefixed `2DKTJ\0\0\0`
(**measured**), e.g. `TKDT,2,,0xec08d88a,0x09400fc5,,,,,,100,2110647597,,0,1024,,,,,,,3000,,,,768,,,,,ETKD`.
Fields 4 and 5 are instance ids of `.tlo` records in the TLO segment.

`cHitResourceFactory::CreateInstance` (`0x00942F24`, vtable `0x00AD40D0`) dispatches on type:
`0x0B8D821A` or `0x0B9EB87E` → `cHitPropertySetTrackResource` (0x28 bytes); `0x7B1ACFCD` → `FUN_0094048B`
(0x28); `0xA3CD96CF` → `FUN_009402A1` (0x114); `0xEBFEE345` / `0xEBFEE349` → `FUN_009396D8` (0x28).

**Hit list** (`0x7B1ACFCD`, group `0x9DBDBF74`) is `u32 version = 1; u32 count; count × u32 instance-id`
(**measured**). *[inferred]* the entries are instance ids of `.hit` records and the high byte carries instance
flags.

### 10.11 The 12 records of type `0x9D796DB4` — answered

**There is no hard-coded reader.** The constant `0x9D796DB4` has **zero byte occurrences** in the whole of
`SimCity4.exe` (all 7,524,352 bytes were scanned for `B4 6D 79 9D` and for the `CMP` immediate form; 0 hits).
For contrast `0x0B8D821A` appears at `0x00942F35` / `0x0093FF50`, `0x7B1ACFCD` at `0x00942F50`, `0x2026960B`
at `0x009840B5` / `0x00984103` / `0x0098419E`.

**Corrects:** INVENTORY-M6 §5's open item **IN6-3** ("the reader of the 12 records of type `0x9D796DB4` in
`Sound.dat`"). **ANSWERED**: the type is bound purely at runtime by
`cHitIniFile::ProcessDBSegmentDirectoryFilesEntry` (`0x00936707`) from the INI line
`0xca4d19e3=0x9d796db4:tlo,0x2026960b:hit` — type `0x9D796DB4` → extension **`tlo`** in the TLO segment
(group `0xCA4D19E3`). The payload is consumed generically through the HitMan directory
(`QueryInterface(0x856C8CF7)`, `vtable+0xC(type, ext)`, HitMan `vtable+0xB8(type, group)`), never by a
type-specific switch.

**Measured payload shape** (smallest record, instance `0xA9225D22`, 48 bytes decompressed): a 16-byte header
`u32 0, u32 count, u32 0, u32 0`, then `count` variable-length entries whose first 8 bytes are one of a few
repeating 64-bit patterns. The entry stride is **not** constant ((size−16)/count varies from 32 to 40 across
the 12 records) *[inferred]* these are track-layout records keyed by 64-bit ids, consumed with the TKDT
references.

---

## 11. The 92 classes of `cSC4AudioCOMDirector`

The constructor `0x004EC710` (1615 bytes) calls `cRZCOMDllDirector::AddCls` (`0x0090E133`) **92 times**.
The first nine are the named managers of §1.2; the other **83** are small leaf classes.

**Corrects:** INVENTORY-M6 §5 says "the audio director registers about 80 further classes from factories at
`0x004EB260`–`0x004EC6F0` *[unsettled: per-event handlers or emanation models]*". **PARTLY.** Measured: 92
registrations in total, so **83** further classes (the "about 80" is right); the factory range is wider than
M6 states — the factories run from **`0x004EB080`** to **`0x004EC6F0`**, and the named managers sit at
`0x004EB080`–`0x004EB180` while the unnamed ones occupy `0x004EB260`–`0x004EC6F0` in two blocks.

### 11.1 What the 83 are

Every one of the 83 factories is a small function that allocates a **fixed-size object** from the ordinary
heap (`operator new` `0x005E55E0`, which locks `DAT_00B230E0` and allocates from heap `DAT_00B230E8`), then
writes a vtable and a few zeroed fields. Example, factory `0x004EB260` (CLSID `0x4A401BA7`):

```
push 0x10                     ; 16-byte object
call 0x005E55E0               ; locked operator new
mov  dword ptr [eax],    0x00A937D0   ; vtable
mov  dword ptr [eax+4],  0            ; u32
mov  byte   ptr [eax+8], 0            ; u8
mov  dword ptr [eax+0xc],0            ; u32
ret
```

**Measured allocation sizes** — the 83 split into exactly three shapes, and the 9 named managers each have
their own size:

| Size | Count | Which | Layout |
|---|---|---|---|
| **8 bytes** | 16 | factories `0x004EC6F0`, `0x004EC3D0`–`0x004EC590` | `{vtable, refcount(+4)}` |
| **16 bytes** | 31 | factories `0x004EB260`–`0x004EB7D0`, `0x004EC600` | `{vtable, id(+4), flag u8(+8), refcount(+0x0C)}` |
| **20 bytes** | 36 | factories `0x004EC650`, `0x004EB800`–`0x004EBE90` | `{vtable, …, refcount}` |
| 32 | 1 | `0x004ED700` | `cSC4AudioSemManager` |
| 52 | 1 | `0x004EB100` | `cSC4AudioEventHandlerDisasterFire` |
| 80 | 1 | `0x004EB0A0` | `cSC4AudioEventHandler` |
| 124 | 1 | `0x004EB0E0` | `cSC4AudioListener` |
| **128** | 1 | `0x004EB080` | **`cSC4AudioSoundManager`** |
| 144 | 1 | `0x004EB150` | `cRZRadio` |
| 152 | 1 | `0x004EB120` | `cSC4AudioScapeManager` |
| 256 | 1 | `0x004EB180` | `cRZDiskRadioStation` |
| 288 | 1 | `0x004EB0C0` | `cSC4AudioAmbienceManager` |

Their vtables are packed at a stride of **`0x18` (6 slots)**. The slot pattern is:

| Slot | Offset | Family A (`0x00A937D0`…) | Family B8 (`0x00A93E5C`…) | Family B20 (`0x00A93AA0`…) | Meaning |
|---|---|---|---|---|---|
| 0 | `+0x00` | `0x004EC5D0` | `0x004EC6C0` | `0x004EBA70` | QueryInterface |
| 1 | `+0x04` | `0x004664A0` | `0x005F47F0` | `0x004F7BA0` | AddRef |
| 2 | `+0x08` | `0x004EC5B0` | `0x004EC6A0` | `0x004EC630` | Release |
| 3 | `+0x0C` | `0x00648F00` | `0x00648F00` | `0x00648F00` | **shared stub** |
| 4 | `+0x10` | `0x00648F00` | `0x00648F00` | `0x00648F00` | **shared stub** |
| 5 | `+0x14` | **unique per class** | **unique per class** | **unique per class** | the class's one real method |

The shared stub `0x00648F00` is `mov al, 1; ret` — slots 3 and 4 are **predicates that always return true**
*[compat]*. Slot 5 is the only per-class behaviour; sampled bodies (`0x004E4370`, `0x004E4490`,
`0x004E2E10`, `0x004E3100`) compare an argument against `this+4` and return `this+8` or format a 0x40-byte
stack buffer; one sample (`0x004E2AA0`) tail-calls `(*DAT_00B43DDC)->slot9`.

**Refcount offset differs per family** (measured from the AddRef/Release bodies):

| Family | AddRef | refcount at | Release |
|---|---|---|---|
| A (16 bytes) | `0x004664A0` `inc [ecx+0xC]` | `+0x0C` | `0x004EC5B0` |
| B8 (8 bytes) | `0x005F47F0` `inc [ecx+4]` | `+0x04` | `0x004EC6A0` |
| B20 (20 bytes) | `0x004F7BA0` `inc [ecx+0x10]` | `+0x10` | `0x004EC630` |

`Release` (`0x004EC5B0`) decrements only while the count is **greater than 1**; at `≤ 1` it tail-calls
`operator delete` (`0x005E5620`) and returns 0. So the refcount is the number of live references and the
object is destroyed on the last one, which means the factories' zero-initialised refcount must be brought to
1 by the caller's `AddRef` (the COM factory protocol does this) *[compat]*.

**`QueryInterface` answers two IIDs and returns `this` for both** (measured, `__stdcall` `ret 8`):

| Family | QI impl | IIDs answered |
|---|---|---|
| A | `0x004EC5D0` | `0x4A40074C`, `0xCA400753` |
| B8 | `0x004EC6C0` | `0x4A40074C`, `0xEA40074F` |
| B20 | `0x004EBA70` | `0x4A40074C`, `0xCA400757` |

Body: `if (iid == A || iid == B) { *out = this; this->AddRef(); return 1; } return 0;` — there is **no
sub-object offset**; the two IIDs are aliases for the same interface at `+0x00`. **`0x4A40074C` is the common
IID** shared by all three families *[compat]*. None of the four IIDs appears in `re/class_id_table.tsv`.

**Answer to IN6-2.** These 83 are **neither** the emanation models nor the event handlers: the emanation
models (`cSC4AudioEmanationModel2D` / `3D` / `3DEffects`, §3.4) and the event handlers
(`cSC4AudioEventHandler`, `cSC4AudioEventHandlerDisasterFire`) are separate named classes with much larger
vtables and their own factories. The 83 are a **registry of small tagged leaf objects**, one CLSID each, with
QI/AddRef/Release, two always-true capability predicates and one unique method, in three fixed shapes (8, 16
and 20 bytes) *[inferred]*. What would settle the remaining question of their role: who looks their CLSIDs up
(a scan of the 83 IDs against the tuning data, and the `+0x20` constant of §3.4 as a cross-check).

### 11.2 The 83 registrations

Format: `CLSID ← factory`. Vtable address is in §11.1's families by factory block.

`0xCA426982←0x004EC6F0`, `0x8A401E0F←0x004EC3D0`, `0x2A426986←0x004EC3F0`, `0x8A42698A←0x004EC410`,
`0x6A42698E←0x004EC430`, `0x0A426991←0x004EC450`, `0x6A426995←0x004EC470`, `0x0A426998←0x004EC490`,
`0xAA54E710←0x004EC4B0`, `0xAA54E715←0x004EC4D0`, `0x0A54E719←0x004EC4F0`, `0x0B817C41←0x004EC510`,
`0xAB817C0B←0x004EC530`, `0xCB817C1B←0x004EC550`, `0x2B817C27←0x004EC570`, `0x8B817C32←0x004EC590`,
`0x0A401BA0←0x004EC600`, `0x4A401BA7←0x004EB260`, `0xCA401BAE←0x004EB290`, `0x6A401BB3←0x004EB2C0`,
`0x6A401BBA←0x004EB2F0`, `0x0A42B520←0x004EB320`, `0x6A43C75D←0x004EB350`, `0xCA5A6A5F←0x004EB380`,
`0x0A5A6A64←0x004EB3B0`, `0x2A5A6A68←0x004EB3E0`, `0x6A5A6A6B←0x004EB410`, `0x4A5A6A6F←0x004EB440`,
`0x0A5A6A72←0x004EB470`, `0x0A5A6A76←0x004EB4A0`, `0xAA5A6A7A←0x004EB4D0`, `0xAA5A6A7D←0x004EB500`,
`0xCA5A6A81←0x004EB530`, `0x2A5A6A84←0x004EB560`, `0x4A5A6A88←0x004EB590`, `0xEA5A6A8B←0x004EB5C0`,
`0x6A5A6A8F←0x004EB5F0`, `0x4A5D0EB0←0x004EB620`, `0xAA5D0EB5←0x004EB650`, `0xEA5F4954←0x004EB680`,
`0x8A5F603E←0x004EB6B0`, `0x8A60F033←0x004EB6E0`, `0x2A872395←0x004EB710`, `0xCA9D68A1←0x004EB740`,
`0xCA9D68A5←0x004EB770`, `0xAA9D68A9←0x004EB7A0`, `0x2AA17957←0x004EB7D0`, `0x8A427F7C←0x004EC650`,
`0xEA4298FC←0x004EB800`, `0x2A429900←0x004EB830`, `0x4A429903←0x004EB860`, `0x2A429907←0x004EB890`,
`0xCA42990A←0x004EB8C0`, `0x8A53E23E←0x004EB8F0`, `0xCA53E243←0x004EB920`, `0x0A53E247←0x004EB950`,
`0x4A53E24B←0x004EB980`, `0x2A53E24F←0x004EB9B0`, `0xCA53E252←0x004EB9E0`, `0x0A53E290←0x004EBA10`,
`0x2A53F0C3←0x004EBA40`, `0x8A53F0E4←0x004EBAA0`, `0x4A53F0E8←0x004EBAD0`, `0x8A53F584←0x004EBB00`,
`0xEA5A6FCE←0x004EBB30`, `0x8A5A6FD1←0x004EBB60`, `0xEA5A6FD4←0x004EBB90`, `0x6A5D0682←0x004EBBC0`,
`0x0A5D068D←0x004EBBF0`, `0x2A5D0691←0x004EBC20`, `0xEA5F495C←0x004EBC50`, `0x4A60D870←0x004EBC80`,
`0x8A60D873←0x004EBCB0`, `0x6A60F166←0x004EBCE0`, `0xEA80835E←0x004EBD10`, `0x0A808367←0x004EBD40`,
`0xCA80836C←0x004EBD70`, `0x2A808371←0x004EBDA0`, `0x4A808375←0x004EBDD0`, `0x4A8084F4←0x004EBE00`,
`0x2A9D6690←0x004EBE30`, `0xAA9D6694←0x004EBE60`, `0x2A9D6697←0x004EBE90`.

---

### 11.3 Director execution and lifetime (AU47)

The constructor (`0x004EC710`) registers the nine named classes first, in this order:
`AA355871`, `29E2797F`, `BDB6171E`, `69E500BC`, `6A12F51C`, `4A47DC0D`, `CA4A66B9`,
`5CFBDBBF`, `5CFBDBD5`. The 83 leaf registrations then follow §11.2's literal table order.
There is no interleaving with the nine named registrations. This constructs registration records,
not instances of those classes. Service holders `+0x4C` through `+0x68` start null; the base
director is constructed first (`0x004EC713`–`0x004EC748`).

OnStart (`0x004EB1B0`) first registers the director's `this+4` framework hook using framework
slot `+0x1C`. It then performs §16.16's five add/start pairs, passing the original OnStart
argument to every child. Neither AddDirector nor child OnStart results gate the next pair.
It returns true after all five pairs. The framework and child directors are required pointers;
there is no checked null-service failure path here.

PostAppInit (`0x004EBF40`) begins with result false. It obtains the Hit configuration resource
with group `0x8A5971C5`, instance `0xEA8A1115`, and constructs the Hit INI reader on the returned
file. Failed resource acquisition or failed default-entry enumeration returns false, releases
the temporary file reference and creates no services. The reader is destroyed when constructed.
After successful enumeration, the following operations occur:

| Order | Operation and ownership |
|---|---|
| 1 | Create radio `0x5CFBDBBF`, IID `0x287FB697`, into a temporary reference; call its `+0x20`, then framework `+0x0C(radio)` only when each preceding operation succeeded |
| 2 | Create SEM `0xAA355871`, same IID, into director holder `+0x58` |
| 3 | Create scape `0x29E2797F`, IID `0x29E39DB4`, into `+0x64` |
| 4 | Create handler `0xBDB6171E`, IID `0xBB523BB2`, into `+0x48` relative to this callback's interface view |
| 5 | Create second handler `0x69E500BC`, IID `0xBB523BB2`, into `+0x4C` relative to that view |
| 6 | Create ambience manager `0x6A12F51C`, same IID, into `+0x50` relative to that view |
| 7 | Create sound manager `0x4A47DC0D`, same IID, into `+0x5C` |
| 8 | Create listener `0xCA4A66B9`, same IID, into `+0x60` |
| 9 | Call first handler `+0x0C`, ambience manager `+0x20`, SoundManager Init, Listener Init, then SEM initialization `0x004EDAA0` |
| 10 | QI second handler then ambience manager for `0x89E51472`; on each successful QI call `+0x0C`, balancing each temporary reference |

The mixed offsets in this table are deliberate: PostAppInit executes on the director's secondary
view; its handler holders are four bytes above those addresses in the base object. Do not use the
decompiler's negative-array object expressions as ownership rules.

Only successful radio creation **and** radio initialization **and** framework registration sets
the result true (`0x004EBF58`, `0x004EBFE1`–`0x004EC005`). Any later failed creation or initialization
clears it. A failed optional QI in order 10 does not clear it. Remaining creations and initializers
are still attempted: this is not a first-failure transaction and there is no unwind. In particular,
a failed creation followed by an unconditional initializer dereferences its output; the original
does not promise a clean false return for a null output. Before replacing any retained service,
the old reference is cleared and released. Temporary radio and hook references are released on exit.
The result is loaded back after reader/file cleanup (`0x004EC286`–`0x004EC2A4`), rather than being
the destructor's result as the pseudocode suggests.

PreAppShutdown (`0x004EC2B0`) visits second handler, first handler, SEM, listener, sound manager,
ambience manager, scape, in that order. Handlers are queried for `0x89E51472` and receive `+0x10`
only on successful QI; SEM uses its teardown (misnamed Init), listener and sound manager use
Shutdown, ambience uses `+0x24`, scape `+0x10`. Each existing holder is then cleared and released;
the temporary handler hook is balanced. It returns true and skips null holders. Destruction is
different: `0x004EBEC0` releases holders in descending order `+0x68, +0x64, +0x60, +0x5C, +0x58,
+0x54, +0x50, +0x4C`, then destroys the base director. It does not perform PreAppShutdown.
Deleting destructor `0x004EC680` frees outer storage only when deletion flag bit 0 is set.

**Conformance cases.** Record the 92 AddCls calls and require the nine-class prefix above before
the 83 leaves; make every child OnStart return false and still require all five pairs and true
from the parent. Deny the configuration file and require false with no service creation. Let
radio registration fail but every later initializer succeed: the final result remains false.
Let radio registration succeed and a later initializer fail: require later initializers to run,
false at exit, and no automatic shutdown. With all required outputs valid, deny either optional
hook QI: it skips only that hook initialization. Check shutdown order separately from destructor
reference-release order, including a second shutdown with cleared holders.

## 12. Save behaviour

**Nothing in the audio layer is saved.** *[compat]*

Evidence:

- No class in `decomp/cSC4Audio*.c`, `decomp/cGZSnd*.c`, `decomp/cHit*.c`, `decomp/cFresh*.c`,
  `decomp/cRZRadio.c` or `decomp/cRZDiskRadioStation.c` has a `GetGZCLSID` method. A class that writes a city
  record needs one so the loader can rebuild it.
- The only `Read`/`Write`-named methods in the whole layer are **resource and tuning** readers, not
  `cIGZSerializable` pairs:
  - `cHitPropertySetTrackResource::Write` `0x0093FF4A` / `::Read` `0x00940DFF` — the Hit track *resource*
    format;
  - `cHitList::ReadResource` `0x00943DBC`;
  - `cSC4AudioAEMSEventSystem::ReadTuningValues` `0x004E1EA0` and
    `cSC4AudioPreferencesWinProc::ReadTunerVariables` `0x004F22C0`.
- [save-records.md](save-records.md) contains no audio record.

Consequences a replacement must match:

1. Sound instances (`automaton+0x8C`) are **not** in the save. After a load every automaton starts with no
   sound playing, and a state that was mid-SFX on save does not resume it.
2. The sound-event pool, the AEMS banks' loaded state, the Hit and freshness players and the radio's playlist
   position are all **runtime-only**.
3. Because `HandleEnterState` only starts a sound on state entry, a loaded automaton that is already inside a
   sounding state will be **silent until it next changes state** *[quirk]*.

---

## 13. Messages the audio layer sends and receives

| Type | Name | Direction | Where |
|---|---|---|---|
| `0x8A385DD8` | AutomataPlaySFX | receive | `cSC4AudioEventHandler::DoMessage` `0x004EF810` → `HandleAutomataMessages` `0x004EE560`; sent by `cSC4Automaton::StartSFX` `0x00509950` |
| `0xAA396960` | AutomataStopSFX | receive | same; sent by `cSC4Automaton::StopSFX` `0x00505B50` |
| `0xA9E3B6E3` | | receive | → `HandleAutomataMessages` |
| `0xEA3ADA33` | | receive | → `HandleAutomataMessages` |
| `0x8A4A69C7` | `kMessageListenerPosChanged` | send + receive | listener **sends** (`0x004F1880`); sound manager receives (§8.3), ambience manager receives ([audio-ambience.md](audio-ambience.md) §4.2); listener camera inputs are `0xA6B79602` and `0xA6B79612` ([supplement §18](audio-owner-contracts.md)) |
| `0x2A3AD653` | `kMessageSetRadioStation` | send + receive | posted by `DoMessage` on `0x26D31EC5`; handled by `HandleRadioMessages` `0x004EF010` |
| `0x26D31EC4` / `0x26D31EC5` | PreCityInit / PostCityInit | receive | `DoMessage` `0x004EF810` |
| `0x89F10866` / `0xA9F10E45` | construction | receive | → `HandleConstructionMessages` `0x004EEC10` |
| `0xAA611B6E`, `0xAA650861`, `0xC9F1011A`, `0xE94671EB` | | receive | `DoMessage` `0x004EF810` |
| `0xFA0C6390` | `kMouseMessageType` | receive | → `HandleMouseMessagesForModalWindow` `0x004EF5D0` |
| `0xDB53383B` | `kSndEventMsgTypeStd` | internal | the type a `cGZSndEventStd` carries |

All sends go through `(*DAT_00B43CCC)->slot4(msg, 0)` = `MessagePost(msg, false)` (foundations §5,
implementation `0x004AB778`) unless a site is listed otherwise.

---

## 14. Quirks *[quirk]*

1. **`StopSFX` never clears the instance id.** `cSC4Automaton::StopSFX` (`0x00505B50`) reads `this+0x8C` and
   never writes it. Calling it twice posts two `AutomataStopSFX` messages carrying the same instance id.
2. **`StartSFX` overwrites the instance id without stopping the previous sound.** `0x00509994` stores the new
   id at `this+0x8C` unconditionally. Starting a second SFX while one is playing orphans the first instance
   id: `StopSFX` can only stop the second.
3. **`StopSFX` returns 1 even when it did nothing.** The `this+0x8C == 0` path jumps straight to
   `mov al, 1` at `0x00505BD0`. `StartSFX` by contrast returns 0 when the id is 0.
4. **Message type 0 does not set the presence bit.** `SetType` (`0x009132DB`) stores 0 but clears bit 0 of
   `+0x08`, so a message with type 0 is indistinguishable from one whose type was never set.
5. **The rate limiter only fires for one magic payload.** `HandleAutomataMessages` (`0x004EE560`) applies the
   1000–4000 ms limiter only when `payload->slot9()` equals `0xA9F14C47`; every other payload value plays
   immediately. The limiter state is on the **handler**, not on the automaton, so it throttles all matching
   sounds game-wide.
6. **Dead code in `StartSFX`.** The block at `0x00509987` (`mov eax,[ecx]; call [eax+8]`) is unreachable: the
   branch that sets `EBX` bit 0 is exactly the branch that zeroes `ECX`, and the guard tests both. A
   decompiler will show a spurious `Release` on the argument *[quirk]*; the argument is a `u32` SFX id, not a
   refcounted object (§6.1).
7. **A size other than `0x38` bypasses the sound-event pool.** `cGZSndEventStd::operator.new`
   (`0x0096BAD1`) falls through to the ordinary heap for any other size, so a replacement that changes the
   object size silently loses the pool and its 16 KB page behaviour.
8. **The pool's page-state word carries a `0x40000000` marker.** `FUN_0090CF9A` adds it on page entry and
   `FUN_0090CFF9` clears it with `add 0xC0000000`; the free path special-cases the value `0x40000045`.
   Treating `+0x0C` as a plain live count is wrong.
9. **A loaded automaton inside a sounding state is silent.** Because `HandleEnterState` is the only place
   `StartSFX` is called and `StartSFX` is only called on state entry (§12.3), a save/load mid-state drops the
   sound permanently.
10. **`cGZSndMgr` almost entirely fails to decompile** (`Field field5_0x18 does not fit in structure
    cGZSndMgr`). The method names and entry points are still exact; only the bodies are unavailable in the
    export. A replacement must work from the names and the interface shape, not from a body-level reading.
11. **The manager never sees the automata SFX messages.** `cSC4AudioSoundManager::DoMessage` (`0x004F9430`)
    has no case for `0x8A385DD8` or `0xAA396960`. The play/stop request is converted into a sound-system
    event by the audio event handler first (§8.7). A replacement that routes the messages straight to the
    sound manager will not match.
12. **A stop for an unknown instance id fails silently but reports success.** `_StopSound` (`0x004F94C0`)
    returns false and does nothing on a miss; its caller `FUN_004F9990` discards that and returns 1.
13. **The rate-limit window can be exactly 4000 ms.** `1000 + rand()*3000/0x7FFF` with `rand()` in
    `[0, 0x7FFF]` gives `[1000, 4000]`, not `[1000, 3999]`.
14. **A sound component ignores its stop mode.** `cSC4SoundComponent::Stop` (`0x005BE4A0`) never reads its
    argument, so a hard and a soft stop are identical; it also clears `+0x1C` *before* checking whether the
    sound system is present.
15. **An effect sound's position is quantised to whole units.** `Update` (`0x005BE620`) converts x, y, z with
    `ftol` before posting `0xCA45190F`, and the receiver (`FUN_004FA4C0`) converts back with `(float)`, so an
    effect sound's position is rounded to integers.
16. **A non-ignored sound with `length == 0` pins its effect forever.** `IsActive` is the started byte
    (`0x00580B50`) and only `Stop` clears it; a description with `+0x18 == 0` never self-stops (§9.2).
17. **The text parser's ignore-bit rule does not hold in the shipped data.** `cGroupSoundCommand::Parse`
    (`0x005A2A50`) derives bit 0 as `(length == 0)`, yet three of the four shipped components that have the
    bit clear also have `length == 0` (§9.3).
18. **`0xC53D10AA` is handled but never subscribed** by the sound manager (§8.3), so the tuning refresh can
    only arrive by a direct post.

---

## 15. Open items

The [owner audit in §§17–21](audio-owner-contracts.md) adds unresolved implementation questions in
[questions/audio.md](questions/audio.md), AU42–AU43. AU41 is settled in §19.2; AU40 and AU44 in §18.4 and
§7.4.1 respectively. The remaining questions qualify the coverage of those sections;
an owner appearing in the inventory does not imply that all its numerical behavior is settled.

| # | Question | What would settle it |
|---|---|---|
| **AU1** | Where exactly is the `"AB"` magic of an AEMS bank checked? It is **not** checked in `cSC4AudioAEMSSoundBank::LoadFromFile` (`0x004E2490`) nor in `SNDAEMSI_addmodulebank` (`0x00867800`). The header *is* structurally validated by the version byte at `+0x02` (§10.7). | decompile `FUN_0086B340` (`0x0086B340`) / `_SNDbankadd` (`0x0086B35B`), which handle the SNDBANK blob at `bank+0x1C` |
| ~~AU2~~ | **ANSWERED** (§8.4): a single process-global counter at `0x00B098C0`, pre-increment, never returns 0, wraps through 0 back to 1. | — |
| ~~AU3~~ | **ANSWERED** (§7.2): `Init` fetches four services, registers with `cIGZSndSys` and subscribes 15 types (2 modal + 13 main); a further 11 are held only while a city is loaded. `Shutdown` undoes all but the city set. | — |
| ~~AU4~~ | **ANSWERED** (§7.3): `0xEA451903`, `0x6A451908` and `0xCA45190F` are **sound-system event ids**, not message types; they are dispatched by `cSC4AudioSoundManager::DoEvent` (§8.3). `field8_0x14` is `cIGZSndSys` at object `+0x14`, not a Lua server. | — |
| **AU5** | What is the role of the 83 leaf classes of §11? They are now measured in shape, size and IIDs; what they *do* is not. | a lookup of their 83 CLSIDs in the tuning data |
| **AU6** | What does the pool's `0x40000045` threshold mean? | the pool vtable slot 2 implementation (the initial value written to `page+0x0C`) |
| ~~AU7~~ | **ANSWERED** (§10.11): `0x9D796DB4` has no hard-coded reader; it is bound to the `tlo` extension at runtime by `cHitIniFile::ProcessDBSegmentDirectoryFilesEntry` `0x00936707`. | — |
| **AU8** | Hit tracks, the radio and the freshness score are named here but not specified. | a later spec (INVENTORY-M6 §11 item 11 covers them) |
| **AU9** | What does the `0xFF000000` tag in `cGZSndMgr::GetSound` (`0x00968AE7`) mean? | a survey of the sound-id values the game uses |
| **AU10** | What is the CLSID `0x00436EB4` registered for type `0x2026960B` at `0x00457EDF`? The factory implementation is pinned (`0x00AD8B28` → `FUN_0098419A` → `cRZStringResource`) but the id is not in `re/class_id_table.tsv`. | resolve it through the live GZCOM class registry |
| **AU11** | The `.tlo` entry layout is variable-stride (32–40 bytes). | the extension-dispatch target registered at `0x00936707`, or a trace of a TKDT field-4 reference |
| **AU12** | Property-key *names* for the audio exemplar keys of §10.8 are unrecovered (the reader takes raw 32-bit ids). | the shipped property-name table |

---

## 20.4 Scape position enumeration and convenience insertion (AU42 partial)

This settles primary slots 11 and 13 from [supplement §20.1](audio-owner-contracts.md#201-scape-construction-and-slots).
AU42 remains open for resource loading, the other instance/group operations, smoothing,
timers and allocation failure. These two entry points do not establish complete scape coverage.

Primary slot 11 (`0x004F57B0`) takes `(groupKey, positions, countInOut, metadataOut)`.
It holds the manager's group lock (`base+0x44`) while looking up the first matching group
in stored list order and copying outputs, and releases the lock on every normal return.
Missing group returns false without changing either output word or the positions buffer.
For an existing group, compare its stored count (`group+0x20`) against the caller's input
capacity **as signed 32-bit integers** (`0x004F57F1`–`0x004F57F6`). If count exceeds
capacity, return false with all outputs untouched. In particular, insufficient capacity
does not return a required size and does not copy a prefix. There is no null-buffer sizing
mode and no clamping of a negative capacity to zero. *[quirk]*

On success, overwrite countInOut with the stored count, copy the raw word at group `+0x30`
to metadataOut, then copy each instance's first three 32-bit words to consecutive 12-byte
positions records in instance-list order. The raw copies preserve floating-point bit patterns,
including NaNs; there is no coordinate conversion. Stop at either the list end or the stored
count, return true, and leave the rest of the buffer untouched. An inconsistent short list
still reports the group's stored count, rather than the number actually copied. Count zero
performs both word writes but no position write (`0x004F5807`–`0x004F584D`). Null count or
metadata output is not accepted on a successful path; a null positions buffer is harmless
only when no position is copied. No service/initialized/enabled gate is tested here.
The resource meaning of the raw metadata word remains part of AU42's resource contract.

Primary slot 13 (`0x004F51E0`) is a four-argument convenience entry. With arguments
`(A,B,C,D)`, it dynamically dispatches the same object's **primary slot 12**, passing
`(A,0,0,B,C,D)` in that order. It adds no checks, locking, output clearing, ownership or
return conversion; the return register is the slot 12 result. This is a dispatch contract,
not an independent instance insertion algorithm. Fire-handler calls to slot 13 therefore
inherit slot 12's full contract once the remaining AU42 work settles it.

### Scape capacity conformance vectors

| ID | Input/action | Required result |
|---|---|---|
| AU42-cap-01 | Missing key; sentinel count, metadata and buffer | False, all sentinels unchanged; group lock acquired/released. |
| AU42-cap-02 | Existing count 3; capacity 2 | False; capacity remains 2, metadata and buffer unchanged; no prefix copied. |
| AU42-cap-03 | Existing count 0; capacity −1, then capacity 0 | First false/outputs unchanged. Second true, count 0 and raw metadata written; null positions allowed because no copy occurs. |
| AU42-cap-04 | Count 2; capacity 4; two positions include NaN bit patterns | True; count becomes 2, raw metadata copied, first two records copied bit-for-bit in list order, remaining records untouched. |
| AU42-cap-05 | Host fixture count 3 but list contains one instance; capacity 3 | True; count remains reported as 3, exactly one record copied; no count repair. |
| AU42-cap-06 | Override slot 12 with a recording host; call slot 13 `(A,B,C,D)` | One call `(A,0,0,B,C,D)`; same return register and no other owner work. |

## 19.2 SEM lifetime and model parameter contract (AU41)

This completes [supplement §19](audio-owner-contracts.md#19-sem-registration-and-the-three-emanation-owners).
The instruction audit, rather than the inferred names or numeric pseudocode, defines the rules
below. There is no model message handler, PCM resource reader or drawing operation.

### Construction, binding and registration

The 32-byte SEM factory `0x004ED700` returns null on failed allocation; otherwise its reference
count and all six retained pointers start zero. The three 40-byte model constructors and identity
strings are §3.4. All three have reference count zero at `+8`, an embedded name/string interface
at `+0x0C`, ID at `+0x20`, and a null retained dependency at `+0x24`. QI rejects unknown IDs
without overwriting the output. Primary slot 3 reads the ID. **Primary slot 4 is not a no-op:**
it calls the supplied object's slot 14 (`+0x38`) with the model's embedded name interface
(`base+0x0C`), ignores that call's result, and returns true (`0x004EDA60`–`0x004EDA73`).

The lifecycle view is `base+4`, with Init `0x004ED180` and Shutdown `0x004ED1F0` in slots
3 and 4. Init gets the SC4 director, reads the pointer at director base `+0x60`, and retains
it at model base `+0x24`; replacing it acquires the new reference before releasing the old,
while assigning the same pointer makes no reference calls. It returns whether the resulting
pointer is non-null. Shutdown clears/releases that pointer if present and returns true. The
literal addresses matter: neither the folded `SFX::AmbienceManager` name nor the folded
`GetInstanceID` name establishes the dependency's meaning. Model transformations read the
**raw 32-bit word at dependency+0x14**, or zero for a null dependency; there is no float
conversion of that word (`0x0040CE70`). Destruction releases this dependency and the name
storage; it does not unregister a model from Hit.

Startup `0x004EDAA0` has the following observable stages. Acquiring Hit uses framework
GetService with service `0x1B2CE671`, IID `0xBB1BD9A8`. Missing framework/service returns
false. Otherwise SEM retains Hit, then saves the old 2D model from Hit slot 22
(`+0x58`, ID `0x7DCDED2D`), then the old 3D model with ID `0x7DCDED3D`. A missing old 2D
stops before the 3D lookup; a missing old 3D stops before construction. Saved model assignment
acquires the new non-null pointer before releasing a different previous pointer.

Only after both old models exist does SEM allocate, retain, query lifecycle IID `0x89E51472`
and initialize replacements, in order 2D, 3D, 3DEffects. Each QI/Init must succeed before
the next allocation. A temporary lifecycle view is released before the next QI and on exit.
After all three succeed, Hit receives exactly this sequence: register effects via slot 19
(`+0x4C`); remove 2D ID via slot 20 (`+0x50`); register new 2D; remove 3D ID;
register new 3D. These registration/removal results do not affect the true return.
There is no initialized latch and no rollback: a failed lookup, QI or model Init returns false
while retaining Hit, saved originals and any replacements already assigned. A failed model
allocation assigns null and then dereferences it for QI; it is a native invalid access,
not a clean false result (`0x004EDB73`–`0x004EDC7D`). *[quirk]*

Teardown `0x004ED750` (misnamed Init) unconditionally uses retained Hit: remove new 2D,
register saved 2D, remove new 3D, register saved 3D, remove effects. It then visits retained
replacement 2D, 3D, effects in that order: QI lifecycle, call Shutdown only on QI success,
clear/release the replacement. It balances each temporary lifecycle reference, then clears/
releases saved 2D, saved 3D and Hit, and returns true. It neither checks registration results
nor skips a null saved original when passing it to Hit. A fresh or twice-torn-down SEM has
null Hit and faults at the first use. Destruction alone releases Hit, effects, saved 3D,
new 3D, saved 2D, new 2D, in that order, with no registration or lifecycle calls.

The director calls startup after SoundManager Init and Listener Init
(`0x004EC1F5`); PreAppShutdown calls teardown after the two handlers and before Listener
Shutdown, then clears/releases SEM (`0x004EC339`–`0x004EC34A`). Thus listener binding is
available during model initialization and remains available during model shutdown. This
also agrees with §11's director lifetime; teardown must not be called as startup.

### Parameter IO and integer volume

Slots 5 and 6 both take **source S, destination D**. A parameter read is S slot 4
(`+0x10`) with one ID argument. A write is receiver slot 3 (`+0x0C`) with
`(ID, value, flag)`. In particular, the `1` pushed before a copied read is the later
write's flag, not a getter fallback argument. No parameter-presence checks occur.
Copied values keep their full 32 bits. Writes are attempted even if earlier writes fail;
there is no rollback. Null S/D is an invalid access.

For the following arithmetic, each integer product and sum wraps to 32 bits. Interpret a
wrapped product as signed for truncation toward zero by 1024. Let V be that quotient of
S parameter `0x14` times S parameter `0x15`. Let M be the raw dependency word described
above. Ordinary 2D/3D volume is the low 32 bits of M×V, **logically** shifted right ten
bits. Thus it lies in `0..0x003FFFFF`, with no clamp to 1024; a negative product is not
an arithmetic shift. The signed divide uses sign bias 1023 then arithmetic shift
(`0x004ED239`–`0x004ED261`, `0x004ED39F`–`0x004ED3C5`). Effects applies one additional
signed, truncating division by 1024 to the wrapped product V×attenuation before the
same final M multiplication/logical shift (`0x004ED638`–`0x004ED672`).

| Model slot | Parameter effects, in order | Return |
|---|---|---|
| 2D slot 5, `0x004ED210` | Read S `0x16`, `0x15`, `0x14`; compute volume; write S `0x0C=volume`, flag 1; D `0x16=saved S pan`, flag 1; D `0x15=volume`, flag 0. | true |
| 3D slot 5, `0x004ED2A0` | Copy S→D `0x20,0x21,0x22,0x40,0x41,0x42`, flag 1; read radius S `0x38`; write scaled radii D `0x1E,0x1F`, flag 1 (below); read S `0x15,0x14`; write S `0x0C=volume`, flag 1; D `0x15=volume`, flag 0. | Leaves final setter's return register; no explicit true/false assignment. |
| Effects slot 5, `0x004ED3F0` | Same six position copies; read S `0x38`, then S `0x20,0x21,0x22` again for projection. Without a view, return false after those copies/reads. With a view, compute attenuation/pan below; write D `0x16=pan`, flag 1; read S `0x15,0x14`; write S `0x0C=volume`, flag 1; D `0x15=volume`, flag 0. | Without view false; otherwise leaves final setter's return register. |
| 2D slot 6, `0x004ECDA0` | Copy S→D `0x13,0x19,0x1A,0x1B,0x23,0x24,0x25,0x26,0x33`, all flag 1. | true |
| 3D slot 6, `0x004ECE70` | Set D `0x1C=1`, flag 1; copy S→D `0x1D,0x13,0x19,0x1A,0x1B,0x23,0x24,0x25,0x26,0x33,0x1E,0x1F`, all flag 1. | true |
| Effects slot 6, `0x004ECF90` | As 3D slot 6, except D `0x1C=0`. | true |

### Radius, projection and pan arithmetic

Numeric constants are the binary32 values at `0x00A823A4` (0.01, bits `0x3C23D70A`),
`0x00A94034` (π, `0x40490FDB`), `0x00A9402C` (π/2, `0x3FC90FDB`), plus exact
0.5, 16, 512, 1024 and 1/1024. Arithmetic uses x87 intermediates; store to binary32
only at the specified boundaries. Float-to-integer conversion uses `_ftol2` truncation
toward zero; do not substitute nearest rounding. Exceptional x87 inputs follow the
native masked-invalid conversion behavior, with no validation or fallback.

For ordinary 3D, convert radius `0x38` as **unsigned u32** to x87, multiply by binary32
0.01, then multiply by global zoom scale `0x00B21A30`, store binary32 R, and write
D `0x1E=trunc(R)` and `0x1F=trunc(R×16)`. There is no binary32 store between the first
two multiplications (`0x004ED33B`–`0x004ED388`). The global starts zero in the file image;
Listener `_CalculateZoomValues` sets it to its fifth zoom scale (§18.4,
`0x004F18C0`–`0x004F18CF`). It is not the current zoom index.

Effects instead stores binary32 r after unsigned radius×0.01. S position `0x20,0x21,0x22`
is converted **as signed i32** to three binary32 coordinates. The global view at
`0x00B43DD8` projects that vector in place through
`SC4ViewUtilities::ProjectPointToViewportCoordinates2D` `0x007F78E0`; projected coordinates
are host/view IO, not a new projection formula in this owner. Read width W and height H
from view `+0x28/+0x2C`; truncate projected x/y to integers X/Y. Set dx=X−trunc(W/2),
dy=Y−trunc(H/2), with wrapped integer subtraction. Store binary32 distance after
sqrt(signed interpretation of wrapped dx²+dy²). Store binary32 radius limit after
signed interpretation of wrapped (W+H), multiplied by 0.5 and r. Attenuation is
trunc(1024) if distance≤limit (also on unordered comparison), otherwise
trunc(1024/(distance/limit)). Zero/negative limits receive no special handling
(`0x004ED504`–`0x004ED5A2`). There is no effects write to `0x1E/0x1F` in slot 5.

Pan uses **absolute X**, not dx and not X/W. Form t=X×binary32 π×(1/1024) in x87;
save binary32 t but take sine of the still-unrounded x87 value. Form p=512×sin(t).
Compare the saved binary32 t against binary32 π/2: if strictly greater, replace p
with 1024−p; otherwise keep p (unordered also keeps it). Truncate, then clamp the
signed integer to `[0,1024]` (`0x004ED5A7`–`0x004ED621`). Consequently pixel pan is
not normalized to viewport width and is periodic beyond the intended span. *[quirk]*

### AU41 conformance vectors

Use a parameter host that records ordered reads/writes and write flags. Supply M as a raw
word, and supply the projection result and view dimensions independently of the model.

| ID | Input/action | Required result |
|---|---|---|
| AU41-01 | Construct SEM/models; reject an unrelated QI; invoke model primary slot 4 | Zero counts/null dependencies; IDs/names as §3.4; failed QI preserves output; slot 4 calls host slot 14 with embedded name view and returns true even if host returns false. |
| AU41-02 | Bind model with director holder P, repeat, bind Q, Shutdown twice | First AddRef P; repeat no count calls; AddRef Q before Release P; first Shutdown clears/releases Q; both Shutdown calls true. |
| AU41-03 | Startup: no Hit; then Hit but no old 2D; then old 2D but no old 3D | False each time; respectively no retained change, retain Hit, retain Hit and old 2D. No model construction or Hit register/remove. |
| AU41-04 | All prerequisites; record successful registration then teardown | Startup: register effects, remove/register 2D, remove/register 3D. Teardown: remove/restore 2D, remove/restore 3D, remove effects, model lifecycle shutdown/release in 2D/3D/effects order, release saved originals/Hit. Ignore Hit result failures. |
| AU41-05 | Fail 3D lifecycle Init; separately fail first model allocation | First false, retains originals/Hit and first two replacements, no Hit mutation; second invalid access during QI, not false. Fresh teardown similarly faults on null Hit. |
| AU41-06 | 2D: S14=1024, S15=512, S16=77, M=1024 | S0C=512 flag 1, D16=77 flag 1, D15=512 flag 0; return true. No geometry copy. |
| AU41-07 | S14=−1025, S15=1, M=1024; also S14=−1, S15=1 | First V=−1, final volume=4194303; second V=0 and volume=0. Neither case clamps to 1024. |
| AU41-08 | 3D: S38=100, zoom scale=2; ordinary 2D volume inputs | Six position copies precede D1E=2, D1F=32: the binary32 R store rounds the product to exactly 2 before integer truncation. Volume writes as AU41-06; no D16 write. |
| AU41-09 | Slot 6, distinct values for every source parameter | Exact ordered copy lists and flag 1 from table; only 3D/effects write D1C, respectively 1/0; all return true. |
| AU41-10 | Effects without a global view | Six destination copies still occur; S38 and second position reads occur; false, with no pan, radius or volume writes. |
| AU41-11 | Effects: W=H=1024, projection (512,512), S38=100, M=1024, S14=S15=1024 | Center distance zero gives attenuation 1024; pan 512, S0C and D15=1024. D1E/D1F untouched. |
| AU41-12 | Same center X/Y, but W=H=2048, S38=0 | Positive distance/zero limit gives attenuation zero and volume zero; pan still 512, demonstrating absolute-pixel pan. |

## 18.4 Listener geometry, tuning and failure contract (AU40)

This completes [owner supplement §§18.1–18.3](audio-owner-contracts.md#18-listener-camera-inputs-and-position-notifications).
All coordinates and tuning elements are f32. Update (`0x004F14B0`) returns false immediately
if the global renderer reference is absent; with a renderer but no camera it returns true
without work. Otherwise terrain, city, sound-system and tuning prerequisites are unchecked.
Missing prerequisites are native invalid accesses, not successful updates or false returns.
The zoom index is a signed 32-bit value and is used directly for table indexing; there is no clamp.

Let C be camera position (`+0x3C`), T its look target (`+0x48`), U its orientation vector
(`+0x6C`), and z its zoom index (`+0x0C`). For z < 2, obtain two integer dimensions via the
city interface `+0x178(&width,&depth)`, with the renderer's `+0x174` and `+0x48` conversion
calls following it. These foreign calls supply the dimensions; use half width/depth as T.x/T.z;
T.y remains the camera target height. This is the low-zoom city-centering rule, not half of
the original target coordinates (`0x004F1524`–`0x004F1577`).

The listener geometry is defined by these vectors, in this order:

| Quantity | Rule / observable |
|---|---|
| Initial direction F | Normalize T − C; right vector R = F × U. R is not normalized here. |
| Sea-plane target | Read terrain `+0x74` (sea level S). Only if T.y < S, replace T by the ray/plane intersection from C along F with plane `(0,1,0,−S)`. The helper result is ignored; F and R are not recomputed. |
| Height H | Process-global zoom height at index z, calculated as supplement §18.3 specifies. |
| Position L, member `+0x0C` | `(T.x, T.y + H, T.z)`; H is a vertical offset. |
| Auxiliary position, member `+0x18` | T − H × F. Products for x/y are rounded to f32 before subtraction; z's product stays in x87 until subtraction. |
| Target, member `+0x24` | Copy of the possibly sea-adjusted T. |
| Final direction D | Normalize T − L. |
| Final up V | Negate Normalize(R × D). |
| Foreign sound calls | Get sound-system listener with `+0x0C`; call returned listener `+0x24(D.x,D.y,D.z,V.x,V.y,V.z)`. Get it again; call `+0x1C(L.x,L.y,L.z)`. No acquired reference or null gate is added around these two gets. |

Numeric audit: `0x004F157B`–`0x004F1617`, `0x004F1669`–`0x004F17DC` use x87
arithmetic. Each normalization stores its reciprocal length as f32, then multiplies components
by that stored reciprocal. In the first and final-direction normalization, x/y differences
remain extended until multiplication; the z difference is first stored as f32. Sum order is
z² + y² + x² for the first direction, x² + z² + y² for the final direction. Cross R's
components are stored as f32. The final-up cross x/y remain extended, z is stored f32;
its length sum is x² + z² + y². Normalized components are stored f32 (the final-up y/z
products stay extended until their negated f32 stores). There is no zero-length test: zero
height, coincident C/T or parallel vectors may propagate infinities/NaNs with ordinary masked
x87 exceptions. No fallback direction or epsilon is substituted. *[quirk]*

The sea-plane helper (`0x005D7360`) treats absolute denominator < f32(0.01) as parallel.
If the absolute plane distance is also < f32(0.01), it writes C and returns true;
otherwise it writes zero and returns false. For a nonparallel ray it writes C + tF with
t = −planeDistance / stored-f32 denominator, without a t >= 0 gate. The caller uses that
output even on false. Its strict T.y < S test leaves equality and unordered values alone.

Notification movement is measured between C and saved camera member `+0x30`, not between L
and its prior value (`0x004F1826`–`0x004F185B`). Strictly greater squared movement than
the current zoom threshold copies C into that member before allocating the message. Equality
or unordered comparison suppresses notification. Changes to T, H or orientation can therefore
change L without sending a message. The payload still points to L. The standard-message path
allocates 44 bytes, AddRefs, sets type, Pointer1 and payload as supplement §18.3, calls server
`+0x10(message,0)`, Releases, and returns true regardless of delivery result. A null allocation
reaches a null virtual call after saving C. There is no false-return allocation recovery.

Tuning (`0x004F1A80`) acquires the global resource manager and requests
TGI `(0x6534284A,0xCA25875D,0x6A359C5C)`, IID `0xA52160F5`, flags 0,0.
A failed resource lookup leaves all tuning unchanged and releases the resource/manager
references. A missing resource manager is an unchecked dereference. With the resource present,
read its property holder (`+0x1C`) and apply the following in order:

| Property | Destination / missing-property rule |
|---|---|
| `0x2A36E3CB` | Scalar `+0x70`, retains previous value (constructor 4). If sound system exists, get `+0x10`, AddRef, call returned object `+0x28(scalar)`, Release. A null returned object is unchecked. |
| `0xCA359F17` | Height array `+0x3C`; missing writes bits `[44A70000,4426C000,43A68000,43110000,42920000]` directly into its first five entries, without resizing. |
| `0x6A359F22` | Multiplier array `+0x48`; missing writes five 1.0 values directly, without resizing. |
| `0x2A35A25F` | Scalar `+0x6C`, retains previous value (constructor 1). |
| `0xEA4A7C55` | Array `+0x54`; missing resizes to five then assigns `[1,0.75,0.5,0.25,0]`. |
| `0x2AA40CCC` | Movement array `+0x60`; missing resizes to five then assigns `[26,13,6.5,3.25,1.625]`. Regardless of lookup success, square each of its first five entries and multiply by 256, storing f32 only after both multiplies. |

The vector property helper (`0x005FFCC0`) resizes successful reads to the actual property count
and copies the f32 data; it neither pads to five nor rejects short arrays. Consequently first-use
missing height/multiplier properties write through empty storage; successful arrays shorter
than five also permit out-of-bounds access in calculation/update. The fallback for the last two
arrays alone guarantees five elements. There is no global "all tuning missing" fallback when
the exemplar itself is unavailable. Repeated reads of a missing movement property reset before
squaring; a fresh successful property copy is squared each time. *[quirk]*

The city helper requires payload QI `0x65297976` even though no payload values are used. Null
message/missing view returns false. City init acquires the global sound system; if absent it
releases the payload and returns false before camera subscriptions or tuning. Otherwise it
subscribes the two camera types, reads tuning, computes heights and updates. City shutdown
unsubscribes those types and clears/releases sound system. It does not reset vectors or tuning.
The outer dispatcher still returns true for either recognized city message. Init/Shutdown and
the deliberately empty cheat helper are specified in supplement §§18.1–18.2.

### AU40 conformance vectors

| ID | Input/action | Expected |
|---|---|---|
| AU40-01 | z=2, C=(0,10,0), T=(0,0,0), U=(0,0,1), sea=-1, H=2 | L=(0,2,0), auxiliary=(0,2,0), target=(0,0,0); orientation D=(0,−1,0), V=(0,0,−1), ignoring signed-zero differences. Orientation call precedes position call. |
| AU40-02 | Same, sea=4 | Target becomes (0,4,0), L=(0,6,0), auxiliary=(0,6,0); initial F/R retained. |
| AU40-03 | z=0, host renderer writes dimensions 200,100, camera target initially (9,3,7) | Geometry uses T=(100,3,50) before sea adjustment. |
| AU40-04 | Saved C=(0,0,0), C=(3,4,0), threshold=25 | No message; threshold below 25 sends. Changing only H never triggers movement. |
| AU40-05 | Movement triggers; message delivery returns false | Saved C already changed; message AddRef/fill/deliver(message,0)/Release still occurs; update returns true. Null allocation instead reaches invalid access and does not return false. |
| AU40-06 | Missing movement property with any old contents | Size five, squared thresholds `[173056,43264,10816,2704,676]`. Repeating read gives same values. |
| AU40-07 | Present movement array `[1,2,3,4,5]` | Thresholds `[256,1024,2304,4096,6400]`. Present length three is not padded or rejected; five-entry transformation is invalid access. |
| AU40-08 | Fresh constructor; present exemplar missing height property | No allocation/resizing before five fallback writes; native invalid access. Exemplar lookup failure instead leaves empty arrays unchanged. |
| AU40-09 | Init without message server; cheat with/without payload QI | Init faults before initialized becomes true. Cheat returns true after releasing successful QI, false otherwise; no command parsing or state mutation. |
| AU40-10 | H=0, or C=T, or R parallel to final D | No normalization fallback; NaNs may reach orientation. Position writes and movement gate still follow normal ordering. |

## 16. Conformance tests

Each test names the behaviour, the address that produces it, and the observable. A replacement passes when
every *[compat]* assertion holds. 42 test groups. Groups 26–42 specify the AU13–AU16 owner closure cases;
their foreign services are recording fixtures, so they do not require an audio device or game session.

1. **Sound-event object size and allocation.** Allocate a `cGZSndEventStd`. The allocation must come from
   the pool when and only when the requested size is `0x38`; any other size goes to the ordinary heap.
   (`0x0096BAD1`)
2. **Sound-event message type and both constructors.** The five-argument constructor must stamp type
   `0xDB53383B` with presence bit 0 set and store its payload at `+0x30`/`+0x34`; the default constructor
   must leave `+0x30` and `+0x34` at 0 and still stamp the same type. (`0x0096BA6D`, `0x0096BB0E`)
3. **The sound-event payload interface.** `QueryInterface(0x5B523BA4)` must return `this+0x2C` and bump the
   refcount, and any other IID must fall through to `cRZMessage2Standard::QueryInterface`. On that interface
   `SetValue(v)` must write `v` to the outer `+0x30` and set the outer `+0x34` byte to 1; `GetValue` must
   return `+0x30`; `GetFlag` must return `+0x34`; `GetOuter` must return `this - 0x2C`.
   (`0x0096BA2B`, `0x0096BA02`, `0x009D7E63`, `0x00891D96`, `0x0096BA27`)
4. **Message presence flags, and type 0 is "no type".** After `SetType(t)` with `t != 0`, `SetData1`,
   `SetData2` and `SetVoid1(p)` with `p != 0`, the byte at `+0x08` must be `0x87` and the four values must
   sit at `+0x0C`, `+0x10`, `+0x14` and `+0x24`. `SetType(0)` must store 0 at `+0x0C` and **clear** bit 0.
   (`0x009132DB`, `0x00913322`, `0x00913330`, `0x0091346B`)
5. **The pool free-list shape.** Freeing two sound events A then B must make the next two allocations return
   B then A (LIFO, links in the objects first dword); a free must recover its page as `p & 0xFFFFC000`, so
   two pointers in the same 16 KB window share a free list; and `operator delete(0)` must be a no-op, not a
   crash. (`0x005E1780`, `0x005E16E0`)
6. **The pool page state word and its two thresholds.** Taking a page into service must add `0x40000000` to
   `page+0x0C`; allocating must increment it and releasing must decrement it. A release whose post-`dec`
   value is 0 must hand the page back through pool vtable slot 3; one whose value is exactly `0x40000045`
   must clear the `0x40000000` marker and unlink the page from the active ring instead.
   (`0x0090CF9A`, `0x005E16E0`, `0x005E1780`, `0x0090D031`, `0x0090CFF9`)
7. **`StartSFX` with id 0.** Must return 0, allocate no message, allocate no instance id and leave
   `automaton+0x8C` unchanged. (`0x00509950`)
8. **`StartSFX` message shape.** With a non-zero id it must allocate a fresh instance id into `+0x8C`, post
   one message with type `0x8A385DD8`, data1 = the id, data2 = the new instance id and void1 = the
   automaton occupant (automaton slot 5), at **normal priority** (the second argument of the post is 0).
   (`0x00509950`)
9. **`StopSFX` guard and shape.** With `automaton+0x8C == 0` it must post nothing and still return 1.
   Otherwise it must post one message with type `0xAA396960`, data1 = the argument it was called with,
   data2 = `automaton+0x8C` and void1 = the occupant, and must **not** clear `automaton+0x8C`.
   (`0x00505B50`)
10. **Both posts are queued.** `StartSFX` and `StopSFX` must return before the sound starts or stops; the
    message-server call is `MessagePost`, not `MessageSend`. (`0x005099EE`, `0x00505BB9`)
11. **State entry and exit drive the slots, and id 0 makes no sound.** `HandleEnterState` must call slot 61
    (`+0xF4`) with `state+0x18` only when that field is non-zero; `HandleExitState` must call slot 62
    (`+0xF8`) with the same field under the same guard. A state whose `+0x18` is 0 must never reach either.
    (`0x00511450`, `0x0050E3F0`)
12. **Handler routing, and only while a city is loaded.** `cSC4AudioEventHandler::DoMessage` must route
    `0x8A385DD8`, `0xA9E3B6E3`, `0xAA396960`, `0xEA3ADA33` and `0x49E64801` to
    `HandleAutomataMessages`. The fifth type is accepted with return 1 and no operation; the four
    operation cases in older fixtures remain valid, but are not an exhaustive dispatch test (AU45).
    Use §7.2's complete subscription table for city entry/exit rather than deriving subscriptions
    from the four operation cases.
    (`0x004EF810`, `0x004EE970`)
13. **Rate limiting.** On a play whose payload slot 9 is `0xA9F14C47`, if the handler timer at `+0x30` is
    running and the elapsed time is below the threshold at `+0x48`, the play must be dropped. Otherwise the
    timer restarts and the threshold becomes `1000 + rand()*3000/0x7FFF`, i.e. 1000-4000 ms. Two different
    automata sharing one handler share one timer and one threshold. (`0x004EE560`)
14. **The 83 leaf classes: shapes, sizes and capability slots.** 16 of their factories must allocate **8
    bytes**, 31 must allocate **16 bytes** and 36 must allocate **20 bytes**, writing the vtable and leaving
    the remaining fields zeroed. Slots 3 and 4 of every one of the 83 leaf vtables must return 1.
    (`0x004EC3D0`, `0x004EB260`, `0x004EB800`, `0x00648F00`; section 11.1)
15. **The 83 leaf classes: `QueryInterface` and `Release`.** For both IIDs of the family (`0x4A40074C` plus
    `0xCA400753` / `0xEA40074F` / `0xCA400757`), `QueryInterface` must write the object own pointer —
    **no sub-object offset** — call `AddRef` and return 1; every other IID must return 0 without touching the
    out-parameter. `Release` must decrement while the count is > 1 and, at 1, free the object and return 0.
    The refcount lives at `+0x0C` (16-byte family), `+0x04` (8-byte) or `+0x10` (20-byte).
    (`0x004EC5D0`, `0x004EC6C0`, `0x004EBA70`, `0x004EC5B0`, `0x004EC6A0`, `0x004EC630`)
16. **Director registration order.** `cSC4AudioCOMDirector::OnStart` must add `cGZSoundCOMDirector`,
    `cGZSndSysMacCOMDirector`, `cGZToolsCOMDirector`, `cHitManCOMDirector`, `cFreshnessCOMDirector`, calling
    each one `OnStart` immediately after adding it. (`0x004EB1B0`)
17. **Nothing audio is saved.** A save/load must not restore any sound instance: after the load every
    automaton `+0x8C` is 0 and no sound is playing, including for an automaton that was mid-state.
    (section 12)
18. **Sound data is looked up by type and extension, not by a constant.** A `0x2026960B` record must resolve
    to `.str` in the HitLists segment, `.hit` in the TLO segment and raw sample data in the
    `[SampleDataSegments]` groups, exactly as the configuration record `[DBSegmentDirectoryFiles]` says;
    `0x9D796DB4` must resolve to `.tlo`. (`0x00936707`)
19. **Tuning is read from one exemplar and is unclamped.** `cSC4AudioSoundManager::_ReadTuningValues` must
    read keys `0x6A946A50`, `0xAABAA0D3`, `0x0A946A57` and `0x4A946A5D` from exemplar
    `0x6534284A/0xCA25875D/0x6A359C5C` and apply no clamping, defaulting to 3, 10 and 30 when the read fails
    (and logging the failure). (`0x004F9250`)
20. **The instance-id counter is global and never issues 0.** `GenerateNewInstanceId` must return 1, 2, 3
    and so on from a single process-wide counter, must return the pre-increment value, and must skip 0 even
    after a 32-bit wrap. (`0x004F7BB0`, counter at `0x00B098C0`)
21. **The sound manager does not see the automata messages.** Delivering `0x8A385DD8` or `0xAA396960`
    directly to `cSC4AudioSoundManager::DoMessage` must do nothing. Those types are handled by
    `cSC4AudioEventHandler::HandleAutomataMessages` and reach the manager only as sound-system events
    `0xEA451903` / `0x6A451908`. Its `DoMessage` must handle exactly the eight types of section 8.3.
    (`0x004F9430`, `0x004FA590`)
22. **The manager instance table.** `_StartSound` with an instance id that is already live must first stop
    the old one and then start the new one under the same id. `_StopSound` with an id that is not present
    must return false and leave the hashtable and the list unchanged. (`0x004F9D70`, `0x004F94C0`)
23. **An effect sound component attaches, loops and stops.** `Start` must post `0xEA451903` with the
    description `+0x10` sound id and `this` as the instance key; `Update` must post `0xCA45190F` with
    **integer** coordinates once per `desc+0x14` seconds and call `Stop(1)` once `elapsed > desc+0x18` when
    that length is non-zero; `Stop` must post `0x6A451908`, must ignore its `mode` argument, and must clear
    `+0x1C` before checking whether the sound system is present.
    (`0x005BE430`, `0x005BE620`, `0x005BE4A0`)
24. **A sound `IsActive` and the ignore bit.** `IsActive` must return the `+0x1C` started byte and must stay
    true until `Stop`, so a non-ignored sound with `length == 0` pins its effect forever. Component record
    `+0x10` bit 0 must become runtime flag bit 2 (`+0x40 |= 4`) and `CheckFinished` must skip components
    with that bit set. (`0x00580B50`, `0x00591730`, `0x005BFD70`)
25. **The full play/stop chain.** From `HandleEnterState` to `_StartSound`, one request must produce: a fresh
    instance id in `automaton+0x8C`, one queued `0x8A385DD8`, one `0xEA451903` sound event after the handler
    rate-limit gate and position lookup, and one `tSound` in the manager hashtable keyed by that id. The
    stop chain must be identical with `0xAA396960` and `0x6A451908` into `_StopSound`. Nothing in the chain
    may touch the city save. (section 8.7)
26. **Construction and partial Init.** With both constructor lookups failing, expect null radio/sound,
    null AudioScape, flag/refcount/window 0, empty modal list, timer selector 4 and stopped. Missing Init
    sound service: false, zero registrations. Sound present but AudioScape missing: false, exactly one
    registration and no unregister; retry duplicates that registration. Failed registration return must
    not stop the next stage. (`0x004EF6F0`, `0x004EFE80`; §7.1.1)
27. **Init repeat, ignored subscription failures and Shutdown retry.** With valid integration services,
    make every subscribe return false: still perform both modal and all 13 main subscriptions and return
    true. Second Init performs nothing. Missing Shutdown sound service: false, flag/services intact;
    restore it and expect unregister, AudioScape release, radio slot 23(false), radio release/null, 13
    main unsubscribes and modal pair, true. Repeated Shutdown does nothing. Re-Init must not reacquire
    radio; city subscriptions, modal stack and timer survive Shutdown. (§7.1.1)
28. **Construction sound selection.** Data1 91, data3 0: non-positional `0x89F10866` calls AudioScape
    slot 14(`0xAA62C82A`,91). Data3 2 selects `0xEA62C800`; bits other than mask 2 do not affect selection.
    Data2 absent or present zero remains non-positional. Present/nonzero data2 and position `(3,4,5)`
    use slot 13(sound,91,position,0.0f). `0xA9F10E45` uses slot 15 with the same tuple. Null QI result
    returns true/no post; null void1 on `0xA9F10E45` returns false; missing AudioScape/payload QI returns
    false; acquired references are released. Main dispatch masks those false returns to true. (§7.3.2)
29. **Deferred station selection and radio helper.** EC4 queues station data1 0, EC5 queues 1; neither
    includes data2. Deliver the queued messages and expect `kmay`/`krgn`, fade record `(0,4000)`; present
    data2 123 gives `(0,123)`. Same resolved current/requested station skips slot 13 but still calls slot
    30(false). Data1 2 is a successful no-op. Failed payload QI rejects direct helper, and no radio
    still accepts valid station messages without calls. (§7.2.1)
30. **Pointer transfer and radio controls.** `0xAA611B6E` with data1 pointer P and distinct message M:
    expect radio slot 7(P), Release(P), no AddRef(P), no Release(M); null radio still releases P, null P
    does nothing. `0x0A611B34` with data1 0/data2 44/data3 absent yields object slots 14(44),12(2);
    present data3 −7 yields 12(1). `0xAA650860` toggles only when desired differs from current, with
    slot 8(0) only on enabling. Verify preference gate on `0x4348B111` and the `0x0BAAB7A4` request
    made by `0x4A7FB7E9`. (§7.2.1)
31. **Dispatch categories and capability-only notifications.** For each category 0,1,5,6 and key 77,
    `0x894671F6` builds `(0x09C66951,77,0,0)` and `0xE94671EB` builds `(0x29D99E9E,77,0,0)` via slot 13,
    then submits/releases the returned interface. Categories 2,3,4,7 and zero key emit nothing. C9F1011A
    and 49F10111 successfully QI `0xC9B6D7D6`, release it, and emit nothing; failed QI returns false to
    the direct helper and true through main dispatch. Missing data1 presence rejects. (§7.2.2)
32. **Ignored subscribed types.** Deliver C53D10AA and E94671ED with otherwise sound-capable payloads:
    true, no effects. Direct 49E64801 reaches automata but produces no sound. Null main message returns
    false. This test uses direct delivery independently from subscription eligibility. (§7.2)
33. **Nested modal ownership.** Begin A, begin B, end notice naming A: retained windows are A then B,
    but the end releases B. Final end releases A. Subscribe mouse once on the first begin, unsubscribe
    once on the final end, never on nested transitions. Failed window QI still pushes/pops. (§7.2.3)
34. **Modal bounds and return exception.** Window rectangle `(10,20,30,40)`: points on every edge/corner
    produce no sound for each subtype 7,8,9. Points `(9,20)`, `(31,40)`, `(10,19)`, `(30,41)` each post
    `(1,0xAA62C9D5,0,0,0)` via slot 11, return true and balance references. Same outside point with
    missing global sound service returns false through main DoMessage. Other subtype or failed mouse
    QI returns true/no sound. (§7.2.4)
35. **Automata packed event and QI failure.** Sound 55, data2 77, position `(1.9,-2.9,3.1)` yields start
    event `0xEA451903` words `(55,77,1,0)`, then position event `0xCA45190F` words `(77,1,-2,3)`; stop
    emits `0x6A451908` words `(77,0,0,0)`. Failed position QI emits no event and returns false, even if
    the host supplies nonzero seed words. Verify payload/position reference releases and that slot 14
    getters receive **no arguments**. A rate-limited magic sound still returns true and performs no
    position lookup. (§7.3)
36. **Alternate automata selection.** A9E3B6E3 with both data presence flags and void1 P: data1 0 calls
    slot 14(`0x49DE8A94`,P), slot 14(`0xA9DE8B59`,P), in order. Data1 1 or −1 with position `(1,2,3)`
    calls slot 13(`0x49DE8A94` or `0xA9DE8B59`,P,position,50.0f). Independently remove either presence
    flag, remove void1, or fail positional QI: direct helper false, no posts, balanced payload/QI.
    No instance generation and no timer changes on these routes. (§7.3.1)
37. **EA3ADA33 matrix and ID source.** Exercise all six cells of the selector table plus data2 0 and
    absent data2 (both `0x69F01280`). Absent data1 selects the nonzero column. Fixture generated ID 99
    appears as the AudioScape slot-13 key, never a data2/key from the message. Verify SFX service access,
    one no-argument global generation and 50.0f range. Failed position QI continues; assert only the
    defined calls/ID, not an invented value for its indeterminate position. (§7.3.1)
38. **DoEvent return and dispatch.** Null member sound system: true, no service/QI. With it available,
    unknown type and each recognized type return false after balancing temporary references, even if
    the initial payload QI fails. AA36D9B6 invokes enumeration `(callback,0,0)`; recording callback QI
    success yields slot 4(32,0) then slot 3(32,result), release and true; failed QI still returns true.
    0BAAB7A4 with value 45 calls the obtained service interface slot 11(127,45) and releases it. (§7.5)
39. **Torch start/stop and sticky mode.** From zero globals, event 2A106113 emits exactly start/volume
    for sound `0x6A10AC51`, key 0x601, volume 1024. AA96FE8F with nonzero data1 enters mode and starts
    keys 0x600/602/603 before stopping 0x601; repeat emits nothing. A zero-data1 transition stops 602,
    stops 603, starts 601. Stop-all event 8A10611A stops only 601/602/603 and leaves mode/enabled flags
    intact. Helper's third/fifth arguments do not alter any event word. (§7.5)
40. **Torch numeric/history and enabled gate.** With previous point `(0,0)`, timestamp 0, speed 0,
    use point `(3,4)`, timestamp 100: speed approximately 5, stored smoothed speed 2.5f, V=17.
    Enabled 4A106126 posts volumes 617 on both keys 602 and 603. Next event at the same timestamp has
    speed 0, stored smoothed speed 1.25f, V=8. Disabled processing emits no volume posts but still
    updates history. Add timestamp wrap `0xFFFFFFF0`→`0x10` (elapsed 32), a speed above 60 (cap before
    averaging), and two handlers sharing the same history. No member AudioScape or no RZ sound service
    changes neither history nor sounds and returns false. (§7.5)
41. **Destruction is separate from shutdown.** Destroy without Shutdown after a begin notice: free
    timer/list storage, release remaining AudioScape/sound/radio once each, but do not unsubscribe,
    unregister, stop sounds or Release the raw stacked window. With Shutdown already successful,
    destruction must not release the already-cleared AudioScape/radio a second time. Deletion flag 0
    retains outer storage; flag 1 frees it. Global torch history survives both. (§7.6)
42. **Unchecked-input boundaries.** Null required final message servers, null void1 on ordinary play,
    empty-stack modal end/mouse, and null torch payload are outside the original's checked contract.
    A replacement may diagnose these safely *[choice]*, but must not treat them as measured successful
    requests, fabricate coordinates, or count them as the false-return cases above. (§§7.1.1–7.6)
