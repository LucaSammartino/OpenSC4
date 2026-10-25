# `cSC4MySimBalloonWinProc`: the MySim thought balloon (behaviour spec)

Scope: the `cSC4MySimBalloonWinProc` owner as tools/coverage.py assigns it (10 functions, 2,857 bytes). One balloon
proc belongs to each MySim: it owns a small window that floats above the MySim's head and shows the pictures of the
MySim's current balloon advice, on a timed show/hide cycle with an optional sound. The constructor (`0x00430B60`)
and destructor (`0x00430C60`) are attributed to `cSC4MySim` (mysims.md) and are summarised in §1.

Related: mysims.md (the MySim interface slots used here, the balloon tunables `MYSIM_BALLOON_*`), advisors.md (the
advice flags), widget-toolkit.md (`cGZWin` flags, fades and private buffers), sound-framework.md (sound events).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; the constructor, `DoMessage`, the
paint and the sound start were checked on the instructions, and the vtables `0x00A8576C` and `0x00A8575C` were read.
Written 2026-10-24 (re-gzballoon7).

## 1. Layout *[compat]*

| Offset | Field | Constructor value |
|---|---|---|
| `+0x00` | win proc, vtable `0x00A8576C`: 3 `DoWinProcMessage`, 4 the base `DoWinMsg` | |
| `+0x04` | message target, vtable `0x00A8575C`: 3 `DoMessage` | |
| `+0x08` | reference count (`cRZUnknown`) | |
| `+0x10` | state (§3) | 5 (disabled) |
| `+0x14` | the current balloon advice (counted) | null |
| `+0x18` | the MySim (constructor argument, counted) | |
| `+0x1C` | the window (`cGZWinGen`, counted) | null |
| `+0x20`…`+0x28` | the pictures: a vector of counted pointers | empty |
| `+0x2C` | repaint pending (byte) | 0 |
| `+0x30` | timer (`cRZTimer`, unit argument 4) | |
| `+0x48` / `+0x4C` / `+0x50` | fade, shown and hidden durations | 300 / 2500 / 300 |
| `+0x54` | initialised (byte) | 0 |
| `+0x55` | sound playing (byte) | 0 |

The constructor also runs the "stop sound" code (§4) on its fresh object; it never fires since `+0x55` was just
cleared.

## 2. Start and the tick *[compat]*

**`Init`** (`0x00430680`), the first time: the window is created (GZCOM `CreateInstance(0x4386D516, 0x5386D516)`)
and, when that works: added to the main game window (`0x00B43DC8`, slot 14); private buffer on (slot 100); flag
`0x20` (fade on show) on; slot 109 with the fade duration twice (the fade period *[inferred]*); area (0, 0, 100, 100)
(slot 55); this proc as its proc (generic-window slot 7) and generic flag `0x1000` on (slot 6); hidden (slot 70);
flags `0x8000` (focusable) off, `0x200000`, `0x20000` (buffer transparency), `0x40000` and 4 on; slot 24 (no
arguments; to the back *[inferred]*); ID `0x4C30E4FA` (slot 64). Then initialised := 1. Every call restarts the timer and posts the first tick.

**The tick** is a message of type `0xCB7CB509` (a new `cRZMessage2Standard`) that the proc posts to its own message
target through the message server (`0x00B43CCC` slot 7). **`tick_self(msg)`** (`0x00430590`) re-posts the message it
is given (or a new one), so the balloon runs once per message-queue pass for as long as it lives.

**`DoMessage`** (`0x00430F70`): the payload `0x65297976` is read (none → false). Other types → false. For the tick:
`tick_self`, then `check_enabled`; when enabled:

1. **Place** the window above the MySim: x := −W, y := −H (its width and height); when the MySim's
   `GetCurrentScreenLocationXY(&x, &y)` (slot 55) succeeds, x −= W/2 and y −= H + 8. `MoveTo(x, y)` (slot 56). An
   off-screen MySim therefore leaves the balloon at (−W, −H).
2. **Step** the state machine (§3).

The tick returns true. The payload reference is released.

**`check_enabled`** (`0x004305F0`): false before `Init`. When the MySim is not active (slot 20 `IsActive`): a visible
window is hidden, state := 5 and the timer stops. When it is active and the state is 5: state := 4 and the timer
restarts. Returns state ≠ 5.

## 3. The cycle *[compat]*

Times are the timer's elapsed value, compared unsigned, and each change restarts the timer.

| State | When | Then |
|---|---|---|
| 1 (fading in) | elapsed > fade | start the sound (§4); state 3 |
| 3 (shown) | elapsed > shown | hide the window (slot 70, fading out); state 2 |
| 2 (fading out) | elapsed > fade | stop the sound (not immediately); state 4 |
| 4 (hidden) | elapsed > hidden **and** the pictures refresh to a non-empty set (below) | stop the sound; show the window (slot 69, fading in); state 1; invalidate (slot 92); repaint pending := 1 |
| 5 | — | disabled (§2) |

In state 4 a failed refresh leaves the timer running, so the refresh is retried on every tick until it finds
pictures.

**Refresh** (`0x00430CE0`):

1. A current advice gets `SetFlags(4, on)` and `SetFlags(2, off)` (advice slot 13; advisors.md names 4 "once" and 2
   "active") and is released.
2. `a` := the MySim's balloon getter slot 61. When it exists: its picture count n (slot 7) sizes the vector; picture
   i (slot 8) fills slot i. When any picture is non-null, `a` becomes the current advice with `SetFlags(2, on)` and
   step 3 is skipped; otherwise `a` gets `SetFlags(4, on)`.
3. When there is still no current advice and the getter slot 60 gives one, its pictures are loaded the same way and
   it becomes current with `SetFlags(2, on)` — even when none of its pictures exists.
4. When the window is narrower than the widest picture or lower than the tallest (slots 41, 42), it is resized
   (slot 53) to exactly (widest, tallest); otherwise its size is kept.

Returns true when the vector is not empty. Null pictures stay in the vector: the size loop and the paint call them
without a check *[quirk]*.

## 4. Sound *[compat]*

Both use the sound system (framework service `0x23CECBA0`, IID `0x106077D2`) and its event slot 11.

- **Start** (`start_balloon_sound`, `0x00430360`), with a sound system and a current advice: `id` := advice slot 10
  (the MySim's slot 18 value); `r` := `RandomDoubleRangeUniform(0, 100)` on the random generator the city's slot 81
  object gives (its slot 9); `p` := advice slot 11. When p > r, id ≠ 0 and the MySim's `RequestToPlaySound` (slot 56)
  agrees: event `(1, id, MySim, 0, 0)` (play); then, when `GetLocation(&xyz)` (slot 14) succeeds, event
  `(0xCA45190F, MySim, x, y, z)` with the coordinates truncated to integers; playing := 1.
- **Stop** (`0x004304C0`, argument `now`): when playing, event `(2, MySim, now ≠ 0, 0, 0)` and playing := 0.

## 5. Painting *[compat]*

**`DoWinProcMessage(win, msg)`** (`0x00430A40`): only for the balloon's window; `0xA2BF8ACD` and `0xA2BF8ACF` →
true; `0xA2BF8ACE` → paint, true; others false.

**Paint** (`on_paint`, `0x00430800`), when a repaint is pending and the window has a private buffer (slot 101): the
buffer is locked (slot 6, `0x8010`). Picture 0 is the base: `dx = (bufferW − W0) / 2` (unsigned), `dy = bufferH − H0`;
it is copied (buffer slot 29, its own bounds, slot 12) offset by (dx, dy) — bottom-centred. Each further picture i is
centred on the base: offset (dx + (W0 − Wi)/2, dy + (H0 − Hi)/2), unsigned halves. Unlock (slot 7); repaint pending
:= 0.

`0x004309F0` assigns one counted pointer to every slot of a range (the vector's fill when it grows).

## 6. Quirks

- The window only changes size when a picture does not fit; a smaller later balloon keeps the larger size and is
  drawn bottom-centred in it.
- Step 3 of the refresh accepts an advice with no pictures; the balloon then shows an empty window for a full cycle.
- The balloon is parked at (−W, −H) when the MySim has no screen position, rather than hidden.
- The paint uses unsigned halves: a picture larger than the base gives a huge offset.

## 7. Function inventory

`0x00430360` `start_balloon_sound`, `0x004304C0` stop sound, `0x00430590` `tick_self`, `0x004305F0` `check_enabled`,
`0x00430680` `Init`, `0x00430800` `on_paint`, `0x004309F0` counted fill, `0x00430A40` `DoWinProcMessage`,
`0x00430CE0` refresh, `0x00430F70` `DoMessage`.
