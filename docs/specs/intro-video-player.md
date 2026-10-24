# The intro video player window `cSC4MadCowPlayerWin` (behaviour spec)

Status: M6 behaviour spec, written 2026-10-22 (re-radio lane, outside contributor). Facts were read on the Windows
executable `bin/SimCity4.exe` (1.1.641, image base `0x00400000`) from the Ghidra export of the class's 16 owned
functions; numbers carry addresses. Implementers work from this spec, the LGPL gzcom-dll SDK headers and general
knowledge only, never from decompiled code.

Tags: *[compat]* observable, must match; *[inferred]* consistent with the evidence, not proven; *[as read]* read
from the export without a byte check.

Scope. The window that plays EA "MAD" videos (the Maxis logo and splash at boot, driven by
`cSC4WinIntroVideoScreen`, [ui-windows.md](ui-windows.md)): its identity, construction, drawing surface, pause on
app deactivation, shutdown and `StopMovie`. The MAD decoder and its frame loop (the rest of `0x0080F760`–`0x00812000`,
INVENTORY-M6 §0.1) are not covered.

## 1. Identity

- A `cGZWin` with a virtual base (constructor `0x0080FFC0`, named `SetAudioEnabled` by the port; `0x0080FFC9` is
  the same constructor's second entry, used when the virtual base is already built). The first argument chooses
  whether the virtual base's tables are installed.
- `QueryInterface` (`0x0080EEA0`): IID `0x452294AA` returns the virtual-base interface; anything else goes to
  `cGZWin::QueryInterface` *[compat]*.
- The constructor zeroes the playback state, sets two flags (`+0x194`, `+0x195`) to 1 and `+0x154` to 1, and builds an
  empty frame queue (`0x00811BB0`).

## 2. The drawing surface (`0x0080FB60`)

Through the graphics system (`RZ::GS()`; none → false): without a buffer (`+0x1F4`) one is created (graphics-system
slot 3); with one that reports lost (buffer slot 39) it is restored (slot 4). With a non-zero size, the buffer is
sized `(w, h)` in the display's colour depth: a 16-bit display mode (graphics-system slot 9, depth field = 16) →
format `(4, 16)`, otherwise `(1, 0)` (buffer slot 3) *[compat]*.

## 3. Pausing on deactivation (`0x0080FC30`)

On the app-activation message (payload IID `0xDA1062FA`, state from its slot 9) *[compat]*:

- state 2 (active) while paused: resume the movie's sound when it is paused (slots 19 then 18), restart the movie
  timer, redraw (window slot 91), clear the paused flag;
- state 3 (inactive) while not paused: pause the sound when it plays (slot 17 with 1), stop the timer, set the
  paused flag.

## 4. Stopping and shutting down

`StopMovie` (`0x008103E0`): only when a movie is open (`+0x128`) and a decoder exists (`+0x120`): stop the decoder
(slot 5) and release it, reset the timer, clear `+0x170`, kill the sound when it plays (slot 9), and empty the frame
queue — a binary heap of 12-byte entries ordered on their first dword (sift `0x0080FA40`/`0x0080FAB0`). Returns
whether it stopped anything.

`Shutdown` (`0x008101B0`): release the decoder, free the buffered frames (`0x0080FE50`), release the audio stream,
remove the window from the canvas service's listeners (framework slot 5 with `(0xA3460DEB, 0x66B217A1)`, QI
`0x3AC7A210`, then slot 6 with `(window, 0x1A10630A)`), release the remaining objects and run `cGZWin::Shutdown`.

## 5. Open items

None filed; the decoder is the remaining part (§ Scope).
