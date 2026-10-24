# The UI script service (`cGZUIScriptService`) (behaviour spec)

Scope: the `cGZUIScriptService` owner as tools/coverage.py assigns it on `bb641878` (46 functions, 2,786 bytes;
inventory in §6). The service builds windows from UI script resources: it owns a script interpreter, registers the
structural tags, and as the interpreter's callback creates one tag object per element (the `LEGACY` element is the
legacy control of legacy-control.md), keeps the stack of parents for `children` blocks and hands back the finished
container. `SC4CreateWindowFromScript` (ui-windows.md) calls its slot 4.

Related:
- ui-windows.md §5.1 (`SC4CreateWindowFromScript`): service `0x5A356E15`, IID `0xFA3562FA`, class `0xFA3562FB`;
- legacy-control.md: the `LEGACY` tag object (class `0xFA450242`) and its properties;
- system-service-pointer.md: the accessor `0x00449750`.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `CreateWindowFromScript`, `ScriptBegin`,
`ScriptEnd`, `BeginExecuteTag`, `EndExecuteTag`, `ExecuteScriptString`, `Init`, `Shutdown` and the parent stack were
checked on the instructions (each interface method runs on its own `this`, so the decompile's field names are
offset). The vtables `0x00AD5158`, `0x00AD5144`, `0x00AD512C`, `0x00AD5114`, `0x00AD5104`, `0x00AD50D0` and
`0x00AD50B8` and the tag-name strings were read. Written 2026-10-24 (re-gzuiscript7).

Conventions: offsets are from the object start. "The interpreter" is the object at `+0x34`; "interpreter slot n" is
its interface's slot. A key is `{type, group, instance}`.

## 1. Identity and fields *[compat]*

| Offset | Interface (IID) | Table |
|---|---|---|
| `+0x00` | `cIGZUIScriptService` (`0xFA3562FA`, also 1) | `0x00AD5158`, 11 slots |
| `+0x04` | legacy support (`0x7A4A48ED`) | `0x00AD5144`, 5 slots |
| `+0x08` | script callback (`0x1A2BDEB8`) | `0x00AD512C`: 3 `ScriptBegin`, 4 `ScriptEnd`, 5 a shared no-op |
| `+0x0C` | execute-tag callback (`0x1A2BDEB9`) | `0x00AD5114`: 3 `BeginExecuteTag`, 4 `EndExecuteTag`, 5 a shared no-op |
| `+0x10` | tag callback (`0x1A2BDEBA`) | `0x00AD5104`: 3 `ScriptTag` |
| `+0x14` | system service (`cRZSystemService`) | `0x00AD50D0`: 8 `Init`, 9 `Shutdown` |
| `+0x18` | COM base | `0x00AD50B8` |

`QueryInterface` (`0x0094B120`) maps the IIDs above (any other → the system service's); AddRef/Release are the system
service's. Fields: `+0x30` initialised (byte); `+0x34` the interpreter; `+0x38` the root scope object; `+0x3C` "in
setup" (byte); `+0x40` the parent stack (list, front = top, counted); `+0x44` the pending parent; `+0x48` "tag scope
pushed" (byte); `+0x4C` the current tag class; `+0x50` the result container; `+0x54` the current tag object; `+0x58`
the legacy window procedure; `+0x5C` the resource factory (all counted where they are objects).

## 2. The `cIGZUIScriptService` slots *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 3 `CreateWindowFromScript(key, parent, id, iid, out)` | `0x0094B08B` | `w := slot 4(key, parent, id)`; with `w`: `w.QueryInterface(iid, out)` is the result and `w` is released |
| 4 `CreateWindowFromScript(key, parent, id)` | `0x0094B210` | §3; returns the window (AddRef'd) or null |
| 5 / 6 / 7 `ExecuteScriptString(s)` / `File(path)` / `Resource(key)` | `0x0094BA22` / `0x0094BA51` / `0x0094BA80` | set up (§4); interpreter slot 3 `(+0x08)` (the callback), then interpreter slot 6 / 7 / 8 with the argument. Returns `+0x50` and clears it (the caller owns the container) |
| 8 `GetScriptInterpreter()` | `0x0094BAAF` | set up, then `+0x34` (not AddRef'd) |
| 9 / 10 `Get/SetResourceFactory` | `0x009D41AD` / `0x0094B0C7` | `+0x5C` (setter: AddRef new, Release old) |

Legacy support (`+0x04`): slot 3 `SetLegacyParent(window)` (`0x0094B676`): a new `LEGACY` tag object (GZCOM
`(0xFA450242, 0x1A356083)`) gets `window` (its `0x9A4527CB` interface, slot 5) and becomes the pending parent, which
is pushed (§5); slot 4 `SetLegacyWinProc(p)` (`0x0094B0F3`): `+0x58` (Release old, AddRef new).

## 3. Creating a window (slot 4) *[compat]*

1. Copy the key. With the persistent resource manager: the graphics system's slot 3 object's width and height
   (slots 41, 42) are printed as `"0x%.4u%.4u"` and read back as hexadecimal; that becomes the key's group when the
   resource manager has a resource with it (its slot 14), else the original group stays. So a script may have a
   variant per resolution, its group spelling the width and height in decimal digits (1024 × 768 → `0x10240768`).
2. `SetLegacyParent(parent)`.
3. `r := ExecuteScriptResource(&key)`. With `r`: its slot 5 `(id)` gives the element; that element's `0x9A4527CB`
   interface slot 3 gives the window (AddRef'd), the result. Then `r` slot 9 `(0)` and `r` is released.

(ui-windows.md's table that calls slot 4 "not separately named" and `0x0094B210` a struct-taking overload is
corrected here: `0x0094B210` is slot 4, and its first argument is the key pointer.)

## 4. Setup and the interpreter callbacks *[compat]*

**Setup** (`TestSetup`, `0x0094B73A`), once: the interpreter is created (GZCOM `(0xBA2E7953, 0x5A2C1570)`) and the
tags are registered (interpreter slot 9 `(name, id)`): `_null` 0, `_children` 1, `_/children` 2, `_define` 3, `_name`
4, `_val` 5; then, with "in setup" set and the callback installed (interpreter slot 3 `(+0x08)`): `none` 0, `children`
1, `/children` 2, `define` 3, `name` 4, `val` 5, `LEGACY` `0xFA450242`, `comic9` `0x12`, `comic10` `0x13`; "in setup"
cleared. Its result is whether the interpreter exists.

- **`ScriptBegin(interp)`** (`+0x08` slot 3, `0x0094B19D`): unless in setup, a root scope (GZCOM `(0xBA2E7954,
  0x1A2BDEBB)`) into `+0x38` and interpreter slot 13 `(it)` (push scope). Then the result container (GZCOM
  `(0x5A354595, 0x1A35608A)`) into `+0x50`; the result is whether it was made.
- **`ScriptEnd(interp, ok)`** (`+0x08` slot 4, `0x0094B55D`): not `ok` → release the container and the current tag
  object, and pop the tag scope if pushed (interpreter slot 14). Then, unless in setup: pop the root scope, release
  it, pop every parent, clear the pending parent. True.
- **`BeginExecuteTag(interp, class)`** (`+0x0C` slot 3, `0x0094B364`): `+0x4C := class`; the old tag object is
  released and a new one created (GZCOM `(class, 0x1A356083)`; failure → false). Its `0x9A4527CB` interface gets the
  legacy window procedure (slot 6); the object gets the top parent (slot 9) and becomes the pending parent;
  interpreter slot 13 `(its slot 4)` pushes its scope; "tag scope pushed" := 1; true.
- **`EndExecuteTag(interp, …)`** (`+0x0C` slot 4, `0x0094B1F4`): pop the tag scope when pushed; true.
- **`ScriptTag(interp, id, params)`** (`+0x10` slot 3, `0x0094BABC`): with a current tag object and a container, the
  object takes the parameters (its slot 3 `(params)`), is added to the container (container slot 4) and released.
  Otherwise by `id`: 1 (`children`) pushes the pending parent; 2 (`/children`) pops one, and the new top becomes the
  pending parent; 3 (`define`): from the `(id, value)` pairs, the `name` (4) value when it is a string variant (type
  `0x800E`) is defined in the interpreter (slot 11 `(name)`), and with a `val` (5) also given a value (slot 10
  `(name, val)`). Other ids → false.

## 5. The parent stack and lifecycle

`PushParent` (`0x0094B653`): the pending parent, if any, is AddRef'd onto the front of `+0x40` and cleared; true.
`PopParent` (`0x0094B53A`): release and drop the front; whether there was one. `TopParent` (`0x0094B354`): the front
or null.

`Init` (`0x0094B07E`): initialised := 1; true. `Shutdown` (`0x0094B5F4`): when initialised, release the interpreter,
the container, the current tag object and the resource factory, pop every parent, clear the flag. Destructor body
`0x0094B6EC` (restore the tables, `Shutdown`, destroy the stack), deleting form `0x0094BBE2`.

## 6. Quirks and function inventory

Quirks:
1. The resolution-specific group is formed from decimal digits printed into a hex string (`%.4u` twice), so only
   sizes below 10000 fit and the group reads as hexadecimal of those digits.
2. `ExecuteScript*` hand back the container and forget it; a second call before release would replace it.
3. `comic9`/`comic10` are registered as tags `0x12`/`0x13` but `ScriptTag` has no case for them (they fall to
   "false") *[inferred: font aliases handled by tag objects]*.

Inventory (46): `0x0094B07E`, `0x0094B08B`, `0x0094B0C7`, `0x0094B0F3`, `0x0094B120`, `0x0094B18D`, `0x0094B195`,
`0x0094B19D`, `0x0094B1F4`, `0x0094B210`, `0x0094B33A` (stack node), `0x0094B354`, `0x0094B364`, thunks
`0x0094B4A2`…`0x0094B532` (19, every 8 bytes), `0x0094B53A`, `0x0094B55D`, `0x0094B5F4`, `0x0094B653`,
`0x0094B676`, `0x0094B6EC`, `0x0094B73A`, `0x0094BA22`, `0x0094BA51`, `0x0094BA80`, `0x0094BAAF`, `0x0094BABC`,
`0x0094BBE2`, `0x009D41AD`.

Not covered: the interpreter, scope, container and tag-object classes themselves, and the UI-script COM directors.
