# Resource and exemplar editor: the property view, Ingred.ini and the value editors — behaviour spec

Status: round 1 (lane re-editors4, 2026-10-21), written from the Windows 1.1.641 executable through the Ghidra
export (tools/re_query.py) and decomp/. REA was unavailable in this round (no app open), so every fact rests on
export instructions and the decompile; numbers were read from the instructions or the data bytes. Facts not proven
on the instructions are tagged *[inferred]*.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced.

---

## 1. Purpose and scope

SimCity 4 ships an in-game exemplar ("ingredient") editor left over from development: a window that lists the
properties of one exemplar resource, shows each value as text, and opens a typed value editor for a property. Its
vocabulary (property names, types, default values, which editor to use, enumerations) does not come from code: it is
read from a text file, `Sys\Ingred.ini`, under the game's data directory. This spec covers:

- the property view `cSCPropertyView` and its free helpers (`getVariantType`, `getDefaultValue`,
  `getUnquotedString`, `convertStringToFloatValue`), §2–§7;
- the value editors `cSC*EntryView` and the value dialog `cSCEditValueDlg`, §8–§9 (identity level in this round);
- the honest owner map, §10.

Out of scope: the DBPF and exemplar binary formats (data-formats.md, owned by the formats lanes), the resource
catalog browser `cSCResourceCatalogView` / `cSCResCatBaseView` / `cSCResCatalogPickerView` and the key picker
`cSCResKeyInputView` beyond their class IDs (§8), and the widget toolkit (ui-windows.md).

Subsystem size (tools/coverage.py --owners, 2026-10-21): 68985 bytes. The family specified here
(`cSCPropertyView` 17843 + free functions 5164 + entry views and dialog about 29700) is the largest coherent part.

## 2. The ingredient file `Sys\Ingred.ini` *[compat]*

`cSCPropertyView::processINI` (0x00673CF0) reads the file once per process: `cSCPropertyView::Init` (0x006751F0)
calls it only while the global "already loaded" byte at 0x00B468F0 is zero. The path is the game data directory
(`RZ::DDir`) joined with `Sys\Ingred.ini` (string 0x00AAE58C). It is a standard INI file (cRZIniFile). Sections are
processed in this order, each only if it exists:

| Order | Section | Entry callback | Effect |
|---|---|---|---|
| 1 | `Aliases` | 0x00672360 | name → 32-bit number table (§3.1) |
| 2 | `ExemplarFilters` | 0x00672CD0 | filter name → bit mask table (§3.2) *[inferred: callback not read]* |
| 3 | `Properties` | 0x00672670 | one property definition per entry (§4) |
| 4 | `ResKeyTableEditorDefaults` | (not read this round) | defaults for the resource-key table editor (§8) |
| 5 | `Uint8ValueMap_0x%08X` | 0x00673A70 | per-property enumeration for UINT8 values (§3.3) |
| 6 | `Uint32ValueMap_0x%08X` | 0x00673BB0 | per-property enumeration for UINT32 values (§3.3) |

The value-map sections are looked up per property: the `%08X` is the property ID in upper-case hex
(format strings 0x00AAE534 and 0x00AAE51C), so `[Uint32ValueMap_0x27812810]` holds the enumeration of property
0x27812810.

### 3.1 Aliases

Each entry is `<number> = <name>`. The entry key is converted with the engine's string-to-uint32 rule
(RZConvertToUint32A: decimal, or hex with a `0x` prefix); the value is trimmed on both sides and becomes the alias
name. If the name is already present with the same number the entry is a no-op; otherwise the name is (re)bound to
the number, so a later entry wins. Alias names are case-sensitive keys of an ordered map. Aliases are used wherever
a later field accepts "a number or a name" (§4 fields 7–9, §3.2).

### 3.2 Filters

A filter field is a list of words separated by single spaces (separator byte 0x20 at 0x00A85B40). Each word that
starts with a digit is converted as a number; any other word is looked up in the name table and contributes 0 when
absent. The result is the bitwise OR of all words (`getFilter`, 0x00672480). `convertStringToFilter` (0x00672420)
is the one-word form: digit-led → number, known name → its value, unknown → 0.

The property view keeps a current filter mask (field +0xD8). Only properties whose filter mask ANDs non-zero with
it get a value editor (§6).

### 3.3 Value maps

Each entry of a `Uint8ValueMap_0x…` / `Uint32ValueMap_0x…` section binds one numeric value of that property to a
display name; the maps are read back by `GetPropertyValueMap` (0x006750F0 for UINT8, 0x00675170 for UINT32) and
feed the combo-box editors (§8). *[inferred: entry direction, value = key, not checked on the instructions]*

## 4. A `[Properties]` entry *[compat]*

The entry key is the property ID (converted as a number). The value is a comma-separated record (separator byte
0x2C at 0x00A99F14), parsed left to right; leading blanks before a field are trimmed. Fields:

| # | Field | Parsing | Stored at (tPropertyInfo) |
|---|---|---|---|
| 0 | key | property ID, number | +0x00 |
| 1 | type | type name, §5; unknown or empty → the whole entry is ignored | +0x04 |
| 2 | count | number of values; 0 = "list of any length" | +0x08 |
| 3 | default | quoted string, parsed by type and count (§7) | value variant (+0x5C) |
| 4 | filter | §3.2 | +0x0C |
| 5 | name | quoted string, display name | +0x14 (string) |
| 6 | help | quoted string, help text shown in the `<help>` area | +0x28 (string) |
| 7 | editor class | number, or alias name (§3.1) | +0x38 |
| 8 | editor parameter | quoted string, or number, or alias, or `prefix_number` (below) | +0x3C, +0x40, +0x44 |
| 9 | last field | number or alias, after trimming both ends | +0x48 |

Rules:

- **STRING** properties are accepted only with count 1; a STRING entry with any other count is dropped.
- **Count 0**: the default value is an empty UINT32 array regardless of the declared type (0x00672670 builds it
  with one zero-length uint32 reference). *[quirk]*
- **Default parse failure** (getDefaultValue returns false): the entry is dropped.
- **Field 8**: if it starts with `"` the unquoted text is stored at +0x40 and +0x3C is set to 0x073B6FD3
  (a "literal string parameter" marker). Otherwise the next comma field is taken: digit-led → number into +0x3C;
  a known alias name → its number into +0x3C and the property ID into +0x44; an unknown name of the form
  `<alias>_<digits>` → the alias of the part before the first `_` into +0x3C and the numeric part into +0x44
  (fallback separator `_`, 0x00AA1B94); anything else leaves +0x3C unchanged.
- The finished record replaces any earlier record with the same property ID (map assignment), so the last entry
  for an ID wins.

## 5. Type names *[compat]*

`getVariantType` (0x0066EE80) trims the word, upper-cases it and maps it to a cRZVariant type code. An empty word
returns 0 without comparing. Codes read from the return instructions:

| Name | Code | Name | Code |
|---|---|---|---|
| BOOL | 0x01 | SINT32 | 0x07 |
| UINT8 | 0x02 | SINT64 | 0x09 |
| SINT8 | 0x03 | FLOAT32 | 0x0A |
| UINT16 | 0x04 | STRING | 0x800E |
| SINT16 | 0x05 | anything else | 0 (entry ignored) |
| UINT32 | 0x06 | | |

Names are matched case-insensitively because the word is upper-cased first.

## 6. Building the view

`Init` (0x006751F0), when the window was not already initialised:

1. Loads Ingred.ini if not yet loaded (§2).
2. If the window has no size yet it becomes 400×450 (0x190 × 0x1C2) and is centred in its parent
   ((parent − own) / 2 on each axis).
3. Sets window flags 0x800 (cleared) and 0x10000 (set) and creates its controls: a `Done` button (text
   0x00AAE61C, button ID 0x24C93018), a `Cancel` button (ID 0x24C93019), a help text area (child 0x87C30D17)
   initialised to `<help>`, and the property list (child 0xFA12B77F) whose header reads `Property ID`.
   *[inferred: control-to-string pairing from the order of the string references in Init; the IDs are proven by
   §6.1]*
4. `mapPropertiesToViews` (0x00672F70) empties the property-ID → value-editor map, then walks every known property
   definition in ID order; for each whose filter mask ANDs with the view filter it asks `GetValueEditor` for an
   editor and stores it under the property ID.

`GetValueEditor` (0x00672D60) for one property ID:

- Unknown ID → no editor.
- The value comes from the caller's variant, or, when none is passed, from the exemplar's current value; when that
  value is a resource-key reference (type tag 0x10) it resolves the referenced object (interface 0xE4FDA3D4) and
  wraps it. Otherwise it copies the definition's default (+0x5C).
- It creates an instance of the definition's editor class (+0x38) through the GZCOM class factory, asking for the
  value-editor interface IID 0x06C5FF7B (cISCValueEditView). If creation fails there is no editor.
- For editor classes 0xA7847D6A (resource-key table, §8) and 0x281F7625 it also queries interface 0x0787ECC4 and,
  if the property ID has a stored parameter record, passes the record's +0x14 and +0x18 values to it.
- It then initialises the editor with the property ID and the value.

### 6.1 Commands, selection and write-back *[compat]*

The view edits one target property object (field +0x114): the player picks a property ID from the list, edits its
value in that property's editor, and `Done` writes the pair into the target. Rows of the list map, by index, to an
ordered vector of property IDs (fields +0xDC..+0xE0); an index at or past the vector's end is ignored.

`GZOnCommand` (0x0066F740):

| Message | Parameter | Action |
|---|---|---|
| 0x287259F6 (button press) | button ID | `doButtonPress` (below); its result is the handler's result |
| 0xE8711255 (list selection) | 0xFA12B77F (the property list) | select the row (below), then set the help area's text (child 0x87C30D17, text interface 0x212CDC1F) to `GetHelpText` of the row's property; returns 0 |
| anything else | | not handled (returns 0) |

Selecting a row: the list (interface 0x412CE496) reports the selected index; the editor shown so far (field +0xF4)
is hidden (window vtable slot +0x118), the row's editor from the property-ID → editor map becomes the current one and
is shown (+0x114). A property without an editor leaves no editor shown.

`doButtonPress` (0x0066F530):

- **Done, 0x24C93018**: performs the same row selection from the list's current index, then
  `assignValuesToProperty` for that row's property ID, then ends the dialog with result 0.
- **Cancel, 0x24C93019**: ends the dialog with result 1. Nothing is written.
- Other IDs: not handled.

`assignValuesToProperty` (0x0066EC80): if the property ID has an editor, reads the editor's value into a fresh
variant (editor slot +0x1C), sets the target property's ID (target slot +0x10) and then its value from the variant
(target slot +0x1C). With no editor nothing is written.

Ending the dialog (helper 0x0066DE90, misnamed `std::vector::size` in the export): if the window has flag 0x1000
(modal), it asks the window manager to end the modal loop (manager slot +0x24, arguments 0x0F, the window, the
result); otherwise it takes the non-modal path (closes the window) *[inferred: else branch not read]*. The modal
loop itself is modal-dialogs.md §2.

## 7. Values as text and default values

`GetPropertyValueStr` (0x00674140) formats a property value for the list:

- An empty list prints `<empty list>`.
- Integers print in hex with width by size: `0x%02X` for 8-bit, `0x%04X` for 16-bit, `0x%08X` for 32-bit.
- BOOL prints `True` / `False`; in a list every element after the first is appended as `, True` / `, False`.
- A pointer-like value the formatter does not know prints `Unk. ptr: %0x08X` *[quirk: the malformed format string
  is in the binary at 0x00AAE5AC]*.
- `GetPropertyNameStr` (0x0066E3E0) prints the property's name, or `0x%08X` of the ID when the definition has no
  name.

The default-value parsers (`getDefaultValue`, eight per-type bodies 0x0066F8E0–0x00670940, and
`convertStringToFloatValue` 0x00670300) split the quoted default into `count` items and convert each by type; a
conversion failure makes the whole entry invalid (§4). `getUnquotedString` (0x00670E10) takes the next field: if it
starts with `"` it returns the text up to the closing quote and consumes the following comma. *[inferred: per-item
separator and range checks not read this round]*

Diagnostics (debug log strings, never shown to the player): `Invalid String Res key: G:%08X T:%08X I:%08X`,
`Missing Debug Name Exemplar Res Key: …`, `Invalid Exemplar Res Key: …`, `Missing Res Key: …`, and the plain key
form `G:%08X T:%08X I:%08X` used by `getFilenameFromKey` (0x0066F360).

## 8. Value editor classes *[compat for the IDs]*

The editors are GZCOM classes registered by `cSC4ResourceLibCOMDirector`'s constructor (0x00664920) with
`AddCls(class ID, factory)`. An Ingred.ini field 7 names one of these IDs (directly or by alias):

| Class ID | Factory | Class (vtable written by the factory) |
|---|---|---|
| 0x1A368ED8 | 0x00664280 | cSCResourceCatalogView |
| 0x671E9725 | 0x00664750 | cSCResCatalogPickerView (catalog view + picker vtables) |
| 0x8739DFC1 | 0x00687F40 | cSCUint32ComboBoxEntryView |
| 0x87C34EDA | 0x00687FA0 | cSCUint8ComboBoxEntryView *[inferred: vtable 0x00AB0428 not in docs/abi]* |
| 0xCA0CB10F | 0x00665C80 | cSCColorSequenceEntryView |
| 0x071E971B | 0x006648F0 | cSCHappinessCurveEntryView |
| 0xE8DDA013 | 0x006642A0 | cSCGeneralResponseCurveEntryView |
| 0x671E972A | 0x006642D0 | cSCStringEntryView |
| 0x490E01A7 | 0x00664310 | cSCSint64TextEntryView |
| 0xC71E972F | 0x00664350 | cSCUint32TextEntryView |
| 0x471E9733 | 0x00664390 | cSCSint32TextEntryView |
| 0x471E9737 | 0x006643D0 | cSCSint8TextEntryView |
| 0xC71E973C | 0x00664410 | cSCUint8TextEntryView |
| 0xC71E9740 | 0x00664450 | cSCFloat32TextEntryView |
| 0x871E9744 | 0x00664490 | cSCUint32SpinnerEntryView |
| 0x47C35AFC | 0x006644D0 | cSCBoolTextEntryView |
| 0x0A09AC46 | 0x00687E20 | cSC16ColumnGridEntryView |
| 0xA7847D6A | 0x00684260 | cSCResKeyTableEntryView |
| 0xA73B81F0 | 0x00662730 | 16-byte ref-counted helper with three interfaces (0x00AAB0D8/0x00AAB0C8/0x00AAB0B8), not a window (§9) |
| 0x67336285 | 0x006836E0 | resource-key input object, vtables 0x00AAF850/0x00AAF838 *[inferred]* |

The same director also registers the exemplar resource factory (0x453429B3), the cohort factory (0x053429C8), the
exemplar cache (0x252EA0DF) and the file parser; those belong to the formats/persistence subsystem.

All typed text editors share the base `cSCStandardTypeEntryView` (ctor 0x00687C20). Class 0x281F7625, which §6
special-cases, is not registered by this director. *[inferred: registered elsewhere or unused]*

### 8.1 The typed text editors *[compat]*

`cSCUint8/Sint8/Uint32/Sint32/Sint64/Float32/BoolTextEntryView` and `cSCStringEntryView` share one shape (Init at
0x006891D0, 0x00688E90, 0x00688510, 0x00688830, 0x006881D0, 0x00689510, 0x00688B50, 0x00688000):

- Init runs once (window init flag); it clears window flag 0x800, sets 0x10000 and adds the base class's background
  fill control (`AddBackgroundFillControl`, 0x00687960).
- If the value is a single scalar of the editor's type, the editor creates one line input at (5, 5) holding the
  value's text. If it is a list, it creates one line input per element at x = 5, y = 5 + 20·i, element i's child ID
  being the base ID + i. An empty list creates no input.
- The text of a number is `GetPropertyValueStr` of that one element (§7), so integers start as hex text.
  The BOOL editor writes `true` (0x00AAEDD4) or `false` (0x00A86DE0), lower case, instead.
- Base child IDs: 0x5A293904 for UINT8, SINT8, UINT32 and SINT32; 0xDA4FB6F6 for FLOAT32; 0x67C35A67 for BOOL;
  0xF9E2E9E4 for STRING; 0x490E039A for SINT64.

Reading back (`GetValue`, proven for SINT64 at 0x00688370): each input's text (text interface 0x231A1C57) is
converted and stored. For a list the conversion is RZAtoi64A per element into a new array of the same length. For a
**single** SINT64 the text goes through the C runtime `atol` and is sign-extended (CDQ at 0x0068841B), so a scalar
SINT64 above 2^31−1 or below −2^31 is not kept *[quirk]*; list elements keep the full 64 bits. The other text editors'
GetValue bodies sit outside this subsystem's byte count and were not read this round.

## 9. The edit-value dialog `cSCEditValueDlg` *[compat]*

A window (vtable 0x00AAB0F8, cGZWin-derived) that edits **one** property object. It holds the property at +0xD8
(a cISCProperty: slot +0x0C property ID, +0x14 value, +0x1C set value) and its current value editor at +0xDC
(a cISCValueEditView, IID 0x06C5FF7B, from `GetValueEditor` §6; its window comes from editor slot +0x0C, its value
from editor slot +0x1C). Who opens the dialog was not found this round (no reference to the vtable in the export).

Note: the class ID 0xA73B81F0 of §8 is **not** this dialog. Its factory 0x00662730 allocates a 16-byte reference
counted object with three interface vtables (0x00AAB0D8, 0x00AAB0C8, 0x00AAB0B8); it is no window.

### 9.1 Init (0x00667320, not decoded by Ghidra; read from the instructions)

Runs once (window init flag; already initialised → returns true). The dialog keeps its current size (no default
size is set). With W, H the dialog's width and height:

1. Window flag 0x800 cleared, 0x10000 set.
2. Buttons (helper `createButton` 0x0066E9F0, button class 0x22DEC92D), laid out right to left along the bottom with
   a 5-pixel margin and gap. Each button keeps the size the helper gave it; with w its width and h the Okay
   button's height, the row top is T = H − 5 − h.

   | Order | Text | Command / button ID | Style arg | Left edge |
   |---|---|---|---|---|
   | 1 | `Okay` | 0x26C10A3B | 0x2001 | W − 5 − w |
   | 2 | `Cancel` | 0x26C10A3C | 0x2007 | previous left − 5 − w |
   | 3 | `Add Value` | 0x27C32AA4 | 0x2010 | previous left − 5 − w |
   | 4 | `Remove Value` | 0xC7C32AA9 | 0x2011 | previous left − 5 − w |

   All four sit at top T. Buttons 3 and 4 exist only when the property is a **variable-length list** (helper
   0x0066E470): the property ID has an Ingred.ini record, its count field (§4 field 2) is below 1, and its editor
   class is not 0xE8DDA013 (the general response curve). Unknown ID → no Add/Remove.
3. Help label (`createLabel` 0x0066DDB0) with the record's help text (`GetHelpText` 0x0066E430), text colour from
   `MakeFillColor(0, 0, 0)` (black), area left 5, top T, right = left edge of the leftmost button, bottom H − 5.
4. Value editor: `GetValueEditor(property ID, the property's current value, 0x06C5FF7B, &editor)`. If that
   succeeds the editor's window gets the area (5, 5)–(W − 5, T) and is added as a child.
5. Finishes with the base window Init and returns its result.

### 9.2 Buttons (`doButtonPress` 0x00666D00, slot +0x25C)

`GZOnCommand` (0x0066B160) forwards message 0x287259F6 with its argument (the button ID) to `doButtonPress`; any
other message returns false. `doButtonPress` returns true for every ID except as noted:

- **Okay, 0x26C10A3B**: with an editor: reads the editor's value into a fresh variant, stores it into the property
  (property slot +0x1C), then ends the dialog through the window manager (dialog slot +0x18, manager slot +0x24,
  arguments 0x0F, the dialog, **1**) and returns that call's result. Without an editor: ends with **0**, nothing
  written.
- **Cancel, 0x26C10A3C**: ends the dialog with 0; returns that call's result.
- **Add Value, 0x27C32AA4** (no editor → nothing): reads the editor's value; for element types BOOL, UINT8, SINT8,
  UINT16, SINT16, UINT32, SINT32 and FLOAT32 (variant type with the array bit 0x8000 masked) it builds a new array
  of count + 1 elements: the old elements copied in order, the new **last** element zero (false / 0 / 0.0).
  For any other type (SINT64 code 9, code 8, STRING) no array is built and the new variant stays empty *[quirk]*.
- **Remove Value, 0xC7C32AA9** (no editor → nothing): reads the editor's value; with **fewer than 2** elements
  nothing happens at all. Otherwise, for the same eight types, the new value is the first count − 1 elements (the
  last element is dropped); other types again leave the new variant empty *[quirk]*.
- After Add or Remove the editor is **rebuilt**: the old editor window's left, top, right, bottom are read, the old
  window is removed from the dialog (`ChildRemove`), `GetValueEditor(property ID, new value, 0x06C5FF7B, &editor)`
  replaces the editor in +0xDC, the new editor's window gets the same four edges (`SetArea`) and is added
  (`ChildAdd`), and one reference on the editor in +0xDC is released. Nothing is written to the property until
  Okay. *[inferred: reference ownership of the replaced editor (the old pointer is overwritten by
  GetValueEditor, the release goes to the new one)]*
- Any other ID: returns true, does nothing.

Worked example: a UINT32 list property (count 0) holding {0x10, 0x20}. Add Value → editor shows {0x10, 0x20, 0x0};
Remove Value twice → {0x10}; a third Remove does nothing (fewer than 2); Okay writes {0x10} to the property.

## 13. Open points for round 3

Per-editor behaviour (Init layout, value parsing and range checks of each text editor, curve editors' point
editing, colour sequence rows, resource-key table add/edit/delete and save status), `cSCEditValueDlg::doButtonPress`
(0x00666D00, 1488 bytes), the `ResKeyTableEditorDefaults` and `ExemplarFilters` callbacks, and how edited values are
written back to the exemplar (beyond §6.1).

## 14. Owner map (honest)

| Owner | Bytes | Level after round 1 |
|---|---|---|
| cSCPropertyView | 17843 | INI loading, record format, type table, view build, commands and write-back, value formatting: specified; ResKeyTableEditorDefaults/ExemplarFilters callbacks and GetPropertyValueStr per-type detail partial |
| fn:getDefaultValue, getUnquotedString, convertStringToFloatValue | 5164 | contract specified, item-level parsing inferred |
| cSC*EntryView, cSCEditValueDlg | ~29700 | typed text editors (8 classes, ~3700 bytes): layout and Init specified, SINT64 read-back specified; curve, colour, grid, combo, res-key table editors and cSCEditValueDlg: identity only |
| cSCResourceCatalogView, cSCResCatBaseView, cSCResCatalogPickerView, cSCResKeyInputView | 16686 | identity only |

Proposed coverage-map entry: `spec` = `resource-exemplar-editor.md` for `cSCPropertyView` and the three free
functions only; leave `cSC*EntryView`, `cSC*View` and `cSCEditValueDlg` at `-` until their behaviour is written.
