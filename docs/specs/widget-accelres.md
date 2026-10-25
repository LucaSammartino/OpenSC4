# `cRZWinKeyAcceleratorRes`: the key accelerator text resource (behaviour spec)

Scope: the owners `cRZWinKeyAcceleratorRes` (17 functions, 3,105 bytes) and `cGZWinKeyAcceleratorResFactory` (1, 61)
as tools/coverage.py assigns them on `bb641878` (inventory in §4). It is a persistent resource that reads a text file of key bindings into a map of key code →
message and registers them with a key accelerator (`cRZWinKeyAccelerator`, widget-toolkit.md §13, whose key-code
layout §13.2 this parser produces).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the second-`=` branch and the token loop were
checked on the instructions. The vtables `0x00ADA5F4` (9 slots) and `0x00ADA5C0` (the `cGZPersistResource` side, 13
slots) and the strings `0x00AC1274` (`"//"`), `0x00AA1C7C` (`"alt"`), `0x00AA1BC4` (`"down"`), `0x00AA1BD4`
(`"up"`) and the key-name strings `0x00AA1B6C`…`0x00AA1C62` were read. Written 2026-10-24 (re-gzaccelres7).

## 1. Identity *[compat]*

`0x28` bytes: vtable `0x00ADA5F4` at `+0`, a `cGZPersistResource` at `+4`, and the bindings (an ordered map key code
→ `cGZMessage {type, d1, d2, d3}`, header at `+0x1C`). Constructor `0x00984425`; destructor body `0x0098435D`,
deleting form `0x00984452`.

**Factory** `cGZWinKeyAcceleratorResFactory::CreateInstance(type, iid, out)` (`0x0098446E`): for resource type
`0xA2E3D533`, a new resource and its `QueryInterface(iid, out)` (the allocation is not checked for null); any other
type → `out := null`, false.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x009841FC` | `0x42E3EA4B` → itself, AddRef'd; else the persist resource's |
| 1 / 2 | `0x0098421E` / `0x00984226` | the persist resource's AddRef / Release |
| 3 `ApplyTo(accel)` | `0x009842EE` | for every binding in key-code order: `accel` slot 5 `RegisterAccelerator(code, message)` (so an accelerator's existing binding wins) |
| 4 `SerialRead(stream)` | `0x00984587` | §2; always true |
| 5 | shared | false (no writer) |
| 6, 8 | `0x00984241` | the persist resource's `DebugInstanceTag` |
| 7 `DebugClassTag(out)` | `0x0098422E` | `out` := `"cRZWinKeyAcceleratorRes"` (its slot 4) |

The persist-resource table's slots 0–2 and 12 are thunks (`0x0098437D`, `0x00984385`, `0x0098438D`, `0x00984395`).

## 2. The text format (`SerialRead`) *[compat]*

**Lines** (`0x0098452F`): characters are read one at a time (stream slot 27 `(&c, 1)`) up to CR or LF; empty lines
are skipped; reading stops at the end of the stream. Each line is trimmed and ignored when it is empty, starts with
`;` or `//`, has no `0x`, or has no `=`.

**Split**: normally the keys are the text before the first `=` and the message is the text after it. When a second
`=` follows the first and comes before the first `0x`, the keys are the text up to one character before that second
`=` and the message the text after it: this lets `=` itself be a key name (`control = = 0x…`).

**Keys**: the key text is split at spaces; each token, trimmed and lower-cased:
- `up` after a key → the binding is for key up; `down` after a key → key down, but the token then goes on to the key
  rules below and becomes the Down key (`0x28`) *[quirk]*;
- `control`, `shift`, `alt` → the modifier;
- any other token sets the key (`vk`):

| Token | `vk` |
|---|---|
| one letter or digit | its upper-case character code (`A` = `0x41`, `5` = `0x35`) |
| starting with `0x` | hexadecimal |
| digits only | decimal |
| contains `numlock` / `capslock` / `scrolllock` | `0x90` / `0x14` / `0x91` |
| contains `back`, `tab`, `return` or `enter`, `pause`, `break` or `cancel`, `esc`, `space` | `0x08`, `0x09`, `0x0D`, `0x13`, `0x03`, `0x1B`, `0x20` |
| contains `pageup`, `pagedown`, `end`, `home`, `left`, `up`, `right`, `down` | `0x21`…`0x28` |
| contains `printscreen`, `insert`, `delete`, `help` | `0x2C`, `0x2D`, `0x2E`, `0x2F` |
| starts with `` ` `` or `~` / `-` or `_` / `=` or `+` / `[` or `{` / `]` or `}` / `\` or `\|` | `0xC0` / `0xBD` / `0xBB` / `0xDB` / `0xDD` / `0xDC` |
| starts with `;`, `:` or `semicolon` / `'` or `"` / `,` or `<` / `.` or `>` / `/` or `?` | `0xBA` / `0xDE` / `0xBC` / `0xBE` / `0xBF` |
| `f` and a digit (`f1`…) | `0x6F + n` |
| contains `numpad`, then a digit / `*` / `+` / `-` / `.` / `/` | `0x60 + d` / `0x6A` / `0x6B` / `0x6D` / `0x6E` / `0x6F` |
| anything else | 0 |

The tests run in the order of the table; "contains" matches anywhere, so the order matters (`numpadenter` is
`0x0D`).

**Message**: from a `"` to the end is dropped; then the first four space-separated tokens starting with `0x` are read
as hexadecimal: type, `d1`, `d2`, `d3` (missing ones stay 0; other tokens are skipped).

**Key code**: without a key, a lone modifier becomes the key (`control` → `0x11`, else `shift` → `0x10`, else `alt` →
`0x12`, its modifier bit cleared); with neither, the line is ignored. Then `code = vk | (up ? 0x10000000 : 0) | (shift
? 0x10000 : 0) | (control ? 0x20000 : 0) | (alt ? 0x40000 : 0)`, and `code → {type, d1, d2, d3}` is inserted unless
the code is already bound (the first line wins).

## 3. Quirks and cases

Quirks:
1. Bindings of a lone modifier (`vk` `0x10`–`0x12`) are stored, but the accelerator never matches them
   (widget-toolkit.md §13.2).
2. `s down` binds the Down arrow, not S, because `down` is read as a key name after setting key-down.
3. "Contains" matching: `downloads` would read as Down (`0x28`); `up` and `down` as the first token set the key.
4. A `"` anywhere in the message starts a comment, even inside a value.

Cases:
1. `control shift s = 0x8A2F0E1C 0x1 "save"` → code `0x00030053` → `{0x8A2F0E1C, 1, 0, 0}`.
2. `f5 up = 0x1234 0x5 0x6 0x7 0x8` → code `0x10000074` → `{0x1234, 5, 6, 7}` (the fifth value is ignored).
3. `alt = 0x99` → `vk` `0x12`, no modifier bits: stored but never matched.

## 4. Function inventory (owner `cRZWinKeyAcceleratorRes`, 17)

`0x009841FC` QI, `0x0098421E`, `0x00984226`, `0x00984241`, `0x0098422E` (slots), `0x009842EE` `ApplyTo`,
`0x0098433C` (map node), `0x0098435D` destructor body, `0x0098437D`, `0x00984385`, `0x0098438D`, `0x00984395`
(thunks), `0x0098439D` (map insert helper), `0x00984425` constructor, `0x00984452` deleting destructor, `0x0098452F`
read a line, `0x00984587` `SerialRead`. Owner `cGZWinKeyAcceleratorResFactory` (1): `0x0098446E` `CreateInstance`.
