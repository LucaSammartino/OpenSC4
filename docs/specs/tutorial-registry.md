# `cSC4TutorialRegistry`: the list of tutorials per region (behaviour spec)

Scope: the `cSC4TutorialRegistry` owner as tools/coverage.py assigns it (19 functions, 4,860 bytes on `5c0ff782`;
inventory in §7). The registry is a plain C++ object embedded in the application (`cISC4App` slot 49,
tutorial.md §4.2.1). It is filled from the Lua table `tutorial_registry` and answers:
- which tutorials a region has, in order;
- which regional city a tutorial plays in;
- which tutorial of a region is due next;
- the default tutorial region of an expansion pack.

tutorial.md §4.2.1 and §7 (TU1) already give the item layout, `GetItem` and `SetActive`; feature-manager.md §6
gives the one caller of the default-region lookup. This spec adds the rest and answers FM2 of feature-manager.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled. The Lua key names are the literals at
`0x00A8FFA8`–`0x00A90014`. The shipped script `tutorial_registry` (`SimCity_1.dat`, type `0xCA63E2A3`, group
`0x4A5E8EF6`, instance `0xFF7D91E7`, 2,672 bytes once decompressed) was read for §3.3. Written 2026-10-23
(re-tutreg7).

Conventions:
- `R` is the registry.
- `L` is the Lua main thread, a `cIGZLua5Thread` (lua-host.md §6.2). The slots used are:
  - 1 `AddRef`;
  - 2 `Release`;
  - 14 `GetTop`;
  - 15 `SetTop`;
  - 19 `Pop`;
  - 26 `IsString`;
  - 29 `IsTable`;
  - 41 `ToNumber`;
  - 43 `ToString`;
  - 48 `PushNil`;
  - 51 `PushString`;
  - 57 `GetTable`;
  - 79 `Next`.
- `-10001` is the globals pseudo-index.
- A number read with `ToNumber` is cut to an integer with `_ftol2` and its low dword is kept, so GUIDs at or
  above `0x80000000` keep their bits.

## 1. Identity and fields *[compat]*

No vtable; every method is a direct `__thiscall`. The fields below span 0x1C bytes; `+0x08` and `+0x14` are never
read or written *[inferred: the maps' empty comparators]*.

| Offset | Field | Meaning |
|---|---|---|
| `+0x00` | `regions` header node | `std::map<cRZString, std::list<cItem>>` (STLport `_Rb_tree`): lowercased region directory → its tutorials, kept ascending by `order` |
| `+0x04` | `regions` count | |
| `+0x08` | — | unused |
| `+0x0C` | `defaultDirs` header node | `std::map<long, cRZString>`: expansion-pack number → default tutorial region directory |
| `+0x10` | `defaultDirs` count | |
| `+0x14` | — | unused |
| `+0x18` | `lua` | the `cISCLua` the registry reads from (counted) |

A `regions` tree node holds the key `cRZString` at `+0x10` and the list's head node pointer at `+0x24`. The list is
STLport's: a circular doubly-linked list with `_M_next` at `+0`, `_M_prev` at `+4` and no element count. A list
node is 0x30 bytes, with the `cItem` at `+8`. The `cItem` (0x28 bytes) is in tutorial.md §7 (TU1): `+0` `guid`,
`+4` `order`, `+8` `task_first`, `+0xC` `task_count`, `+0x10` `active` (byte), `+0x14` `region_dir` (a
`cRZString`; its last dword, `+0x24`, is the string's own field that every string constructor zeroes).

## 2. `SetLua(lua)` (`0x004B7D70`) and the reset (`0x004B72C0`)

`SetLua(lua)` takes one pointer and pops 4 bytes. It returns a bool in AL.
1. `lua` null, or equal to `R+0x18`: return **false** and do nothing else *[quirk]*. AL = 0; the rest of EAX is
   whatever it held.
2. Reset (`0x004B72C0`, shared with other owners):
   1. erase every `regions` node and reset the header;
   2. erase every `defaultDirs` node and reset the header;
   3. release `R+0x18` and clear it.
3. AddRef `lua` and store it in `R+0x18`.
4. Return `Init()` (§3).

The advisor system calls `SetLua` with the shared context (advisors.md step 5 of its init). Its shutdown passes a
null context (advisors.md), which step 1 **ignores**: the registry keeps its reference and its lists until the next
`SetLua` with another context *[quirk]*.

## 3. `Init()` (`0x004B7720`) *[compat]*

Returns a bool in AL. The application's init calls it directly (application.md); `SetLua` calls it after the reset.

### 3.1 The Lua context and the table

1. **No context yet** (`R+0x18` null):
   1. get the Lua script server (framework slot 5, service `0x4BD10133`, interface `0x4BD10129`);
   2. take its slot 5 (its Lua context), twice;
   3. release any old `R+0x18`, then QI that context for `0xEA10C4A2` (`cISCLua`) into `R+0x18`.

   A missing server, a null slot 5 or a failed QI returns **false**.
2. `L` = `R+0x18` slot 5 (`AsIGZLua5`), then its slot 3 (`AsIGZLua5Thread`). AddRef `L`; `top` = `GetTop()`. A
   null `L` is not tested and faults on the next call *[quirk]*.
3. `PushString("tutorial_registry")`, `GetTable(-10001)`. When `IsTable(-1)` fails:
   1. `Pop(1)`; `SCLuaHelp::LoadResourceScript(R+0x18, 0x4A5E8EF6, "tutorial_registry.lua")` (`0x005FBF50`, lua-host.md).
      A nonzero result: `SetTop(top)`, release `L`, return **false**.
   2. Push and look the global up again. Still not a table: `SetTop(top)`, release `L` (`0x00418710`), return
      **false**.

   The table `T` stays on the stack top for §3.2.

### 3.2 The two sub-tables

1. **Default regions.** Clear `defaultDirs`, then `defaultDirs[0] := "maxisland"`.
   1. `PushString("default_regions")`, `GetTable(-2)`; `PushNil()`. Nothing checks that the value is a table
      *[quirk]*.
   2. While `Next(-2)`:
      1. `n` = `ToNumber(-2)` (the key), cut to an integer. A key that is not a number gives 0.
      2. When `IsTable(-1)`: `PushString("region_dir")`, `GetTable(-2)`; when `IsString(-1)`,
         `defaultDirs[n] :=` that string, lowercased; `Pop(1)`.
      3. `Pop(1)` (the value).
   3. `Pop(1)` (the sub-table).
2. **Tutorials.** Erase every `regions` node.
   1. `PushString("tutorials")`, `GetTable(-2)`; `PushNil()`. Again no table check.
   2. While `Next(-2)`:
      1. Make an item: `guid` = `ToNumber(-2)` (the key), `active` = 1, `region_dir` = "".
      2. When `IsTable(-1)`, read the keys below. Each is pushed, looked up with `GetTable(-2)`, read, then popped.

         | Key | Read | Into |
         |---|---|---|
         | `order` | `ToNumber`, no type check (a missing key gives 0) | `+4` |
         | `task_first` | the same | `+8` |
         | `task_count` | the same | `+0xC` |
         | `region_dir` | only when `IsString`, then lowercased; else it stays "" | `+0x14` |

         Then insert a copy of the item into `regions[region_dir]`'s list. That entry is found or made by
         `0x004B7650`. The copy goes **before the first item whose `order` is ≥ its own**, or at the end when there
         is none. So of two equal orders, the one read later comes first *[quirk]*.
      3. A value that is not a table makes no item.
      4. `Pop(1)` (the value).

   The sub-table is not popped; step 4 of §3.3 restores the stack.

### 3.3 The completed tutorials and the end

3. **Preferences.** `P` = application slot 38 (`GetPreferences`) + `0xF34`: the 256 GUIDs of the completed
   tutorials (tutorial.md §4.2.1). For each cell `i` = 0…255 holding a nonzero GUID:
   - the registry knows it (`GetItem`): set that item's `active` to 0, and `keep` := 1;
   - else: zero the cell, and `keep` := 0.

   Zero cells leave `keep` unchanged. After the loop, when `keep` = 1, call application slot 17
   (`SavePreferences`). So the save happens when the **last** nonzero cell was a known GUID, and is skipped when the
   last one was cleared. That is the reverse of saving after a change *[quirk]*.
4. `SetTop(top)`, release `L`, return **true**.

**The shipped script.** It defines the global table with empty `default_regions` and `tutorials`, and methods that
fill them:
- `add_default_region(pack, dir)` writes `default_regions[pack] = {region_dir = dir}`;
- `add_tutorial(guid, order, dir)` writes `tutorials[guid] = {order, region_dir, task_first = 0, task_count = 0, …}`.

The task scripts fill `task_first` and `task_count` later through `set_task_range_and_print_info`.

The file registers:
- packs 0 and 1, both as `timbuktu`;
- five tutorials, all in `timbuktu`:

  | GUID | `order` | Name in the script's comment |
  |---|---|---|
  | `0xABFAD020` | 0 | Getting Started |
  | `0xEA5D6DC8` | 1 | Terraforming |
  | `0x0BFDB71B` | 2 | Making Money |
  | `0xCC02D3A3` | 3 | Big City |
  | `0x2C12D0E9` | 4 | What's New |

The old Mayor Tutorial (`0x8A5B7A6C`, `maxisland`) is commented out. So, with the shipped data:
- `defaultDirs` is `{0: "timbuktu", 1: "timbuktu"}`: pack 0's `maxisland` is overwritten;
- `regions` has one entry, `"timbuktu"`, with the five items in the order above.

## 4. Lookups

### 4.1 Items (`GetItem` `0x004B5D10`, `SetActive` `0x004B5D80`, the GUID mapping `0x004B5C20`)

These are in tutorial.md §4.2.1 and §3. `GetItem` walks the regions in key order and each list in order, and
returns the first item with the GUID. The mapping turns the two legacy save IDs into GUIDs: 0 → `0x8A5B7A6C`,
1 → `0xEA5D6DC8`, anything else unchanged.

### 4.2 `GetTutorialRegionDefaultDirName(out, n)` (`0x004B6F60`)

The return is a `cRZString`, written into the caller's buffer `out` (a hidden pointer). The call pops 8 bytes and
returns `out`.
1. `s` = a copy of `defaultDirs[0]`. `operator[]` makes the entry (empty) when it is missing, as before the first
   `Init` *[quirk]*.
2. When `defaultDirs` has `n`, `s` := that entry.
3. Return `s`.

So pack `n`'s directory, else pack 0's; with the shipped script, `"timbuktu"` for every pack. The caller is the
nation (feature-manager.md §6).

### 4.3 `GetRegionalCity(guid)` (`0x004B5E50`)

Takes one dword and pops 4 bytes. Returns a regional-city pointer, not AddRef'd, or null.
1. Get the current region: application slot 34 (the nation), then its slot 8.
2. Make an empty `ilist`. Its allocator is the shared one at `0x00B207E0`, fetched once from the framework (service
   `0x3AE4BEA3`, interface `0x3AE4BEAB`). Region slot 17 (`GetAllCities`) fills it.
3. `hex` = `Sprintf("%08x", guid)`: eight lowercase hex digits.
4. Walk the list from the front; skip null entries. A city **matches** when either:
   - `guid` ≠ −1 and the city's slot 65 (`GetTutorialID`, region-model.md) equals `guid`; or
   - its slot 31 (`GetFilePath`) contains `hex` (`cRZString::Find` from 0; `Find` is case-sensitive).
5. Free the list; return the first match, else null.

### 4.4 `GetNextActive(region)` (`0x004B6140`)

Takes a region pointer and pops 4 bytes. Returns an item pointer (list node + 8), or null.
1. `key` = the region's slot 5 string (`GetDirectoryName`, region.md), copied and lowercased. No entry in `regions`,
   or an empty list: return **null**.
2. Walk the list **from the last item to the first** (highest `order` first), with `found` = none:
   - `GetRegionalCity(item.guid)` is null: skip the item.
   - The item is active: `found` := this item.
   - The item is not active (completed): stop.
3. Return `found`, or null.

So the answer is the **lowest-ordered active tutorial above the highest-ordered completed one**. Items whose city is
missing from the region neither count nor stop the walk. When the highest-ordered tutorial with a city is completed,
the answer is null even if lower ones are still active. That answers tutorial.md's "out of order" note.

The region view uses it for the tutorial signpost (region-view.md §6).

## 5. Container helpers

These are STLport container code instanced for the registry's two maps and its list. Any ordered map and list with
the same order and copy semantics will do.

| Address | Role |
|---|---|
| `0x004B5E10` | `less<cRZString>`: compares the two strings' contents (`0x004B5CA0`); a null side compares as empty |
| `0x004B6030` | `cItem` copy constructor: the five fields, then a copy of `region_dir` |
| `0x004B6450` | list insert of an item copy before a given node; returns the new node |
| `0x004B6AA0` | `regions` value copy constructor (key string and a copy of the list, `0x004B65F0`) |
| `0x004B6E70` | `regions` value constructor from a key and a list |
| `0x004B6EF0` | `regions` subtree erase: recursive, frees each list (`0x00498110`) and key |
| `0x004B7030` | `regions` node insert at a found place, with rebalancing |
| `0x004B70F0` | `regions` unique insert |
| `0x004B7330` | `regions` unique insert with a hint |
| `0x004B7650` | `regions[key]` (find or insert an empty list) |
| `0x00418710` | the Lua stack guard's end: `SetTop(saved)`, then release the thread |

## 6. Quirks and cases

- `SetLua(null)` and `SetLua(same)` do nothing and return false; the advisor shutdown therefore leaves the registry
  filled and holding its context.
- `default_regions` and `tutorials` are walked without checking they are tables.
- A tutorial with no string `region_dir` is filed under the empty directory.
- Of two tutorials with the same `order`, the one the table walk gives later is placed first.
- The preferences save depends on the last nonzero cell, not on whether a cell was cleared.
- `GetNextActive` stops at the highest-ordered completed tutorial that has a city.
- `GetTutorialRegionDefaultDirName` before any `Init` makes an empty pack-0 entry and returns "".

| Case | Setup | Expected |
|---|---|---|
| TR-C1 | the shipped script, no completed tutorials, all five cities present | `GetNextActive("Timbuktu")` = `0xABFAD020` (order 0) |
| TR-C2 | the same, `0xABFAD020` and `0xEA5D6DC8` completed | `0x0BFDB71B` |
| TR-C3 | the same, only `0x2C12D0E9` (order 4) completed | null |
| TR-C4 | the same, only `0xEA5D6DC8` completed and the city of `0x0BFDB71B` missing | `0xCC02D3A3` |
| TR-C5 | prefs cells `[0xABFAD020, 0x12345678, 0, …]` | cell 1 zeroed, `0xABFAD020` inactive, **no** save |
| TR-C6 | prefs cells `[0x12345678, 0xABFAD020, 0, …]` | cell 0 zeroed, `0xABFAD020` inactive, save |
| TR-C7 | `GetTutorialRegionDefaultDirName(7)` with the shipped script | `"timbuktu"` (pack 0's entry) |
| TR-C8 | `GetRegionalCity(0xABFAD020)`, a city with tutorial ID −1 whose path contains `abfad020` | that city |

## 7. Function inventory (owner `cSC4TutorialRegistry`)

All 19 functions:
- §2: `0x004B7D70`;
- §3: `0x004B7720`;
- §4: `0x004B5D10`, `0x004B5D80`, `0x004B5C20` (tutorial.md), `0x004B6F60`, `0x004B5E50`, `0x004B6140`;
- §5: `0x004B5E10`, `0x004B6030`, `0x004B6450`, `0x004B6AA0`, `0x004B6E70`, `0x004B6EF0`, `0x004B7030`,
  `0x004B70F0`, `0x004B7330`, `0x004B7650`, `0x00418710`.

Not covered:
- the reset `0x004B72C0` (owner `<shared>`, described in §2);
- the `defaultDirs` internals (`0x004B6250`, `0x004B6D90`) and the list range copy `0x004B65F0` (other owners).
