# Budget window family: `cSC4BudgetUIWinProc` and the sub-budget dialog `cSC4SubBudgetWindow` (behaviour spec)

Status: first pass (re-panels4, 2026-10-21). Covers the **lifecycle, command routing and data contracts** of the two
largest SC4-specific panel owners (`cSC4SubBudgetWindow` 71 235 B, `cSC4BudgetUIWinProc` 17 952 B; ui-windows.md
§14.3). It does not re-specify the money model (budget.md) or generic widgets (the widget lane). The per-row layout
code (`initNeighborBudget` 13 933 B, `updateNeighborBudget` 14 988 B, `initOrdinanceBudget` 4 468 B, the `add_*`
helpers) is summarised by contract only; pixel layout is §8 open work.

Evidence: Ghidra export via `tools/re_query.py` (asm, callers, callees, vtable) and decomp/. REA was **not available**
in this round (no binary open in the shared session), so every claim below is from the export; the decompiler output
of `on_init` and `DoSubBudgetWindow` is stack-mangled and was checked against the instructions at the cited addresses.

## 1. Identities *[compat]*

BW17's native method/slot inventory and population/computation bodies are in
[budget-window-bodies.md](budget-window-bodies.md). These are owner algorithms, not external callbacks that
may supply populated rows or refreshed totals. Its bounded remainder is tracked by BW22, BW24 and BW25 (split from BW17; BW23 is settled).

| Item | Value |
|---|---|
| `cSC4SubBudgetWindow` primary vtable | `0x00AB5494`, 6 slots: 0 QueryInterface, 1 AddRef, 2 Release, 3 `cIGZWinProc::DoWinProcMessage` thunk, 4 `DoWinMsg` `0x0078BCA0`, 5 deleting dtor `0x0077E420` |
| `cSC4SubBudgetWindow` size | `0x140` bytes (operator new at `0x00768478` via `DoSubBudgetWindow`) |
| Dialog window object | created through GZCOM: class `0x4386D516`, interface `0x5386D516` (the generic window-generator class; the same IID is queried again in `DoSubBudgetWindow`) |
| Dialog window ID | `SetID(0x0423278F)` (`cIGZWin` slot 64, `+0x100`, at `0x00779F96`) |
| `cSC4BudgetUIWinProc` vtables | `0x00AB4A40`, `0x00AB4A2C`, `0x00AB4A1C` |
| Budget-panel tab windows | `0xAA3AC000` and `0xAA3AC001` (ShowWindow `0x00768670` shows one, hides the other by the flag at `+0x194`) |

### 1.1 Construction and observation without a modal loop (BW21) *[compat]*

`0x007862B0` is an x86 **thiscall constructor with no explicit arguments**: ECX is the address of
caller-owned storage and EAX returns that same address. It takes neither selector, period, department nor
window on the stack. The bytes move ECX to ESI at `0x007862B8`, return ESI in EAX at `0x00786523`, and
use plain RET at `0x0078652B`. Both callers agree (`0x0076847B`–`0x00768493`, `0x00786530` caller).
The four explicit arguments belong to CreateSubBudgetWindow (§3), not construction.

The normal allocation is `GlobalHeapAlloc(0x140)` (`0x005E55E0`, caller-cleans-stack), followed by the
constructor with the returned pointer in ECX. Its matching free is `GlobalHeapFree(pointer)`
(`0x005E5620`, also caller-cleans-stack). These are the executable's global heap wrappers: allocation
enters the heap at `0x00B230E8` under the lock at `0x00B230E0`; free reads the allocator tag in the
allocation prefix (`0x005E5636`–`0x005E5656`). Do not mix this storage with a foreign malloc/free pair.
The deleting destructor at `0x0077E420` takes one stack flags word (RET 4): it always performs owner
cleanup `0x0077C4C0`, and frees this storage only when flags bit 0 is set (`0x0077E428`–`0x0077E430`).
The constructor sets reference count +4 and initialized byte +8 to zero, empties the vectors, initializes
the string objects and deal-list sentinel, and sets signed-64 scale +0xB0/+0xB4 to 1. It is not a memset
of all 0x140 bytes. In particular, obtain the mode arguments through CreateSubBudgetWindow before init.
Construction already needs the native allocator and localized-string services (LTEXT
`0x6A3FF01C:0x8A5796D1`, loaded at `0x007864CB`–`0x00786508`).

There is **no window-free native calculation entry point** on this owner. Construction alone produces
no rows or totals; `on_init(null)` returns 0. The minimum path avoiding a *modal loop* is to construct,
hold a counted reference, call CreateSubBudgetWindow with the desired group selector, period and
department 0, then deliver the normal init to its real generated window (§4). The init builds the content
and child controls synchronously; ShowWindow, focus transfer, location persistence and DoModalWin in §2
are not part of the population algorithm. An off-screen observation adapter therefore still needs working
GZCOM/window factories, resources, city/budget providers and the simulator pause service. Calling
UpdateValues on a bare constructed object or replacing its content pointer with null is not this lifecycle:
the builders dereference content and existing child controls. This describes the static call boundary;
an off-screen adapter has not been run against the game in this audit.

Observe the initialized content before teardown; an admitted UpdateValues refresh can then recompute it
from public providers (§6 of the companion). Balance the one init pause with one DestroySubBudgetWindow,
let window shutdown clear its references, release the out-window reference and finally the held owner
reference (§2 and §6). The modeless end path requires a valid parent window (§6); use a functioning
window-manager parent even for an invisible adapter. Preserve the native unguarded-resume quirk on
failure rather than describing an uninitialized Destroy as a harmless cleanup.

| Observation | Native location / contract |
|---|---|
| Mode, department, period | +0x20, +0x24, +0x28 after CreateSubBudgetWindow; zero period selects monthly, nonzero selects annual in init |
| Counts and viewport | Unsigned A/B counts +0x124/+0x128; A/B offsets +0x11C/+0x120 |
| Group department cancel snapshot | Vector +0x38..+0x3C of {f32 starting fraction, counted department}; not rendered row values |
| Current slider identities | Vector +0x2C..+0x30 of {control ID, counted department}; not a vector of amounts |
| Current section amount | Signed-64 scratch +0x90/+0x94; reused for A then B, not two persistent category totals (`0x00789B93`, `0x0078A269`) |
| Rendered labels, item amounts and section totals | Children of content window +0x10, obtained by child ID (window +0x8C); inspect their text after population/refresh. Money strings are assigned to controls, not retained in a public row-result array |
| Owner string objects | +0xB8, +0xCC, +0xE0, +0xF4 contain period labels; +0x108 contains the constructor-loaded label. They are not category total strings |

The six-slot primary vtable (§1) has no row, total or display-string getter. `get_window_indiv_item`
retrieves the typed occupant property of an individual row; it does not expose group calculation results.
The existing control-ID/visibility rules (§4.1 and companion §6) remain necessary to distinguish headers,
items and totals. In particular, reading +0x90 after both passes cannot recover A's total.

## 2. Opening a sub-budget dialog (`cSC4BudgetUIWinProc::DoSubBudgetWindow`, `0x00768460`) *[compat]*

Input: one 32-bit **selector**. Only the budget panel's button handler calls it (9 call sites in `DoWinProcMessage`
`0x00768750`). Steps, in order:

1. If a sub-budget object is already held (`+0x18`), do **nothing** (no re-entry, no refresh).
2. Allocate a `cSC4SubBudgetWindow` (0x140 B), construct it (`0x007862B0`), hold one reference at `+0x18`.
3. `CreateSubBudgetWindow(out win, selector, this+0x198 value, 0)` (§3). On failure skip to step 5.
4. On success: `ShowWindow`; `SetFlag(0x800, 1)`; `PullToFront`; `SetFlag(0x800, 0)`; `SetFlag(0x100, 1)`; if the
   window answers interface `0x5386D516`, set generator flags `0x1000`, `0x2000`, `0x4000` to 1. Then
   `WinLocationSaver.RestoreWindowLocation(win, 1)` (app slot 43 service, `+0x28`), **run the window modally**
   (`DoModalWin`, window manager `+0xA4`, `0x00768598`), hand focus back to the 3D view window (manager `+0x94`), and
   `WinLocationSaver.SaveWindowLocation(win)` (`+0x18`). The dialog's position therefore persists per window ID.
   Then **Release `win`** (the `*out` reference, `0x007685DB`) and the generator interface reference (`0x007685E8`).
5. Always (also when step 3 failed): `DestroySubBudgetWindow` (§6), drop the held object (`+0x18 := null`, Release),
   then `UpdateBalanceSheetInfo` (`0x00767990`) so the main panel shows any changed funding. The modal loop's result
   (`DoModalWin`'s return) is not read.

References *[compat]* (BW6a): the sub-budget object starts at count 0 (constructor `0x007862B0` zeroes `+0x04`) and the
panel's `+0x18` is its only reference taken here, so the Release in step 5 runs the destructor unless something else
holds it. The window has two references from `CreateSubBudgetWindow`: `*out` (released by the panel in step 4) and
`+0x0C` (released by `on_shutdown` (§4) if the window was initialised, else by the destructor's base cleanup
`0x0077C4C0`, which releases `+0x1C`, `+0x18`, `+0x14`, `+0x10`, `+0x0C` in that order). On failure `*out` is null and
nothing is released by the panel.

**Nested individual dialog** (`DoIndividualBudgetWindow` `0x00786530`, from a group row `0x0ABCE6xx`, §5): the same
steps on the parent sub-budget object's `+0x1C` (re-entry: nothing if already held), with `CreateSubBudgetWindow(out,
0, parent +0x28, department)`, the same show/flag sequence and modal run, but **no** window-location restore/save,
**no** focus hand-back and **no** balance-sheet refresh; then Release `win`, the generator reference,
`DestroySubBudgetWindow`, `+0x1C := null` and Release. The parent's rows are not refreshed on return.

### 2.1 Selector table (from the panel's command IDs) *[compat]*

| Panel command (`0x287259F6` button message, value) | Selector | Dialog mode |
|---|---|---|
| `0xAA3AC401`, `0xAA3AC409`, `0xAA3AC501`, `0xAA3AC509` | `1` | ordinances (§4.1) |
| `0xAA3AC402`, `0xAA3AC40A`, `0xAA3AC502`, `0xAA3AC50A` | `2` | neighbour deals (§4.2) |
| `0xAA3AC403`, `0xAA3AC503` | `0x0A5A72D1` | department group (civic group, civic-buildings.md) |
| `0xAA3AC404`, `0xAA3AC405`, `0xAA3AC504`, `0xAA3AC505` | `0xAA369059` | group: transportation |
| `0xAA3AC406`, `0xAA3AC506` | `0x4A357B40` | group: police/fire/jail (protection.md) |
| `0xAA3AC407`, `0xAA3AC507` | `0x6A357B7F` | group: education/health |
| `0xAA3AC408`, `0xAA3AC508` | `0x4A357EAF` | group: utilities (power, water, waste) |
| `0xAA3AC40B`, `0xAA3AC50B` | `0x6A357B96` | group: parks/landmarks |
| `0xAA3AC40C`, `0xAA3AC50C` | `0xEA597195` | group: the second civic group (civic-buildings.md, department `0xEA59717A`) |

The `0xAA3AC4xx` set is the compact panel, the `0xAA3AC5xx` set the expanded one; both map identically. Every
command first passes `nsSC4TutorialSystem::FilterUICommandMessage`; a filtered command is swallowed (returns 1).

Other panel commands seen in the same handler, for completeness: `0xAA3AC400`/`0xAA3AC500` → `DisplayTaxesWindow`;
`0xAA3AC40D`/`0xAA3AC50D` → `DisplayLoanWindow`; `0xAA3AC40E`/`0xAA3AC50E` and `0x4A8879CB` set the tab byte
`+0x194 := 1` then call the proc's own slot 6, which is `ShowWindow` (`0x00768670`, §1); `0xAA3AC40F`/`0xAA3AC50F` and
`0x4A8879DB` set `+0x194 := 0` then the same call; both return 0 (BW4: `DoWinProcMessage` runs on the
`cIGZWinProc` sub-object at primary `+8`, so its `[ESI+0x18C]` at `0x00768DB8`/`0x00768DD4` is primary `+0x194`).
`ShowWindow` with `+0x194 = 1` hides tab `0xAA3AC000` and shows `0xAA3AC001`; with 0 the reverse; the shown tab also
gets `SetFlag(0x800, 0)`; then `UpdateBalanceSheetInfo` (`0x00768670`–`0x00768712`). Which tab is "compact" is not
read from the bytes; `0x0A4C3354` → `RemoveLoanWindow`; `0x2A4C33A6` / `0xAA4C339C` scroll the loan list by 2 (bounded below by 0 and
above by count − 4); `0xAA4C33E6` takes the loan (§5.2). `0x4A5BB1B5` and `0x6A5BB0FB` hide the panel (its slot 7,
`HideWindow`); `0x4A8879CC` / `0x4A8879DC` select the monthly and `0x4A8879CD` / `0x4A8879DD` the yearly period
`+0x198` (the value step 3 of §2 passes on). The taxes and loan windows are budget-window-bodies.md §9 and §10
(re-budgetwin, 2026-10-22).

## 3. `CreateSubBudgetWindow` (`0x00779E50`, thiscall, 4 args, returns bool) *[compat]*

1. `*out := null`. Create the generator (`0x4386D516` / `0x5386D516`); failure → return false.
2. `*out :=` its `cIGZWin` (AddRef); also hold it at `this+0x0C`.
3. Generator flags, in this order: `1:=1`, `2:=0`, `0x10000:=0`, `0x20:=0`, `0x40:=0`, `0x1000:=1`, `0x2000:=0`,
   `0x4000:=0`; set the generator's window procedure to **this object** (generator `+0x1C`).
4. Window flags: `SetFlag(0x100,1)`, `SetFlag(0x10000,0)`, `SetFlag(4,1)`, `SetFlag(0x800,0)`; `SetID(0x0423278F)`.
5. Store `+0x20 := selector`, `+0x28 := arg3`, `+0x24 := arg4`. Return true. `arg4` is a **department ID**, not an
   occupant (re-budgetwindow4, `initIndividualBudget` compares `+0x24` with department IDs at `0x00786815`–`0x007869EE`
   and passes it to `GetDepartmentBudget`, budget slot 28 `+0x70`, at `0x007867F6`). The panel passes 0 (group,
   ordinance and neighbour modes); `DoIndividualBudgetWindow` (`0x00786530`) passes the clicked row's department ID
   with selector 0 and the parent's `+0x28` (`0x0078658B`–`0x0078659A`). Below, "individual mode" means `+0x24 ≠ 0`.

The window is empty until the window manager delivers the init message (§4).

## 4. Window procedure `DoWinMsg` (`0x0078BCA0`) *[compat]*

| Message | Action |
|---|---|
| `0xA2BF8AD0` (init) | `on_init` (`0x007876B0`) |
| `0xA2BF8AD1` (shutdown) | `on_shutdown` (`0x0077E440`) |
| 3 / `0x287259F6` (button) | `on_button_activated(win, buttonID)` (`0x0078B120`), §5 |
| 3 / `0x887113A4` (slider moved) | §4.4 |
| 3 / `0xE8711255` (combo box changed) | neighbour mode: rewrite the selected deal rows (vector `+0x50..+0x54`, 24-byte entries), then `updateNeighborBudget` |
| 5 (key down), key `0x1B` (Esc) | close: remove the child held at `+0x14` (window `+0x3C`), drop it, `end_dialog(win, 0x42B7C353)`, return 1 |
| 5, key `0x0D` or `0x73` | the same close, **only** when modifier bit `0x4` (Alt held, §4.0) is set; otherwise ignored |

Return values *[compat]* (BW9, `0x0078BCA0`–`0x0078C1FA`): init returns `on_init`'s result; shutdown returns
`on_shutdown`'s (always 1); a button returns `on_button_activated`'s (§5); the slider message returns **0** on every
path (also when the slider lookup fails); the combo message returns 0; the closing keys return 1; an ignored key
(any other key, or `0x0D`/`0x73` without bit `0x4`) and every other message return 0. For message type 5 the key
code is the message's first data word and the modifier mask the second (`[ESP+0xCC]`, `[ESP+0xD0]`).

### 4.0 Where the key message and its modifier mask come from *[compat]* (BW5)

The GZ window manager's Win32 message translator (`FUN_0098D230`, two jump tables at `0x0098D71C`/`0x0098D748` and
`0x0098D848`/`0x0098D860`) routes **both** `WM_KEYDOWN` (`0x100`) and `WM_SYSKEYDOWN` (`0x104`) to the same key-down
builder (`0x0098D537` → `FUN_0098C400`); `WM_KEYUP`/`WM_SYSKEYUP` go to the key-up path. The builder ignores
`VK_PROCESSKEY` (`0xE5`) and does nothing while its owner's `+0x60` is non-zero; otherwise it posts message type **5**
with data words {virtual-key code (`wParam`), modifier mask, repeat count (`lParam & 0xFFFF`)}. The mask comes from
`FUN_0098B6E0(lParam)`, which samples `GetKeyState` (high bit = down) at translation time:

| Bit | Meaning | Source |
|---|---|---|
| `0x1` | Shift down | `GetKeyState(VK_SHIFT 0x10)` |
| `0x2` | Ctrl down | `GetKeyState(VK_CONTROL 0x11)` |
| `0x4` | **Alt down** | `GetKeyState(VK_MENU 0x12)` (`0x0098B702`–`0x0098B70A`) |
| `0x10` / `0x20` / `0x40` | left / right / middle mouse button down | `GetKeyState(1 / 2 / 4)` |
| `0x40000000` | key was already down (auto-repeat) | `lParam` bit 30 |

The mouse-message helper `FUN_0098B750` uses the same layout (`MK_SHIFT`→1, `MK_CONTROL`→2, Alt via
`GetKeyState(VK_MENU)`→4, buttons→`0x10`/`0x20`/`0x40`). A **delivered type-5 message** therefore closes the
dialog for Alt+Enter and Alt+F4 (other modifier bits may also be set), and plain Enter/F4 do nothing.

**BW11: delivery, default processing and the runtime boundary (2026-10-26).** Before dispatching the Win32
message, the translator traverses its hook list in list order. Each hook's slot 3/+0x0C receives
`(HWND, message, wParam, lParam, &consumed)`, with `consumed` reset to false before each call. A true output
immediately returns that hook's result: later hooks and the translator's own switch do not run. A false
output advances to the next hook even when the hook's return value is nonzero. This is the actual gate at
`0x0098D29D`–`0x0098D302`, not a test of the dialog's handled result.

If no hook consumes `WM_SYSKEYDOWN`, its key-down path calls the builder and **returns Win32 result 0**
(`0x0098D537`–`0x0098D554`), regardless of whether a dialog later handles the generated message. There is
no fall-through to `DefWindowProcA` on that path, even when the builder suppresses input because +0x60 is
nonzero. Thus default system processing of this particular delivered key-down is not an additional action
performed by this translator. The dialog's close body (`0x0078BCFE`–`0x0078BD40`) removes/releases its popup,
calls `end_dialog` with `0x42B7C353`, and returns 1; it contains neither a display-mode change nor an application
quit request. These local rules do not exclude effects of an earlier hook, accelerator, or a separate queued
message. In particular, a separately delivered `WM_CLOSE` (`0x10`) takes `0x0098D46E`–`0x0098D482`, which
calls `0x0098C590` to submit a **type-10** message (`0x0098C5B7`–`0x0098C5C9`); it is not the type-5 close.

Worked delivery cases for an engine test:

| Input / hook result | Required local result |
|---|---|
| `WM_SYSKEYDOWN`, F4, Alt sampled down; all hooks leave consumed=false; builder enabled; type 5 delivered to dialog | Popup removed once if present, dialog ends with `0x42B7C353`, dialog result 1, translator result 0; no default-window-procedure call from this path |
| Same input, first hook returns 7 with consumed=false, second returns 9 with consumed=true | Translator returns 9; no builder or dialog dispatch by the translator; the first return value does not stop traversal |
| Same unconsumed input, builder +0x60 nonzero | No type-5 submission, translator still returns 0 |
| Delivered type 5, Enter, mask `0x5` / mask `0x1` | First closes (Alt+Shift); second returns 0 with no close |

**Runtime observation still required:** use the unmodified Windows 1.1.641 game, record its windowed/fullscreen
state, open a sub-budget with no popup, and press/release Alt+Enter once. Reopen it before pressing/releasing
Alt+F4 once; do not reuse the already closed dialog. For each trial record whether the sub-budget disappears,
whether display mode changes, and whether a quit/save prompt or application termination follows. To attribute
an otherwise invisible quit attempt, trace entry to `cSC4App::RequestQuit` (`0x004546B0`) and the ordered
`WM_SYSKEYDOWN`/`WM_SYSCOMMAND` (`0x112`, `SC_CLOSE=0xF060`)/`WM_CLOSE` messages, hook consumption,
type-5 delivery and `end_dialog`. Record any action on key release as well. A UI-only observation establishes
visible behavior but cannot exclude a rejected quit request. Repeat with popup +0x14 present to distinguish its
removal from closing only the popup. The unresolved result is precisely which of these paths the live input
route takes and whether another action accompanies the close; no game run or "dialog only" verdict is claimed.

`on_init(win)` (`0x007876B0`): `win` null → return 0 and do nothing. If the initialised byte `+0x08` is already set →
return 1 and do **nothing** (no second pause). Otherwise take the hidden pause, build the mode, set `+0x08 := 1` (at
the end of each mode branch: `0x00787C4E`, `0x00787D69`, `0x007882B0`, `0x00789777`) and return 1.

**Content window `+0x10`** *[compat]* (BW10a, `0x00787A9E`–`0x00787BD5`): after the pause and the string loads and
**before** the selector branch (so in every mode), `on_init` asks GZCOM for a new object of class `0x4386D516`,
interface `0x5386D516` (the same generic window class as the dialog, §1). If that fails, `+0x10` stays null and
only the rest of this block is skipped (see "Failed content creation" below). Otherwise `+0x10 := ` the object's `cIGZWin` (generator slot 3 `+0x0C`), stored with a
reference (`cRZAutoRefCount<cIGZWin>::operator=`); then, in this order, generator flags `1:=0`, `2:=0`, `0x10000:=0`,
`0x20:=0`, `0x40:=0`, `0x1000:=1`, `0x2000:=1`, `0x4000:=1` (generator slot 6 `+0x18`); window flags `0x100:=1`,
`0x10000:=1`, `0x200000:=0`, `0x800:=0`, `0x8000:=0`, `4:=1` (`SetFlag`, slot 68 `+0x110`); `SetID(0x0423278E)`
(slot 64 `+0x100`); and `win.ChildAdd(+0x10)` (dialog slot 14 `+0x38`). The temporary generator reference is then
released (`0x00787BE4`). `+0x10` is the container the mode builders lay their background segments and rows into
(`SetupWindow` and the other populate routines read it). `on_shutdown` and the destructor release it.

**Failed content creation** *[compat]* (BW13): `CreateInstance` (GZCOM slot 3 `+0x0C`, at `0x00787ACD`) returning
false jumps (`JZ` at `0x00787AD2`) to `0x00787BDA`, the common tail of the block: release the generator out-pointer
if it is non-null (`0x00787BDA`–`0x00787BE4`), then fall into the selector branch at `0x00787BE7` exactly as on
success. So on failure: no `+0x10` assignment, no generator or window flags, no `SetID`, no `ChildAdd`; the hidden
pause taken earlier (`0x007878C0`) stays held; the selector branch runs; each branch still sets `+0x08 := 1` and
`on_init` returns 1 (the paths of the first paragraph). There is no null check of `+0x10` anywhere in `on_init`.
Every mode builder passes `+0x10` as the parent window (first argument) to `add_text` (`0x00779660`), for example
group mode at `0x0078837B`/`0x0078839F`, `initOrdinanceBudget` at `0x0077C90E`, `initNeighborBudget` at
`0x0077F59B`, `initIndividualBudget` at `0x00786C88`, and `add_text` dereferences that parent unconditionally (reads
its vtable at `0x0077982B`, then calls its slot 14 `+0x38`, `ChildAdd`). The native program therefore faults on a
null read at the first such `add_text` call; it has no recovery path, so whether the remaining steps ever execute
is undefined in the original. An implementation must not fault: it follows the control flow above (skip the block,
keep the pause, run the mode branch, set `+0x08`, return 1) and treats every child insertion into a missing `+0x10`
as a no-op. This is a deliberate *[divergence]* from the native crash, not observed behaviour.

`on_shutdown(win)` (`0x0077E440`, BW9): if `+0x08` is 0 → return 1 with no effect. Otherwise `+0x08 := 0`; empty the
vectors `+0x2C` (slider rows), `+0x38` (group restore pairs), `+0x44` (individual restore records), `+0x50` (deal
rows), `+0x5C`, `+0x68` (held deals; emptied without a per-deal Release), `+0x74` (ordinance row map); free the
ordinance ID array `+0xA4` (then `+0xA4 := null`) and the byte-pair array `+0xA8` (the pointer is **not** cleared,
and the count `+0xA0` is not reset); AddRef itself; Release `+0x0C` and `+0x10` (each nulled first); Release itself;
return 1. It does not touch the pause. After it, `DestroySubBudgetWindow` finds `+0x0C` null and skips `end_dialog`.

`on_init` first takes a **hidden simulator pause** (`cISC4Simulator::HiddenPause`, slot 10, at `0x007878C0`);
`DestroySubBudgetWindow` releases it (`HiddenResume`, slot 13; the pause is a counter, see §6). The date does not advance while any sub-budget dialog
is open. It then branches on the selector (`+0x20`, `0x00787BE7`):

- **1 → ordinances**: load the background image (type `0x856DDBAC`, group `0x46A006B0`, instance `0x140155F2`), then
  `initOrdinanceBudget` (`0x0077C660`): one row per ordinance with an on/off check, cost text; the original on/off
  state and the edited state are kept as byte pairs (`+0xA8`, count `+0xA0`).
- **2 → neighbour deals**: image instance `0x140155D2`, then `initNeighborBudget` (`0x0077E600`), which calls
  `NeighborsSimulator.SetUIDisableFlag(true)` and `ProposeDeals(list, true)` (neighbors.md §7).
- **`+0x24` non-null → one occupant** (`initIndividualBudget`, `0x00786690`).
- **otherwise → a department group**: `budget.GetDepartmentBudgetsInGroup(selector, list)` (budget slot 31,
  `+0x7C`), one slider row per department (`SliderInfo` = {slider window ID, department}, vector `+0x2C..+0x30`), the
  starting funding fraction of each department saved (vector `+0x38..+0x3C`, 8 bytes each), totals with
  `SC4MakeMoneyString`. Groups `0x6A357B96`, `0xEA597195`, `0x0A5A72D1` differ only in which department header rows
  are shown (§4.1).
- **Order and the empty group** *[compat]* (BW10b): `+0x38` is emptied (`0x00787F8B`) and then filled **inside the
  row-counting loop**, one pair per department returned by `GetDepartmentBudgetsInGroup`, before any visibility test:
  {the f32 read through department slot 10 (`+0x28`, which returns a pointer), the department with a reference}
  (`0x00787FDB`–`0x00788045`). The reference ownership is given under "Department references in the saved pairs"
  below (BW14). The empty test comes after the loop (`0x0078822C`–`0x00788242`): when both row counts
  `+0x124` and `+0x128` are 0, `on_init` calls `SetEmptyWindow` (`0x0077BEC0`, which touches neither vector) and
  skips the slider rows: `+0x2C` stays empty, but `+0x38` **keeps** one saved pair per group department (a group
  whose departments are all hidden still has pairs; only a group with no departments has none). A later
  `RestoreStartSettings` on such a window writes each saved fraction back unchanged. The empty branch also creates no
  scroll arrows (they are built at the end of the non-empty branch, below).

**Department references in the saved pairs** *[compat]* (BW14): the department half of each `+0x38` pair is a
counted reference (`cRZAutoRefCount`-style, `cIGZUnknown` slot 1 `+0x04` AddRef, slot 2 `+0x08` Release). Per
department, in the row-counting loop:

| Step | Address | Department refcount |
|---|---|---|
| a temporary pair {f32, department} is built; the department is AddRef'd if non-null | `0x00787FE7`–`0x00787FF9` | +1 |
| append to `+0x38`, fast path (spare capacity): element copied, department AddRef'd if non-null | `0x0078800D`–`0x00788026` | +1 |
| append, grow path (`FUN_0077D7E0`, at `0x00788040`): the new element and every moved old element are copy-constructed with AddRef (`FUN_0077AC20`/`FUN_0077AC60`), then every old element is Released and the old block freed (`0x005FF0C0`, at `0x0077D89B`) | `0x0077D7E0`–`0x0077D8B0` | new +1, old elements net 0 |
| end of the iteration: the temporary pair's department is Released if non-null | `0x007881AC`–`0x007881BD` | −1 |

Net: each pair in `+0x38` owns exactly one reference to its department. Every path from the top of the loop
(`0x00787FB0`) reaches the Release at `0x007881AC` before the back edge (`0x007881D8`). The references are released:

- when `+0x38` is emptied: the erase-range routine `0x0077C420` (called with begin and end) Releases the department
  of every removed element (non-null only, `0x0077C450`–`0x0077C463`) and sets end := begin; capacity is kept. Its
  callers for `+0x38` are `on_init` (`0x00787F8B`, before the loop; the vector is already empty there in practice,
  because `on_init` runs only with `+0x08` clear, i.e. on a fresh window or after `on_shutdown`) and `on_shutdown`
  (`0x0077E46F`);
- by the destructor (`~cSC4SubBudgetWindow` `0x0077E420` → `0x0077C4C0`, at `0x0077C592`): `0x005FF0C0` Releases the
  department of every element still present and frees the storage. After an `on_shutdown` the vector is empty, so
  the destructor releases nothing more; a window destroyed without `on_shutdown` releases its pairs here.

No other code path writes `+0x38` (callers of `0x0077C420`; `RestoreStartSettings` only reads it).

**Slider rows `+0x2C`: what a `SliderInfo` owns** *[compat]* (BW15, byte-audited): an element is 8 bytes,
{`+0` the slider's 32-bit window ID, `+4` the department, a counted reference}. The ID is a plain integer: the
element holds **no** reference to the slider window. The slider is made by `FUN_007794E0` (thiscall, 9 stack
arguments, `RET 0x24`): it creates the slider through the window factory (`RZ::WCM`), configures it, inserts it with
`ChildAdd` (slot 14 `+0x38`, at `0x00779585`) into its **first argument**, which at both call sites is the dialog
`win` itself (the `on_init` argument, `PUSH EBP` at `0x00788D24` and `0x00789173`), not the content window
`+0x10`, and then Releases its own creation reference (`0x0077958C`). The slider is therefore owned by the dialog's
child list, and `§4.4` finds it again by ID. The window ID passed to `FUN_007794E0` is the same value stored in the
element.

Only two places append to `+0x2C`, both in `on_init`'s group branch, both in the expense-list (B) build loop
(`0x00788A30`–`0x00789282`), both keyed to the **department** of the current outer iteration (the department list
from `GetDepartmentBudgetsInGroup`, element `[EBX]`), never to a line item. The income-list (A) loop creates no
sliders. The only append routine is `FUN_0077E550` (push_back, callers `0x00788D6E` and `0x007891BD` only); nothing
else in `on_init` writes `+0x2C..+0x34`.

| Step | Address (header row / item row) | Department refcount |
|---|---|---|
| temporary `SliderInfo` {ID, null} built, then the department stored and AddRef'd if non-null | `0x00788D39`–`0x00788D5C` / `0x00789188`–`0x007891AB` | +1 |
| push_back fast path (end ≠ capacity): ID and department copied, department AddRef'd if non-null, end += 8 | `0x0077E553`–`0x0077E576` | +1 |
| push_back grow path (end = capacity): `FUN_0077D7E0` with count 1 (`0x0077E58F`), the same grow routine as `+0x38` (BW14 table): new element +1, moved old elements net 0 | `0x0077E57E`–`0x0077E594` | +1 (net) |
| end of that row's block: the temporary's department Released if non-null | `0x00788EC2`–`0x00788ED6` / `0x00789241`–`0x00789263` | −1 |

Every path from each push_back reaches its Release before the loop's next iteration (all branches between
`0x00788D6E` and `0x00788EC2`, and between `0x007891BD` and `0x00789241`, jump forward inside that range). Net:
**each `+0x2C` element owns exactly one reference to its department.** A department can own several elements:
one for its B header row and one per shown B item row, within the 9 B rows that are built (§4.1; the row counter
`[ESP+0x14]` is shared by all departments of the group, and a header that would be the 10th row gets no slider,
path `0x00788EDB`). A department with no shown expense row has no `+0x2C` element.

When the window's references are taken and dropped, in order:

| Event | `+0x2C` | `+0x38` |
|---|---|---|
| `on_init`, group mode, `+0x08` clear | emptied at `0x00788376`, **only in the non-empty branch** (after `SetupWindowBig`, before the first row), then filled by the B loop | emptied at `0x00787F8B`, then filled in the counting loop (BW14) — so `+0x38` is filled **before** any `+0x2C` element |
| `on_init`, empty group branch | untouched (stays empty) | filled (BW14) |
| `on_init` with `+0x08` set | untouched | untouched |
| `on_shutdown` with `+0x08` set | erased first (`0x0077E45F`) | erased second (`0x0077E46F`) |
| destructor `0x0077C4C0` | released **second** (`0x0077C59A`) | released **first** (`0x0077C592`) |

Erasing (`0x0077C420`, called with begin and end) Releases the department of each removed element front to back
(`0x0077C450`–`0x0077C463`), skipping nulls, and sets end := begin, keeping the capacity. The destructor's
`0x005FF0C0` Releases each remaining element's department front to back (`0x005FF0D0`–`0x005FF0E1`), then frees the
block. In the destructor both vectors are released after `+0x50` and `+0x44` (`0x0077C582`, `0x0077C58A`) and
before the interface references `+0x1C`, `+0x18`, `+0x14`, `+0x10`, `+0x0C` (`0x0077C59F`–`0x0077C5DD`).

Failed content creation (BW13) and `+0x2C`: in the native program the first `add_text` into the missing `+0x10`
(`0x0078837B`, right after the `+0x2C` erase at `0x00788376`) faults before any slider is appended, so at the fault
`+0x2C` is empty and `+0x38` already holds its pairs. Under the no-fault rule of BW13 the sliders are still
created (their parent is `win`, not `+0x10`) and appended exactly as on success, with the same counts.

### 4.1 Group mode: the two lists and the special groups *[compat]* (BW1, BW7a)

The group dialog has two scrolling lists, each showing at most **9** rows:

| List | Rows | Row count | Scroll position | Arrow buttons |
|---|---|---|---|---|
| A | **income** line items (`GetType() == 1`) | `+0x124` | `+0x11C` | `0x451` up, `0x452` down |
| B | **expense** line items (every other type) | `+0x128` | `+0x120` | `0x453` up, `0x454` down |

`on_init` counts the rows first (`0x00787FB0`–`0x007881D8`). For each department of the group, in
`GetDepartmentBudgetsInGroup` order, it fetches the department's line items (department slot 21 `+0x54`) and walks
them in order:

1. The **first** income item of a department adds one **header row** to A; the first expense item adds one header row
   to B. In the three special groups (`0x6A357B96` parks/landmarks, `0xEA597195`, `0x0A5A72D1` the civic groups) the A
   header is added only if the department's `GetTotalIncome()` (department slot 16 `+0x40`) is non-zero and the B
   header only if its `GetIdealMonthlyFunding()` (slot 14 `+0x38`) is non-zero (tests at `0x0078807C`–`0x007880AB`
   and `0x007880F7`–`0x00788123`). Other groups always add the header. A department without income items has no A
   header in any group.
2. Each item whose `GetDisplayFlags()` (line item slot 17 `+0x44`) has bit 0 (ShowLineItem) adds one row to its list
   if `GetID() <= 100` (line item slot 3, unsigned compare with `0x64`) **or** its amount is non-zero:
   `GetIncome()` (slot 14 `+0x38`, 64-bit) for A, `GetFullExpenses()` (slot 10 `+0x28`, 64-bit) for B. Items with
   ID > 100 and a zero amount are hidden; an item with ID ≤ 100 is shown whatever its amount. (BW16, settled from
   bytes: at all four sites, count A `0x007880D3`, count B `0x00788147`, build A `0x007886ED`, build B `0x00788F33`,
   the ID is compared with `0x64` and `JBE` jumps to the "shown" path; only the fall-through (ID > 100) reaches the
   amount test, whose zero result jumps to "hidden". In the count loops the flag test comes first, in the build loops
   the ID/amount test comes first; both conditions must hold, so the order changes nothing.)

   Slot 3 is the line item's ID. The element pointers come from the department's line-item map, whose values are
   `cSC4LineItem` objects at their primary interface (vtable `0x00AB0F58`); slot 3 there is `0x006AAEB0`, which
   returns the dword at object `+0x54`. `Read` (`0x006A9B60`, called through the serializable interface at object
   `+4`, so its `+0x50` is object `+0x54`) loads it from the save; otherwise it is set by the department's line-item
   factory (department slot 17, `FUN_006AB040`), which stores its first argument at `+0x54` (`0x006AB076`) and uses
   the same value as the key of the department's map at `+0x4C` (`0x006AB0A8`). For comparison, slot 4 (`+0x10`,
   `GetType`, the `== 1` income test at `0x0078806C`) returns `+0x64` and slot 17 returns the flags at `+0x58`
   (default 5 from the constructor `FUN_006AAE20`).

The same header predicate is applied again when the rows are built (`0x007884B8`, `0x00788A6E`) and on every refresh
in `UpdateValues` (`0x00789C04` for A, `0x0078A2E8` for B), so a header that was hidden stays hidden.

After counting: the background image is instance `0x140155D2` (as the neighbour mode). If **both** counts are 0, the
dialog becomes the empty window (`SetEmptyWindow`, `0x0077BEC0`, with the LTEXT group `0x6A3FF01C` instance
`0x4A5CB4B6`) and no list is built. Otherwise `SetupWindowBig(win, min(countA, 9), min(countB, 9))` (`0x0077A6F0`)
sizes the dialog, and at most 9 rows of each list are created; the rest are reached by scrolling (§5.3).

Worked example (group `0x6A357B96`): department X has an income item ID 5 with income 0 and an income item ID 200
with income 0, and `GetTotalIncome() = 0`; department Y has one expense item ID 7, full expenses 300, ideal funding
300. A: X adds no header (special group, total income 0); item 5 is shown (ID ≤ 100, amount ignored); item 200 is
hidden (ID > 100, amount 0). countA = 1. B: Y adds a header and item 7: countB = 2. In group `0x4A357EAF` the same
data gives countA = 2 (header always added). If item 200 had income 50 instead, it would be shown too (countA = 2,
or 3 in group `0x4A357EAF`).

### 4.4 Slider moved (`0x887113A4`)

Order (`0x0078C067`–`0x0078C17F`): get the slider child of the window by the message's control ID through the
window's interface lookup (window slot 37 `+0x94`, IID `0x21325207`). If that fails, nothing else happens. Then the
process-wide re-entry byte (`0x00B4DFC8`): if it is set, nothing happens; otherwise set it, do the work below, clear
it. The **position** is the slider's 32-bit signed integer value (slider slot 4 `+0x10`).

- Group mode (`+0x24 = 0`): for **every** `SliderInfo` whose window ID equals the control ID (no early exit), call
  that department's `SetFundingPercentage(&p, 0)` (department slot 11 `+0x2C`, by reference; budget.md §7.1, which
  redistributes) with `p = f32(position) × 0.01f` rounded **once** to f32 (`FILD`, `FMUL float [0x00A823A4]` =
  `0x3C23D70A`, `FSTP float`, `0x0078C12F`–`0x0078C142`). The product of a small integer and the f32 constant is
  exact on the x87, so this is the IEEE f32 product `position as f32 * 0.01f32`. It is **not** the f64 product
  rounded to f32: of positions 0–200, 59 differ (5, 9, 10, 15, 18, 20, …); position 30 gives `0.29999998f`
  (`0x3E999999`), not `0.3f` (`0x3E99999A`). Then `UpdateValues(win)` (`0x007898A0`) once.
- Individual mode: `item := get_window_indiv_item(win, controlID)`; if null, skip to clearing the byte. Otherwise
  `SetOccupantFundingLevel(item, which, level)` with `which = 0` for control IDs below `0x0ABCE500` and `1` from
  `0x0ABCE500` up (unsigned), and `level = f32(position)` **unscaled** (percent; the 0.01 is applied inside, §5.5);
  then `updateIndividualBudget(win, 0)` (`0x00785740`).

`get_window_indiv_item(win, id)` (`0x00779600`): the child window with that ID (window slot 35 `+0x8C`); its
property `0x4BFEE088` (window slot 113 `+0x1C4`); if the property's type (slot 3) is `0xF`, the object it holds
(slot 51 `+0xCC`), else null. The property is released; the returned object is not AddRef'd.

## 5. Buttons (`on_button_activated`, `0x0078B120`) *[compat]*

Signature `bool on_button_activated(cIGZWin* win, uint32 id)` (thiscall, `RET 8`). Dispatch order
(`0x0078B120`–`0x0078B67E`): `id ≤ 0x384` goes through a jump table on `id − 0x67` (bytes `0x0078BC28`, targets
`0x0078BC08`); above that the explicit compares; every ID not matched below returns **0 with no refresh**
(`0x0078BBF9`). "Close the child" means: if `+0x14` is non-null, `win.RemoveChild(+0x14)` (window slot 15 `+0x3C`),
then `+0x14 := null` and Release it. Every `end_dialog` call here passes `(win, 0x42B7C353)` (§6).

| Button ID | Action, in order | Returns |
|---|---|---|
| `0x67`, `0x42B7C353` | **OK**: `end_dialog` (keeps all edits) | 1 |
| `0x6D` | **Cancel**: `RestoreStartSettings(win)` (§5.1), then `end_dialog` | 1 |
| `0x68`, `0x384` | close the child (the ordinance info popup, §5.6); the dialog stays | 0 |
| `0xCC`, `0x385` | `end_dialog` | 1 |
| `0xCD` | ordinances **apply**: for `i` in `0..count(+0xA0)`, if the edited byte differs from the original byte, `OrdinanceSimulator.SetOrdinanceOn(ordinanceID[i] (+0xA4), edited)` (global `0x00B43D90`, slot 13 `+0x34`; the edited byte zero-extended); then close the child; `end_dialog` | 0 |
| `0xCE` | close the child; `end_dialog` | 0 |
| `0xCF` | neighbour **accept**: `ApplyDealEdits` (`0x0077AB40`, §5.4); close the child; `end_dialog` | 0 |
| `0x451`, `0x452`, `0x453`, `0x454` | scroll (§5.3) | 0 |
| `0x0ABCE600`–`0x0ABCE6FF` | `item := get_window_indiv_item(win, id)`; null → return 0. Else open the **nested** individual dialog for the department `item.GetID()` (slot 3) through `DoIndividualBudgetWindow` (`0x00786530`), which builds a second `cSC4SubBudgetWindow` held at `+0x1C` | 0 |
| `0x0ABCDE00`–`0x0ABCDEFF`, individual mode only | **camera jump** to the row's building (§5.7) | 0 |
| `300`–`499` (`0x12C`–`0x1F3`) | ordinance row check box: `edited[id − 300] :=` the check box state (§5.6) and `updateOrdinanceBudget(win)` | 0 |
| `500`–`699` (`0x1F4`–`0x2BB`) | open the ordinance info popup for ordinance `id − 500` (§5.6) | 0 |

There is **no** common refresh tail: only the scroll buttons refresh (§5.3); `0xCD`/`0xCE`/`0xCF` end the dialog without
a refresh, and `UpdateValues` runs from a button only on the group-mode scroll path (`0x0078BBF4`).

### 5.1 `RestoreStartSettings` (`0x0077C5E0`)

Takes the window (`RET 4`) but does not use it. The branch is on `+0x24` (individual mode), not on the selector.
Group mode (`+0x24 = 0`): for every saved {fraction f32, department} pair (8 bytes, vector `+0x38..+0x3C`), in vector
order, call that department's `SetFundingPercentage(&fraction, 0)` (slot 11). Individual mode: for every
`IndividualRestoreInfo` (12 bytes: occupant pointer, level0 **f32**, level1 **f32**; vector `+0x44..+0x48`) call
`SetOccupantFundingLevel(occupant, 0, level0)` then `(occupant, 1, level1)` (`0x0077C5F6`–`0x0077C611`); the levels
are passed as raw 4-byte floats, in percent like the slider path. Ordinance and neighbour edits are not applied
until `0xCD` / `0xCF`, so Cancel needs no restore for them. In ordinance and neighbour modes the group branch runs
over an empty vector.

### 5.2 Loan taken (`0xAA4C33E6`, budget panel)

*Rewritten 2026-10-22 (re-budgetwin).* Reads the chosen amount (primary `+0x190`; the handler's `+0x188` is the `+8`
view), builds a `LoanInfo` record from the bond terms (`GetFullCostOfLoan`, budget slot 49 `+0xC4`;
`GetLoanTimeInMonths`, slot 47 `+0xBC`) and appends it to the panel's loan list (`0x00692110`); then the amount added
`+0x18C` grows by the amount and the count `+0x184` by one, `UpdateLoanDisplay` redraws the list, **`IssueNewLoans`**
(`0x00764BE0`; corrected: not a refresh) issues every loan added since the window opened (`IssueBond`, slot 40), and
`RemoveLoanWindow` closes the window (budget-window-bodies.md §10). The bond arithmetic is budget.md §5.4.

### 5.3 Scrolling (`0x451`–`0x454`) *[compat]* (BW7)

`pos` is the list's position (A `+0x11C`, B `+0x120`), `len` its row count (A `+0x124`, B `+0x128`), all u32.
"Show/hide arrow X" is `win.GetChild(X)` (slot 35 `+0x8C`) then `SetFlag(1, on)` (window slot 68 `+0x110`; flag 1 =
visible), with no null check of the child.

| Button | Step 1 (move) | Step 2 | Step 3 |
|---|---|---|---|
| `0x451` A up | if `pos > 0`: `pos −= 2` | if `pos == 0`: hide `0x451` | if `pos + 9 < len`: show `0x452` |
| `0x452` A down | if `pos + 9 < len`: `pos += 2` | if `pos + 9 ≥ len`: hide `0x452` | show `0x451` **unconditionally** |
| `0x453` B up | as `0x451` on B | hide `0x453` at 0 | show `0x454` if `pos + 9 < len` |
| `0x454` B down | as `0x452` on B | hide `0x454` | show `0x453` unconditionally |

So the largest position is the first even value with `pos + 9 ≥ len` (for `len = 12`: 0 → 2 → 4, and 4 shows rows
4–12, one past the end; the row builders skip missing rows). A down press on a list that fits (`len ≤ 9`) does not move
but **shows** the up arrow. Nothing hides the arrows at open time in this routine; `on_init` sets their initial state.

**Initial arrow state** *[compat]* (BW10c, group mode, `0x007895A0`–`0x0078973A`): at the end of the non-empty group
branch `on_init` creates the four arrows with `add_image_button2` (up arrows image `0x140155CB`, down arrows
`0x140155CC`), sets each one's notification target to the dialog (child slot 86 `+0x158`), then:

| Arrow | Initial state |
|---|---|
| `0x451` A up | hidden, always |
| `0x452` A down | hidden when `+0x124 ≤ 9` (unclamped count, `0x007896C9`); otherwise left as created |
| `0x453` B up | hidden, always |
| `0x454` B down | hidden when `+0x128 ≤ 9` (`0x00789714`); otherwise left as created |

Hiding is `GetChild(id)` then `SetFlag(1, 0)`, as above. After this `+0x08 := 1` (`0x00789777`). The empty-group
branch (§4, BW10b) creates no arrows at all.

**Selector-specific initial arrow state** *[compat]* (BW12). The ordinance helper (selector 1,
`initOrdinanceBudget` `0x0077C660`, called once from `on_init` at `0x00787C42`) and the neighbour helper (selector 2,
`initNeighborBudget` `0x0077E600`, called at `0x00787D5D`) end with their own copy of the sequence: the same four
`add_image_button2` calls, same IDs and images, each followed by setting the notification target, then the hides. Both
read the same fields, `+0x124` (list A count) and `+0x128` (list B count), as unsigned 32-bit values compared with the
constant 9. Up arrows `0x451` and `0x453` are hidden unconditionally in both. The down-arrow bounds differ:

| Helper | `0x452` hidden when | `0x454` hidden when | Addresses (compare, branch) |
|---|---|---|---|
| group mode (`on_init`) | `+0x124 ≤ 9` | `+0x128 ≤ 9` | `0x007896D4` `JA`; `0x00789714` `JA` |
| ordinance (selector 1) | `+0x124 < 9` (≤ 8) | `+0x128 < 9` (≤ 8) | `0x0077D73E` `JAE`; `0x0077D77C` `JAE` |
| neighbour (selector 2) | `+0x124 ≤ 9` | `+0x128 ≤ 9` | `0x00781BEA` `JA`; `0x00781C28` `JA` |

The ordinance helper is the odd one: it branches past the hide on `count ≥ 9` (carry clear), so with exactly 9 rows
its down arrow stays as created even though every row fits; pressing it then does not move (`pos + 9 < len` is
false), hides the down arrow (step 2, `pos + 9 ≥ len`) and shows the up arrow (step 3, unconditional), as in the table
above. Where the counts come from: the ordinance helper only increments them, one per ordinance, into A or B by the
ordinance's own test (`0x0077C734`/`0x0077C73C`), starting from the zeros the constructor writes (`0x007864AE`,
`0x007864B4`) and not changing them later in the helper. The neighbour helper sets both to 0 itself (`0x0077E6A6`,
`0x0077E6AC`) and then adds one per row plus one for the first row of each section, so its counts include those
section header rows. In both, the hide is `GetChild(id)` then `SetFlag(1, 0)` with no null check; the helpers do not
write `+0x08`, `on_init` sets it to 1 after each returns (`0x00787C4E`, `0x00787D69`).

Worked boundary cases (count shown applies to either list; A drives `0x452`, B drives `0x454`):

| Count | group mode down arrow | ordinance down arrow | neighbour down arrow |
|---|---|---|---|
| 0 | hidden | hidden | hidden |
| 8 | hidden | hidden | hidden |
| 9 | hidden | **as created** | hidden |
| 10 | as created | as created | as created |
| `0x80000000` (unsigned, not negative) | as created | as created | as created |

Refresh after the move (no `UpdateValues` unless stated):

| Button | Selector 1 | Selector 2 | otherwise |
|---|---|---|---|
| `0x451`, `0x452` | `updateOrdinanceBudget(win)` | `updateNeighborBudget(win)` | `UpdateValues(win)` (also in individual mode, `0x0078B3EF`) |
| `0x453`, `0x454` | `updateOrdinanceBudget(win)` | `updateNeighborBudget(win)` | individual mode: `updateIndividualBudget(win, 0)`; group mode: `UpdateValues(win)` |

### 5.4 `ApplyDealEdits` (`FUN_0077AB40`, button `0xCF`) *[compat]* (BW2)

For each entry of the held-deal vector (`+0x68..+0x6C`, 8 bytes, the deal pointer at `+4`), in order, on the deal
`d` (deal slots per neighbors.md §5.4, SDK order; checked by body):

1. AddRef `d` (skipped if null; the calls below are not guarded).
2. If `d.GetFlags()` (slot 36) has **Active** (`0x2`):
   - if `d.GetLastAmount()` (slot 12) ≠ 0: `d.SetAmount(d.GetLastAmount())` (slot 11), then
     `d.SetIncAmount(d.GetLastIncAmount())` (slots 17, 14);
   - else: `d.Cancel()` (slot 34), `d.SetLastAmount(0)` (slot 13), `d.SetLastIncAmount(0)` (slot 15).
3. Then, reading the flags **again**: if **Cancelled** (`0x4`) is set and `d.GetLastAmount() ≠ 0`: `d.Activate()`
   (slot 35; its result is ignored), `d.SetAmount(d.GetLastAmount())`, `d.SetIncAmount(d.GetLastIncAmount())`.
4. Release `d`.

The dialog uses a deal's lastAmount / lastIncAmount as the **edited** amount: the combo-box handler writes them, and
this routine commits them. Order matters: an active deal edited to 0 is cancelled in step 2 and its lastAmount is
cleared, so step 3 does not revive it. `Activate` succeeds only for a deal that still has Proposed and no failure
flag (neighbors.md §5.4); for a plain cancelled deal it returns false, and `SetAmount` still runs (and is itself
ignored under Grace/Probation).

Worked cases: Active, last 50 → amount 50, inc := lastInc. Active, last 0 → Cancel (Cancelled set, Active cleared,
amount 0), last := 0, lastInc := 0. Proposed+Cancelled, last 30 → Activate succeeds (Active, not Proposed, not
Cancelled), amount 30. Proposed only (not Cancelled, not Active) → untouched here; `DestroySubBudgetWindow` then
cancels it (§6, bit 0 → slot 34).

### 5.5 `SetOccupantFundingLevel` (`0x0077BC90`) *[compat]* (BW3, BW8)

`bool SetOccupantFundingLevel(cIGZUnknown* occupant, bool which, f32 level)` (thiscall, `RET 0xC`). `level` is in
percent. Steps: QI `occupant` for the occupant IID `0x26BB767E` into a holder; failure → return false. Branch on the
dialog's **service kind** `+0x134`, which `initIndividualBudget` sets from the department ID `+0x24`
(`0x00786815`–`0x00786A16`; department IDs per budget.md):

| Department `+0x24` | Kind `+0x134` | `which = 0` | `which = 1` |
|---|---|---|---|
| `0x09188F42` health, `0xAA538CB3` | `0xCA565486` hospital | staff funding | ambulance funding |
| `0x09188F4C` education, `0x4A538CC6` schools | `0xEA5654B6` school | staff funding | bus funding |
| `0xA2963983` police, `0xA2963984` jail | `0x0A567BAA` police | station funding | return false |
| `0x28F55A9F` fire | `0xEA567BC3` fire | station funding | return false |
| `0x8910BC8A` power | `0xCA58E540` power | plant funding | return false |
| any other | 0 | return false | return false |

- **Hospital / school**: `q := ResidentialSimulator` (global `0x00B43DA4`) `.GetHospitalQueryData(occ, &rec)` (slot 27
  `+0x6C`) or `.GetSchoolQueryData` (slot 26 `+0x68`); false → return false. Overwrite one f32 of the record with
  `level × 0.01f` rounded once to f32: `which = 0` the staff funding (`rec+0x2C`), `which = 1` the vehicle funding
  (`rec+0x34`); the other keeps the value just read. Then `SetOccupantFundingPercentages(occ, &rec+0x2C, &rec+0x34,
  isSchool)` (slot 19 `+0x4C`; isSchool 0 for hospital, 1 for school). Return true. The record layout is
  residential-education-health.md (query data).
- **Police**: `PoliceSimulator` (`0x00B43D98`) `.SetStationFunding(occ, &p)` (slot 8 `+0x20`), `p = level × 0.01f`.
- **Fire**: `FireProtectionSimulator` (`0x00B43D78`) `.SetStationFunding(occ, &p)` (slot 9 `+0x24`).
- **Power**: `PowerSimulator` (`0x00B43DA0`) `.SetPowerPlantFundingPercentage(occ, &p)` (slot 51 `+0xCC`).

The setter's own result is ignored; the routine returns true after the call and releases the holder on every path.
`which ≠ 0` for police, fire or power returns false **before** any call. Funding level 0 is therefore "staff / station /
plant" and level 1 "vehicles" (bus, ambulance). Example: school, which 1, level 75.0 → bus funding `0.75f`
(`0x3F400000`), staff funding unchanged.

### 5.6 Ordinance rows and the info popup (child `+0x14`) *[compat]* (BW9)

The ordinance dialog keeps a row map `+0x74` (u32 per button, indexed by `id − 300` or `id − 500`) giving the index
`k` into the byte pairs `+0xA8` (`[2k]` original, `[2k+1]` edited) and the ordinance IDs `+0xA4`.

- **Check box `300 + r`** (`0x0078B600`–`0x0078B663`): `k := map[r]`; the child window `id` (slot 35) is asked for its
  check-box interface `0x8810` (slot 0); if it answers, `edited[k] :=` its state (slot 10 `+0x28`, low byte) and the
  interface is released; then **always** `updateOrdinanceBudget(win)`. Return 0.
- **Info `500 + r`** (`0x0078B67E`–`0x0078BB69`): close any open child first; `k := map[r]`; build a full-dialog
  generator window (class `0x4386D516`), store it at `+0x14`, ID `0x0423278D`, add it to `win`, then a second
  generator window (ID `0x0423278F`, background image instance `0x144161E4`); the popup content is two texts
  (`add_text` IDs `0x0ABCE000`, `0x0ABCE001`, from ordinance `k`) and two image buttons, `0x384` (image
  `0x14416242`) and `0x68` (image `0x140155B4`). Both buttons close the child (§5 table). Return 0. (Pixel layout
  is out of scope.)

`+0x14` is also set by `SetEmptyWindow` (`0x0077BEC0`, the empty-group message window of §4.1). Buttons `0x68` and
`0x384` call `win.RemoveChild(+0x14)` **without** a null check (`0x0078B287`–`0x0078B291`) and only then test the
pointer; the other "close the child" paths test first.

### 5.7 Camera jump (`0x0ABCDE00`–`0x0ABCDEFF`, individual mode) *[compat]* (BW3)

`item := get_window_indiv_item(win, id)`; null → return 0. QI `item` for the occupant IID `0x26BB767E`; if it answers:
`v := (z, z, z)` where `z` is the f32 global `0x00B4DFCC` (in uninitialised data, so 0.0 unless written), then
`occupant.GetPosition(&v)` (slot 8 `+0x20`), then the camera control of the renderer (`0x00B43DD0` slot 16 `+0x40`,
no null check) jumps to `v` (`0x007CD810`, the routine of the Lua `camera_jump`, advisors.md). Release the holder.
Return 0, no refresh. The dialog stays open.

## 6. Closing (`end_dialog` `0x00779850`, `DestroySubBudgetWindow` `0x0077A1A0`) *[compat]*

`end_dialog(win, code)` (thiscall, `RET 8`; BW9): `win` null → false. `parent := win.GetParent()` (window slot 6
`+0x18`). If `win` has flag `0x1000` (window slot 67 `+0x10C`; the modal flag), call `parent` slot 9 (`+0x24`) with
`(0xF, win, code)`: **`code` is the modal result** handed to the loop that `DoModalWin` runs; the panel ignores that
result (§2 step 5), so every caller's `0x42B7C353` has no further effect. Otherwise call `win` slot 11 (`+0x2C`) with
first argument 1 and pass its result, with `win`, to `parent` slot 14 (`+0x38`) (`0x0077988C`–`0x007798A0`; `code`
unused). Returns true in both cases.

`DestroySubBudgetWindow` (`0x0077A1A0`): if selector 2, for each held neighbour deal (vector `+0x68..+0x6C`, 8 bytes
each): AddRef; if the deal's flags (slot 36) have bit 0 set, call deal slot 34; Release. Then
`NeighborsSimulator.SetUIDisableFlag(false)` and `MarkDealsDirty()`. In every mode: `Simulator.HiddenResume()`
(`0x0077A22C`, **unguarded**); then if `+0x0C` is non-null, `end_dialog(+0x0C, 0x42B7C353)` and return its result,
else return true. `+0x0C` is neither released nor cleared here (§2 references).

Pause balance *[compat]* (BW6b): the simulator's pause is a signed counter (`+0x34`). `HiddenPause` (`0x00702200`)
increments it and posts message `0x4A7FB7E2` with 1 only on the 0→1 step; `HiddenResume` (`0x00702240`) decrements it
with no floor and posts `0x4A7FB7E2` with 0 only when it reaches 0; `IsHiddenPaused` (`0x00703D60`) is `counter > 0`.
`on_init` pauses once per window (guarded by `+0x08`), `DestroySubBudgetWindow` resumes once per call. When
`CreateSubBudgetWindow` fails (no window, so no `on_init`), the resume still runs: the counter drops by one with no
matching pause. Worked case: counter 0, create fails → counter −1, no message, not paused; the next dialog's
`HiddenPause` brings it to 0, posts nothing, and the simulation is **not** paused while that dialog is open. A nested
individual dialog (§2) pauses on top (1 → 2) and resumes back to 1 on close, with no message.

Order matters *[compat]*: the pause ends **before** the window is ended, and the panel refreshes its balance sheet
only after both (§2 step 5).

## 7. Budget panel message hooks (`cSC4BudgetUIWinProc::DoMessage`, `0x00768620`) *[compat]*

Messages `0x08DAC3E7` and `0x08DAC3E8` set a dirty byte (`+0x192` of the `+0x0C` message-target interface that
`DoMessage` runs on, i.e. object `+0x19E`, the byte slot 8 `Update` also sets: budget-window-bodies.md §11.5);
message `0x86AD10EE` (a frame/update tick), when
the byte is set, clears it and runs `UpdateBalanceSheetInfo`. So the panel refreshes at most once per tick however
many money messages arrive. `ShowWindow` (`0x00768670`) refreshes immediately and sets `+0x19D := 1`.

## 8. Open work (docs/specs/questions/budget-window.md)

BW1–BW10 are answered (re-budgetwindow4 and re-budgetwindow-answers4, 2026-10-21; archive
`questions/answered/budget-window.md`). **BW11** is narrowed to the explicit live-input observation in §4.0;
the hook consumed-byte and no-default-processing rules are settled statically, with no game verdict.
**BW16** (row visibility, §4.1) and **BW15** (+0x2C ownership) are byte-settled. **BW21**'s constructor and
nonmodal observation boundary is §1.1.

**BW17** is archived as a partial parent closure; its duplicate open entry was stale. The companion
[budget-window-bodies.md](budget-window-bodies.md) §1 gives the method/slot inventory, §12 the requested-part
crosswalk, raw-provider checklist and panel worked cases. Its §§4/8–11 specify the panel's calculation,
rendering, formatting, taxes, loan windows, labels, Init and notifications, including BW18's former summary
gaps. The remaining **sub-budget** body questions are BW22, BW24 and BW25 (companion §7), not omitted panel methods:
neighbour refresh/rendering, group B and item rendering/cleanup,
and remaining widget/popup/setup ownership. Companion §§1.2–1.3 now settle pointer-property and bounded
empty/setup/segment ownership; §§3.3–3.4 settle neighbour list releases, unused metadata and combo
options/selection/deal writes, including exact public raw inputs and worked cases. Section 5 settles +0x44 acquisition/rebuild references, service-produced
float values and amounts, control reuse and the initializer continuation. The generator procedure setter's reference contract remains BW25. Counts, rows, restore
records and refresh results computed by either owner are never external raw inputs. No whole-sub-budget-owner,
implementation or native-shadow completion is implied by the panel's specification status.

## 9. Conformance tests

BW21 observation vectors (static contracts; no native adapter run claimed):

- Construct in correctly allocated 0x140-byte storage with no explicit arguments: EAX equals the storage,
  +4=0, +8=0, counts/offsets are zero, scale=1. No budget-provider enumeration occurs until init.
- Call CreateSubBudgetWindow for a group with period 7, department 0: before init there are no populated
  rows; after normal init scale=12. Period 0 instead yields scale=1. Passing the selector as a constructor
  stack argument does not configure the group.
- Populate both A and B with different nonzero totals: both total controls retain their own money text,
  but +0x90/+0x94 after both passes contains B's scratch total. A reader treating +0x90/+0x98 as an
  A/B total pair fails this vector (+0x98 is layout state).
- An init delivered twice pauses only once; one Destroy balances that pause. A null-window init returns
  0 and produces no rows. Test modeless teardown with a valid parent and release all held references.

1. Opening any selector while a sub-budget dialog is held does nothing.
2. While a dialog is open `IsHiddenPaused()` is true; after OK or Cancel it is false and the panel balance sheet was
   refreshed once.
3. Group mode, move one slider to 30, press Cancel: every department in the group is back to its starting fraction.
4. Ordinances: toggle two rows, press `0xCD`: exactly two `SetOrdinanceOn` calls; toggle and press `0x6D`: none.
5. Neighbour mode: `uiDisabled` is 1 while open, 0 after close, and deals are marked dirty on close.
6. Esc closes; Enter without modifier bit `0x4` does not (returns 0).
7. Pause counter: with the counter at 0, open and close a dialog: it reads 1 while open (one `0x4A7FB7E2` message
   with 1), 0 after (one message with 0). Force `CreateSubBudgetWindow` to fail: after the call the counter is −1,
   `IsHiddenPaused()` is false and no message was posted.
8. Nested individual dialog: the counter is 2 while it is open and 1 after it closes; no pause message on either
   step; the panel balance sheet is not refreshed by the nested close.
9. `DoWinMsg` returns: slider message 0, combo 0, Esc 1, unknown message 0, a second init 1 with no second pause.
10. `on_shutdown` on an uninitialised window returns 1 and leaves `+0x0C` set; on an initialised one it clears
    `+0x0C`, after which `DestroySubBudgetWindow` calls no `end_dialog` and returns true.
11. Panel commands `0xAA3AC40E`/`0xAA3AC40F`: tab byte `+0x194` becomes 1/0, tab `0xAA3AC001`/`0xAA3AC000` is shown
    and the other hidden, the balance sheet is refreshed, and the command returns 0.
12. Scrolling list A with `len = 12`: positions 0 → 2 → 4, a fourth down press stays at 4; scrolling down on `len = 5`
    stays at 0 and shows the up arrow.
13. Group slider at position 30 passes `0x3E999999` (f32 product) to `SetFundingPercentage`.
14. Key message type 5 with key `0x0D` and mask `0x4` (or `0x5`, `0x40000004`) closes and returns 1; mask `0x1`,
    `0x2` or `0` returns 0 with no effect. The same for key `0x73`.
15. After `on_init` in any mode with a working GZCOM, `+0x10` is non-null, has ID `0x0423278E` and is a child of the
    dialog; after `on_shutdown` it is null.
16. Group with departments but both row counts 0: after `on_init`, `+0x2C` is empty, `+0x38` holds one pair per
    department, and no arrow `0x451`–`0x454` exists.
17. Group mode with `+0x124 = 9`, `+0x128 = 10`: after `on_init` arrows `0x451`, `0x452`, `0x453` are hidden and
    `0x454` is in its created state.
18. Selector 1 (ordinances) with `+0x124 = 9`, `+0x128 = 8`: after `on_init` arrows `0x451`, `0x453`, `0x454` are
    hidden and `0x452` is in its created state. Same counts with selector 2 (neighbours): all four hidden.
19. (BW13) GZCOM `CreateInstance` for class `0x4386D516` fails, group mode with departments: `on_init` returns 1,
    `+0x08` is 1, the hidden-pause counter went up by one (and `DestroySubBudgetWindow` brings it back), `+0x10` is
    null, the dialog gained no `0x0423278E` child, `+0x38` holds one pair per department, and nothing faults.
20. (BW14) Group with N departments, each a refcounted fake: after `on_init` each department's count is its start
    count + 1 for its `+0x38` pair (all rows hidden, both row counts 0, so no `+0x2C` rows), including across a vector grow; after `on_shutdown` every count is back to its start value; a window destroyed without
    `on_shutdown` also returns every count to its start value. An implementation that holds the department as an
    uncounted handle fails this test.
21. (BW15) Empty groups. (a) `GetDepartmentBudgetsInGroup` returns no departments: after `on_init`, `+0x2C` and
    `+0x38` are empty and no department was touched. (b) Departments whose rows are all hidden: `+0x2C` is empty,
    each department's count is start + 1 (only its `+0x38` pair), as in test 20.
22. (BW15) Non-empty group `0x4A357EAF`, refcounted fakes; every line item has display flag bit 0 set and a non-zero
    amount (shown under any reading of the ID test). Department X has only one income item; Y has two expense items;
    Z has twelve expense items, Y before Z. After `on_init`: X count start + 1 (pair only, no slider); Y start + 4
    (pair + header slider + 2 item sliders); Z start + 7 (pair + header + 5 items: Y used rows 1–3 of B, Z gets rows
    4–9, so the cap of 9 stops it). `+0x2C` holds 9 elements in build order {Y, Y, Y, Z, Z, Z, Z, Z, Z}, each with a
    distinct slider ID matching a child of `win`. The slider windows' own refcounts do not depend on `+0x2C`. With a
    capacity-growing vector the counts are the same.
23. (BW15) Repeated `on_init`: a second `on_init` while `+0x08` is set changes no count and no vector. `on_shutdown`
    then `on_init` again: after the shutdown every count is at its start; after the second `on_init` the deltas are
    those of test 22 again (not doubled).
24. (BW15) Shutdown then drop: after `on_shutdown` all counts are at start; destroying the window afterwards Releases
    nothing (no count goes below start). Release order, seen by a logging fake: `on_shutdown` Releases all `+0x2C`
    elements front to back, then all `+0x38` pairs front to back; a destructor without `on_shutdown` Releases the
    `+0x38` pairs first, then the `+0x2C` elements, each front to back, and every count ends at start.
25. (BW15, BW13) Content-window creation fails in the case of test 22: the implementation does not fault, sliders
    are still created as children of `win`, and the counts and `+0x2C` contents equal test 22. (The native program
    faults at `0x0078837B` with `+0x2C` empty and the `+0x38` pairs taken; this test does not reproduce that.)
26. (BW16) Row visibility at the ID boundary. Group `0x4A357EAF`, one department with six income items, in order:
    ID 99, 100, 101 each with income 0, then ID 99, 100, 101 each with income 25; every item has display flags 5
    (bit 0 set). countA = 1 header + 5 rows = 6: hidden is only ID 101 with income 0. Repeat with the same items as
    expense items (B, `GetFullExpenses()` 0 / 25): countB = 6, the same row hidden. The built rows (first 9) show,
    in order, 99, 100, 99, 100, 101 after the header.
27. (BW16) Same items as test 26 but every display flag is 4 (bit 0 clear): countA = 1 (header only) and no item row
    is built, for every ID and amount; with flags 1 the result equals test 26.
28. (re-budgetwin) Monthly view, tab 1, funds −5, income 10, expense 2: `0xAA3AC10E` shows −5 in (0x77, 0, 0);
    `0xAA3AC111` shows 3 in (0x44, 0x55, 0x66). Yearly view, tab 0: `0xAA3AC101` shows `GetEstIncome()` and
    `0xAA3AC112` is not written; tab 1: `0xAA3AC112` shows 12 × the monthly residential taxes.
29. (re-budgetwin) A group `0x4A357B40` record with expense 300 and income 7 shows 300 in `0xAA3AC125`.
30. (re-budgetwin) Tax edit field 1 with an unparsable text while type 3's rate is 9.0: the slider becomes 90; field
    10 with an unparsable text: the slider becomes 0.
31. (re-budgetwin) Loan window opened with 2 loans; accept 50,000: `IssueBond(50000)` is called once and the list
    holds 3 records.
32. (re-budgetwin) Pressing `0x4A8879CD` sets `+0x198` to 1; the next sub-budget dialog gets `+0x28` = 1.
