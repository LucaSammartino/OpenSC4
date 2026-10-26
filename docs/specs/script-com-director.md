# The script COM director and the script vocabulary (behaviour spec)

Scope: the owner `cGZScriptCOMDirector` as tools/coverage.py assigns it (23 functions): the GZCOM director of the
legacy script interpreter, the interpreter's constructor and the **vocabulary** class (a case-insensitive table from
names to objects) with its map helpers. The interpreter's methods are gz-framework-core.md §9; its vocabulary
stack (PushVocabulary, lookup) is §9.1 there; the director base is com-directors-misc.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile and the constructors in the disassembly; the
vtables were read in the PE. Written 2026-10-25 (re-scriptdir8).

## 1. The director *[compat]*

`GetGZScriptCOMDirector` (`0x009444CF`) returns the process-wide director at `0x00B633F8`, constructed on the first
call (guard bit 0 of `0x00B63444`; the destructor is registered with `atexit`). The constructor (`0x0094448E`) builds
the `cRZCOMDllDirector` base, installs its tables and registers two classes:

| Class | Factory | Object |
|---|---|---|
| `0xBA2E7953` | `0x00944462` | a script interpreter (`cGZScriptTerp`, `0x5C` bytes, constructor `0x00945404`) |
| `0xBA2E7954` | `0x00944478` | a vocabulary (`0x14` bytes, constructor `0x009453D4`) |

Each factory allocates and constructs, returning null when the allocation fails. `GetDirectorID` (`0x009444C9`)
returns `0xFA2E78C2`.

## 2. The interpreter's constructor (`0x00945404`) *[compat]*

Vtable `0x00AD4718`; the reference count `+0x04 := 0`; the vocabulary stack `+0x14…+0x1C` (begin, end, capacity)
empty; `+0x08`, `+0x0C`, `+0x10`, `+0x20`, `+0x24`, `+0x28`, `+0x2C`, `+0x3C`, `+0x40` := 0; the byte `+0x38 := 0`; a
text buffer at `+0x44` (`0x00944A77`: begin/end at `+0x44`/`+0x48` set to a fresh 64-byte block, capacity `+0x4C`, the
words `+0x50` and `+0x58` zero, and its allocator at `+0x54`, a new `cGZDefaultAllocator` (8 bytes, AddRef'd) that
also provided the block). Finally a new vocabulary is created, AddRef'd and pushed onto the vocabulary stack, so a
fresh interpreter always has one (the bottom one PopVocabulary keeps).

## 3. The vocabulary *[compat]*

`0x14` bytes: vtable `0x00AD4764`, the reference count `+0x04` (0 at construction), and an ordered map at `+0x08` from a
name (a string kept in the node at `+0x10`) to an object (`+0x24`), ordered by `_stricmp(a, b) < 0` on the names
(`0x009446C1`), so names are case-insensitive and unique.

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x009444FA` | QueryInterface: IID 1 and `0x1A2BDEBB` → itself, AddRef, true; else false without writing |
| 1, 2 | shared | AddRef / Release of the reference count |
| 3 | `0x00945527` | `AddVariant(name, value)`: a new variant copied from `value` (`cRZVariant`, `0x14` bytes) is inserted under `name` when the name is absent and then AddRef'd; when the name exists the new variant is destroyed (its slot 127 with 1) and nothing changes. Returns whether it was inserted |
| 4 | `0x009455C1` | `Add(name, object)`: inserts `object` when `name` is absent and AddRefs it. Returns whether it was inserted |
| 5 | `0x009452A1` | `Remove(name)`: when present, release its object and erase the entry. Returns whether it was present |
| 6 | `0x00944BDE` | `Find(name, &out)`: when present, `out :=` the object, AddRef'd, true; otherwise false and `out` unchanged |
| 7 | `0x00945310` | `RemoveAll`: release every object in name order, empty the map, true |
| 8 | `0x0094550B` | deleting destructor: RemoveAll, free the map (`0x009452F9`), free when flag bit 0 is set |

The name arguments are strings; each call builds a temporary copy of the name for the lookup. The map helpers
`0x0094481E` (find), `0x0094533F` and `0x0094547A` (insert, reporting whether a new node was made), `0x00945243`
(recursive free) and `0x00945278` (clear) are the ordinary ordered-map operations.

## 4. Conformance tests

1. `Add("Width", a)` then `Add("WIDTH", b)`: the second returns false and b is not referenced; `Find("width")` gives a.
2. `Remove("Width")` releases a once and returns true; a second `Remove` returns false.
3. A new interpreter's vocabulary stack holds exactly one vocabulary, and PopVocabulary on it returns false.
