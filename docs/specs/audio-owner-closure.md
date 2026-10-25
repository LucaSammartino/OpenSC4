# Audio owner closure: the last bodies of twelve audio owners (behaviour spec)

Scope: the remaining unspecified functions of `cSC4AudioAEMSEventSystem`, `cSC4AudioAmbienceLayer`, `cGroupLayer`,
the three emanation models and `SFX` (§1–§5, re-audioclose8), of `tSound` and `cSourceFileDebugInfo` (§7–§8,
re-tsound8), and the ownership statement that lets `cSC4AudioListener`,
`cSC4AudioSemManager` and `cSC4AudioAEMSSoundBank` be claimed, as tools/coverage.py assigns them. Everything else
about these owners is already specified: audio.md §3.4, §10.7–§10.7.1, §18.4, §19.2; audio-owner-contracts.md
§17–§19; audio-ambience.md §8.1–§8.7. This spec adds the missing bodies and one correction (§2.3).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function below was read in the disassembly. Written 2026-10-25 (re-audioclose8).

## 1. AEMS event system *[compat]*

- **RemoveAllBanks** (`0x004E1840`, the body audio.md §10.7.1 describes without an address): while the bank list
  (`+0x138`) is not empty, take the first bank, AddRef it when non-null, unlink and free the list node, call the
  backend remove (`0x00B107E0`) with the bank's handle (`+0x20`, getter `0x004ED900`), then Release the bank twice
  (the temporary reference, then the list's). The backend result is ignored.
- **View thunks**, eight bytes each, adjusting ECX and jumping: for the views at `+4`, `+8`, `+0x0C` and `+0x10`,
  QueryInterface (`0x004E1430`), AddRef (`0x004E19C0`) and Release (`0x004E19D0`) at `0x004E19E0`–`0x004E1A00`
  (−4), `0x004E1A10`–`0x004E1A30` (−8), `0x004E1A40`–`0x004E1A60` (−0x0C), `0x004E1A70`–`0x004E1A90` (−0x10); and
  the deleting destructor (`0x004E1D80`) at `0x004E1AA0` (−0x10).

With §10.7.1 this completes the owner; question AU48 is answered there.

## 2. Ambience layer *[compat]*

### 2.1 Small members

| Function | Behaviour |
|---|---|
| `GetName` (`0x004E7650`) | returns the address of the name string `+0x188` |
| `SetName(s)` (`0x004E7660`) | the string's slot 14 assigns `s`, then `MakeLower` |
| `SetLayerId(id)` (`0x004E7690`, vtable word `0x00A934C8`) | `+0x1FC := id` (a second id word; the constructor's layer id is `+0x1F8`, audio-ambience.md §8.1.1) |
| `GetAveragePosition(out)` (`0x004E76B0`, vtable word `0x00A934D0`) | copies the three floats `+0xC8`, `+0xCC`, `+0xD0` |
| `0x004F1200` | `cGroupLayer::Clear(this, 0)`, returns true (the per-layer clear `_RemoveLayers` uses) |

### 2.2 `_RemoveLayers` (`0x004E50A0`)

In this order: the group layer `+0x30`; then `+0x2C` only when it is not the same object as `+0x28` (the shared
layer of audio-ambience.md §8.4); then `+0x28`. Each existing one is cleared (`0x004F1200`), its holder set to null,
then Released. Shutdown's removal order (audio-ambience.md §8.1.1) runs this first.

### 2.3 `_RemoveAllFilters` (`0x004E6A70`) and the list references (correction)

For each kind list (kind 1 `+0x214`, kind 2 `+0x230`, kind 3 `+0x24C`), walking from the **last** entry: AddRef the
filter when non-null, Release it, store null in the slot, Release it again; then the kind count (`+0x220`, `+0x23C`,
`+0x258`) := 0 and the list end := its begin. Each entry thus loses one reference; a null entry is dereferenced.

**Corrects** audio-ambience.md §8.3 step 4 ("no reference of its own; the cache owns it"): `_AddFilter` AddRefs the
filter once more just before appending it (`0x004E717D`–`0x004E717F`) and releases only its local reference at the
exit, so **every list entry holds one reference** and the cache holds another. The counts in §8.3 are unchanged.

### 2.4 `_StaticRemoveAllFilters` (`0x004E6C10`)

Walks the process filter cache (buckets `0x00B2195C`–`0x00B21960`) node by node: AddRef the filter, call its slot 4
(the always-true stub, audio.md §11.1), Release it; when that Release returns 0 a message "Error:
cSC4AudioAmbienceLayer::_StaticRemoveAllFilters : Failed to release filter (%08x)" is formatted with the class id and
discarded; then Release once more (the cache's reference). The hash table is then destroyed. StaticShutdown runs it
first (audio-ambience.md §8.1.1).

### 2.5 Destructor helper `0x004E6300`

Called by the destructor (`0x004E77D8`) for the score list: frees every node, releasing the object at node `+0x20`
when non-null, then resets the sentinel to empty.

With audio-ambience.md §8.1.1 (which answers AA14) this completes the owner.

## 3. `cGroupLayer` *[compat]*

- `0x004F08F0(item)` (from `0x004F0B19`, the instance removal): finds the first list node whose `+8` is `item`;
  absent → nothing. Found: `item+4 := 0`, the node is unlinked and freed, the count `+4` is decremented.
- Release (`0x004F1070`): count `+4` greater than 1 → decrement and return the new count. Otherwise restore the
  vtable `0x00A94200`, clear the list, destroy the hash table, free the list sentinel `+0x0C`, free the object
  (`GlobalHeapFree`) and return 0.

## 4. Emanation models *[compat]*

The deleting destructors `0x004EDD30` (2D), `0x004EDD80` (3D), `0x004EDDD0` (3DEffects) restore the two vtables of
audio.md §3.4, release the object at `+0x24` when non-null, reset the string base at `+0x0C` and free its buffer
`+0x10`, then free the object when flag bit 0 is set. They return `this`.

## 5. `SFX` *[compat]*

`SFX::Listener` (`0x004E8490`): `GetSC4AudioCOMDirector()`; null → null; otherwise the listener holder of the
director (base `+0x64`, the `+0x60` of audio.md §11.3's secondary view), borrowed: no reference is taken.
`SFX::SoundManager` (`0x004BB9E0`) and the SEM lifecycle entry `0x004ED180` are audio.md §7 and §19.2.

## 6. Owners already complete

- `cSC4AudioListener` (11 functions): audio-owner-contracts.md §18 and audio.md §18.4 (AU40 answered).
- `cSC4AudioSemManager` (6 functions): audio-owner-contracts.md §19 and audio.md §19.2 (AU41 answered).
- `cSC4AudioAEMSSoundBank` (4 functions): audio.md §10.7–§10.7.1.

## 7. `tSound`: position, linked volume and pan, pauses (AU50, sound side) *[compat]*

Fields as audio.md §8.3.1: `+0x08` instance ID, `+0x0C` the source holder, `+0x10` the buffer (slot 3
`SetProperty(id, value, flag)`, slot 4 `GetProperty(id)`), `+0x18` volume, `+0x1C` gain, `+0x20`–`+0x28` raw
position, `+0x2C` flags, `+0x34`/`+0x38` inner/outer radius, `+0x3C` pause count, `+0x4C` child list, `+0x50`
master, `+0x54`–`+0x5C` averaged position. Property `0x39` is the emanation model id (audio.md §3.4: `0x7DCDED2D` 2D,
`0x7DCDED3D` 3D, `0x0A63A543` 3D effects), `0x33` the loop flag, `0x15` volume.

### 7.1 Small members

| Function | Behaviour |
|---|---|
| `GetPosition(out)` (`0x004F8030`) | no children → the raw position; else the averaged one |
| `SetVolume(v)` (`0x004F9640`) | `+0x18 := v`; property `0x15 := trunc(float(v)·gain)` (flag 0); then UpdatePosition when it has children or a master |
| `UpdatePosition` (`0x004F9050`) | with children, by property `0x39`: 3D → UpdateVolPan3D, 3D effects → UpdateVolPanEffects, other ids nothing; then the position properties of audio.md §8.3.1 |
| `UpdateVolPan3D` (`0x004F8F00`) | listener (§5's `SFX::Listener` path): L := listener `+0x0C`–`+0x14`, T := `+0x24`–`+0x2C`; without a listener both are the sentinel `0x00B21AF0`; `UpdateAveragePosition(T − L, L)` (each component a float32 difference) |
| `UpdateVolPanEffects` (`0x004F8EA0`) | view utility `0x00B43DD8` (null → nothing): L := `+0x3C`–`+0x44`, T := `+0x48`–`+0x50`; `UpdateAveragePosition(T − L, L)` |

### 7.2 `UpdateAveragePosition(d, L)` (`0x004F8920`, pop 8)

1. d := d·(1/|d|) in place (x87 length `sqrt(x² + y² + z²)`, each component stored float32).
2. The instance itself is pushed at the **front** of its child list for the walk and removed afterwards, so the walk
   visits the instance and every child. Starting with weight sum W = 0, sums S = 0, the minimum distance m = FLT_MAX
   (`0x7F7FFFFF`) and the nearest position N = the sentinel, for each instance s:
   r := s's raw position − L (float32 per component); dist := sqrt(rz² + ry² + rx²); when dist < m, m := dist
   (float32) and N := s's raw position; a := float32 |(r/dist)·d| (x87, the sum ordered x, z, y);
   v := s's property `0x15` (int); w := v·(1/1024)·(1/(1 − a))·a (x87); W += w, S += position·w (each float32).
3. The average A := S·(1/W) is stored at `+0x54`–`+0x5C`.
4. With u := normalize(A − L) and c := normalize(N − L): when |u·d| − |c·d| is greater than 0 (the average lies
   closer to the view direction than the nearest instance), A moves outward along u by max(m, inner radius `+0x34`)
   − inner radius (each component stored float32).

A direction exactly on an instance (a = 1) divides by zero; an all-silent group (W = 0) divides by zero; m stays
FLT_MAX for an empty walk (not reachable: the instance itself is always visited).

### 7.3 Pauses and resume

- `ShouldPauseForDistance` (`0x004F8070`): false (AL) for a child (`+0x50` set) or for the 2D and 3D-effects
  models; otherwise EAX := 1 when the distance from the listener position (`SFX::Listener` `+0x0C`–`+0x14`, not
  null-tested) to `GetPosition` is at least the outer radius `+0x38`, else 0.
- `PauseForDistance(on)` (`0x004F97E0`): on → flag bit 3 (`8`) set, count += 1; when the count becomes 1 and there
  is no master, `EnableCompletionNotification(false)` and holder slot 8 (`+0x20`, the soft stop). Off → bit 3
  cleared, `Resume`. Returns the count.
- `StartStopSoundForDistance` (`0x004F9B90`): only for looping sounds (property `0x33` ≠ 0) that are not children and
  use the 3D model; bit 3 clear and `ShouldPauseForDistance` → `PauseForDistance(true)`; bit 3 set and not
  `ShouldPauseForDistance` → clear bit 3 and `Resume`.
- `StartStopSoundForSimulationPause` (`0x004F9C10`): bit 5 (`0x20`) marks a sound that pauses with the simulation,
  bit 4 (`0x10`) a sound paused by it. Bit 4 clear, bit 5 set and the simulator (`0x00B43D5C`) `IsPaused` (slot 15)
  → set bit 4, count += 1, and on the first pause without a master the same disable and soft stop as above. Bit 4
  set, and bit 5 clear or no simulator or not paused → clear bit 4 and `Resume`.
- `Resume` (`0x004F9750`): count −= 1; when it reaches 0 and there is no master: `EnableCompletionNotification(true)`;
  the holder queried for `0xBB1BE367` gets slot 12 (`+0x30`) when the query succeeds; buffer property `0x10 := 3`
  (flag 0); holder slot 5 `Play(instance ID, volume +0x18, 512, 0)`; `UpdatePosition`; `SetVolume(+0x18)`; the query
  reference is released. Returns the count.

The voice-count gates and the manager-side UpdateAll of AU50 belong to `cSC4AudioSoundManager` and stay open there.

## 8. `cSourceFileDebugInfo` (HIT debug info) *[compat]*

Layout: a name string at `+0`, base offset `+0x14`, size `+0x18`, and a list `+0x1C` of (line `+8`, offset `+0x0C`)
nodes.

| Function | Behaviour |
|---|---|
| constructor `(name, base, size)` (`0x0093A42E`) | string; empty list (sentinel node allocated by `0x0073DD20`); string slot 13 assigns `name`; base and size stored |
| `0x0093A130(line)` | the offset of the first node with that line, else 0 (with an empty list and line −1 it reads the sentinel's `+0x0C` *[quirk]*) |
| `OffsetFromLineNum(line)` (`0x0093A1A1`) | that offset 0 → 0; else base + offset |
| `0x0093A159(rel)` | walks the nodes keeping two sticky flags, "some offset ≤ rel" and "some offset ≥ rel"; returns the line of the first node at which both are set, else 0 |
| `LineNumFromOffset(off)` (`0x0093A1C0`) | off outside [base, base + size) → 0; else `0x0093A159(off − base)` |
| destructor body (`0x0093A462`) | erases the nodes, frees the sentinel, destroys the string |
| `0x0093A506` | the container destructor (restores the `cHitDebugInfo` vtables): destroys and frees every listed `cSourceFileDebugInfo`, then its hash table |

## 9. Conformance tests

1. Two layers listing the same filter X: after both CreateFilters, X holds three references (the cache and one per
   list entry); `_RemoveAllFilters` on one layer leaves two.
2. `_RemoveLayers` with `+0x2C == +0x28` clears and releases that layer once.
3. RemoveAllBanks with three banks calls the backend remove three times, first bank first.
4. `SFX::Listener` without the director returns null and makes no reference calls.
5. `UpdateAveragePosition` on a childless instance at the listener's look direction (a = 1) divides by zero; two
   children with volumes 1024 and 0 weight only the first.
6. A looping 3D sound beyond its outer radius pauses (count 1, one soft stop); two more pause requests raise the
   count without stops; three resumes restart it once.
7. `LineNumFromOffset(base + size)` is 0; `OffsetFromLineNum` of a missing line is 0.
