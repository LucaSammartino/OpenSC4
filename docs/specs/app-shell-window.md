# The application's top window and SC4's framework and message-server classes (behaviour spec)

Scope: `cWinSC4App` (the top-level application window that owns the current screen), and the three SC4 subclasses of
GZ framework classes: `cSC4Framework` (`cGZFramework`), `cSC4MessageServer` (`cGZMessageServer`) and
`cSC4MessageServer2` (`cGZMessageServer2`). The GZ bases are gz-framework-core.md and framework-gz-services.md; the
screens are intro-video.md, the splash/region specs and view3d-window.md; the cheat dialog is cheat-dialog.md; the
key accelerator is widget-toolkit.md §13 and widget-accelres.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly (most of `cWinSC4App` fails to decompile).
Constants, strings and vtables were read in the PE. Written 2026-10-25 (re-appshell8).

---

## 1. `cWinSC4App` *[compat]*

### 1.1 Identity and state

A `cGZWin` (vtable `0x00A92F28`) with a message-target sub-object at `+0xD8` (vtable `0x00A92F18`; its QI thunk
`0x004E0FE0` subtracts `0xD8`). Constructor `0x004E0EE0` (one argument, the owner); the application keeps it as its
window owner (application.md `+0x50`).

| Offset | Meaning | Constructor |
|---|---|---|
| `+0xDC` | the owner (the constructor's argument) | argument |
| `+0xE4` | the graphics system (`0x00B43C9C`), held | null |
| `+0xE8` | its 3D device (QI `0xC47B747C`) | null |
| `+0xEC` | the current screen (`cIGZWin`), held | null |
| `+0xF0` | the city view (`cSC4View3DWin*`) while the city screen is current | null |
| `+0xF4` | an object set elsewhere, closed in Shutdown | null |
| `+0xF8` | the key accelerator, held | null |
| `+0x100` | a frame timer (`cRZTimer`, mode 3) | constructed |
| `+0x118` | frames per second (float) | 1.0 |
| `+0x11C` | a string (buffer `+0x120…+0x128`) | empty |
| `+0x130…+0x14C` | eight words | 1,000,000 each |
| `+0x150` | painted-frame counter | 0 |
| `+0x154` | an object released by the destructor | null |

**QueryInterface** (`0x004E08C0`): IID `0x452294AA` (`cIGZMessageTarget2`) → `this + 0xD8`, AddRef (own slot 1), true.
A null receiver writes null and then calls through the null pointer *[quirk]*. Any other IID → `cGZWin::QueryInterface`.
`View3D()` (`0x004E0960`) returns `+0xF0`.

### 1.2 Init (`0x004E1060`)

Once (the window's init flag). In order:

1. `cGZWin::Init`; caption `WinSC4App` (slot 74); id `0x6104489A` (slot 64); `SetFlag(0x800, false)` (slot 68);
   `SetArea(0, 0, W, H)` (slot 55) with W and H the main window's width and height (slot 10, then its slots 41 and 42).
2. **Keys**: the resource manager (`0x00B43CA8`) slot 3 `GetResource({0xA2E3D533, 0x6A231EAA, 0x6A9362F1}, 0x42E3EA4B,
   &r, 0, 0)`; on success create class `0x42E967BE` with IID `0xE2C1B3C4` (GZCOM slot 3) into `+0xF8` and, on success,
   `r.ApplyTo(+0xF8)` (resource slot 3, widget-accelres.md).
3. Start the frame timer.
4. The command line (framework slot 21) value of `intro` (its slot 8): when present and, lower-cased, equal to `off`,
   `CreateSplashScreen`; otherwise `CreateIntroVideoScreen`.
5. Hold the graphics system in `+0xE4` (AddRef) and query its 3D device (`0xC47B747C`) into `+0xE8`.
6. Message server 2 (`0x00B43CCC`), when present: `AddNotification(this + 0xD8, 0x2A9496C9)`.
7. The cheat code manager (`0x00B43CAC`), when present: register this as a listener (slot 5, `(this, 0)`) and the
   cheat `0xEA5EDB88` = `ShowTime` (slot 3; the string through `0x00B097F0`).
8. When the preferences (`0x00B43CD8`) byte `+0x4EE` is set, show the time display window
   (`cSC4TimeDisplayMiniWindowWinProc::DoModelessWin`, `0x0078C590`).
9. Release `r`; return true.

### 1.3 Screens

`CreateIntroVideoScreen` (`0x004E0C20`), `CreateSplashScreen` (`0x004E0CB0`), `CreateRegionScreen` (`0x004E0D40`),
`CreateCityScreen` (`0x004E0DD0`): when a screen is current (`+0xEC`), return false. Otherwise allocate and construct
the screen (`0x108` bytes `cSC4WinIntroVideoScreen`, `0x118` `cSC4WinSplashScreen`, `0x200` `cSC4WinRegionScreen`,
`0x448` `cSC4View3DWin`), hold it in `+0xEC` (AddRef), add it as a child (own slot 14 `ChildAdd`), show it (its slot
69), `SetFocusTo(screen)`, return true. For the city screen, `+0xF0` := the view itself (not held) and `+0xEC` its
window (`view + 0x0C`).

`DestroyCurrentScreen` (`0x004E0E70`): with a current screen, `ChildDelete(screen)` (own slot 16), release and clear
`+0xEC`, clear `+0xF0`, true; without one, false.

### 1.4 Messages and painting

**DoMessage(msg)** (`0x004E0A70`), by the type word `msg[0]`:

- `0x1B` with `msg[2]` = 5 (key down) or 6 (key up), `msg[3]` pointing at the key record (`+4` key, `+8` modifiers):
  `TestForAccelerator(key, modifiers, msg[2] = 5, &cmd)` (accelerator slot 10). A hit runs
  `ExecuteCommand(cmd, 0, 0)` on the command server (`0x00B43CB0`, slot 20) and returns true; a miss returns false.
  Other `msg[2]` values go to `cGZWin::DoMessage`.
- `0x230E27AC` (a cheat) with `msg[1]` = `0xEA5EDB88`: show the time display window, return true.
- `0x4348B111` (the application gained or lost focus): when the owner exists, its slot 12 with `msg[1] ≠ 0`; when
  message server 2 exists, **send** a new standard message `0x4348B111` with data1 `msg[1]` (slot 3); then return
  `cGZWin::DoMessage(msg)`.
- Anything else: `cGZWin::DoMessage(msg)`.

**Cheat dialog** (`0x004E0BB0`, called from the `+0xD8` target's DoMessage on `0x2A9496C9`): cheat-dialog.md (create
the dialog, run it modally through the window manager, destroy and release it); returns false.

**GZPaint** (`0x004E0910`): increment `+0x150`; `+0x118 := 1,000,000 / elapsed` (the timer's elapsed microseconds as an
unsigned value, `0x00A867AC`); restart the timer; return true. So `+0x118` is the rate of the last frame.

### 1.5 Shutdown and destruction

**Shutdown** (`0x004E0970`), when initialised: clear the init flag; release and clear `+0xF8`; unregister from the
cheat manager (slot 6 `(this, 0)`, slot 4 `(0xEA5EDB88)`); `RemoveNotification(this + 0xD8, 0x2A9496C9)`; when the city
view exists, shut its window (`view + 0x0C`, slot 5) and release the view (primary slot 2), clear `+0xF0`; release and
clear `+0xE4`, `+0xE8`; when `+0xF4` exists, its slot 7 with `&+0x11C`, release, clear. Then set the init flag again and
run `cGZWin::Shutdown` (which clears it). Returns true.

The destructor body (`0x004E0FF0`) reinstalls the vtables, runs Shutdown, releases `+0x154`, destroys the string and
the timer, releases `+0xEC` and runs the `cGZWin` destructor; the deleting form (`0x004E1300`) frees when flag bit 0 is
set.

## 2. `cSC4Framework` *[compat]*

Constructed (`0x00472E10`, by `0x0044C1B2`) as a `cGZFramework` with its own tables `0x00A8C970` / `0x00A8C954`. It
overrides slots 15 (`0x00473440`), 16, 17, 18, 24 and the deleting destructor (`0x004730A0`: reinstall the tables, the
base destructor, free on flag bit 0). Slot 16 is the **tick pass** (`0x004735F0`):

1. `t := +0xC0`; `+0xC0 := t + 1`; own slot 15 with `t` (`0x00473440`, the ordered-participant pass).
2. Lock (`+0xE0`, slot 1); increment the nesting count `+0xD8`.
3. When `+0xC0` is a multiple of the scheduling count `+0xBC` (gz-framework-core.md §2.1; default 4):
   `n := ++(+0xC4)`; with the iteration byte `+0xD2` set, copy the tick participants (the list at `+0xA8`, element at
   node `+8`) into a vector without references; clear `+0xD2`; unlock. Call each participant's slot 11 with `n`, in list
   order, inside a catch-all: an exception stops the loop and, after it, **sends** message `0x49A780B2` (the exception
   report, application.md) through message server 2 (`0x00B43CCC`, slot 3 `MessageSend`; not null-tested). Free the
   vector.
   Otherwise unlock.
4. When the nesting count is 1, empty the list at `+0xAC`: release each element (`+8`) and free its node.
5. Decrement the nesting count.

## 3. `cSC4MessageServer` and `cSC4MessageServer2` *[compat]*

Both are created by `AddSystemServices` (`0x0045051D`, `0x004505D3`) in place of the GZ classes. Their constructors
(`0x00472E90`, `0x00472EB0`) run the GZ constructor and install SC4 tables; the deleting destructors (`0x004730D0`,
`0x00473140`, reached through the thunks `0x00472F80` = `this − 4` and `0x00473040` = `this − 8`) reinstall them, run
the GZ destructor and free on flag bit 0.

| | `cSC4MessageServer` | `cSC4MessageServer2` |
|---|---|---|
| tables | `0x00A8CA7C`, `+0x04` `0x00A8CA64`, interface at `+0x1C` `0x00A8CA34` | interface `0x00A8CAFC`, `+0x04` `0x00A8CAC8`, `+0x08` `0x00A8CAB0` |
| overridden | interface slot 3 `MessageSend` (`0x00473320`) | interface slot 3 `MessageSend` (`0x004737E0`) |

The overrides behave as the GZ sends:

- **Generation 1** (`0x00473320`, on the interface): lock; apply pending registrations (`0x00930121`); depth `+0x44`
  up; for a non-zero type, copy that type's targets (AddRef each) into a 386-slot stack array without a bound check and
  terminate it with null; depth down; unlock; then each target's DoMessage (slot 3) followed at once by its Release.
  Type 0 reaches nobody; there is no wildcard pass (framework-gz-services.md §4A).
- **Generation 2** (`0x004737E0`): lock (`+0x20`); apply pending (`0x0092E8B5`); depth `+0x64` up; with T the message's
  type (slot 4), copy T's targets and then, when T ≠ 0, the type-0 targets into a growable vector (each AddRef'd);
  depth down; unlock; deliver to all in order; then release all and free the vector. A failure while growing the vector
  destroys it and rethrows (framework-gz-services.md §4.3).

Both return true.

## 4. Quirks

1. A null-receiver QI of `cWinSC4App` for `0x452294AA` calls through null.
2. The city view in `+0xF0` is stored without a reference but released in Shutdown; its window `+0xEC` is held
   separately.
3. The focus message is re-sent synchronously to every listener of `0x4348B111` while the window handles it.
4. The generation-1 send's snapshot is a fixed stack array.

## 5. Conformance tests

1. Command line `-intro:off` creates the splash screen; `-intro:OFF` also (lower-cased); no `intro` value creates the
   intro video screen.
2. A second `CreateRegionScreen` while a screen is current returns false and changes nothing.
3. A key-down message `{0x1B, ·, 5, &{·, 0x4E, 0}}` bound in the accelerator to command C executes C and returns
   true; with no binding it returns false.
4. GZPaint after 16,667 µs sets the rate to about 60.0.
5. Framework ticks with scheduling count 4: participants are called on the 4th, 8th, … tick with 1, 2, …; a throwing
   participant sends `0x49A780B2` once and the remaining participants of that pass are skipped.
6. Generation-2 send of type T to a target registered for T and for 0: the target receives it twice.
