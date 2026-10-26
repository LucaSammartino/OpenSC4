# `cSC4AnimationTickManager`: the animation tick (behaviour spec)

Scope: the `cSC4AnimationTickManager` owner as tools/coverage.py assigns it (37 functions, 4,052 bytes). The manager
is a framework system service that sends the **animation tick** message `0x86AD10EE` to registered objects on the
wall clock, in four priority classes with different rates, with pause, single-step and auto-step. The simulation
depends on it (INVENTORY-M6.md §2.4: the lot and prop developers, the terrain, the fire disaster, the automata
controller manager, the effects manager, the city situation panel, the advisor faces register with it).

Related: effects.md §6 (what the effects manager does with a tick; the pause counter's effect, §6.1),
foundations.md §11.6 (the terrain's tick), simulator.md §7 (the animation time dilation the receivers apply),
application.md (the main loop that drives the framework's tick), advisor-face.md (a receiver).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; the vtable `0x00A86760` and the
constants `0x00A867A4`–`0x00A867AC` were read. Written 2026-10-24 (re-animtick7).

Times are microseconds (the two timers are `cRZTimer` with unit argument 3).

## 1. Identity and state *[compat]*

- Class `0xA9C73857` (`0x004483E0`); system service `0x86AD10EC` with tick priority `0xFFE17B80` (−2,000,000).
- **`QueryInterface`** (`0x00447E60`, thunks `0x004483F0`, `0x00448420`, `0x00448450`, `0x00448480`): 1 and
  `0x86AD10ED` (`cISC4AnimationTickManager`) → the object; `0x452294AA` → `+4` (message target); `0xE4FDA3D4` → `+8`
  (serializable); else `cRZSystemService`'s. **`Release`** (`0x00447E40`, thunks `0x00448410`, `0x00448440`,
  `0x00448470`, `0x004484A0`): the last reference deletes (destructor `0x004487D0`, body `0x00448340`, thunk
  `0x004484B0`).

| Offset | Field | Constructor (`0x00448E30`) |
|---|---|---|
| `+0x28` | reference count | 0 |
| `+0x2C` / `+0x2D` | initialised / inside a tick | 0 |
| `+0x30` | the tick message (`cRZMessage2Standard`) | null |
| `+0x38` | the clock | |
| `+0x50` / `+0x54` | minimum / maximum delta | 2,000 / 500,000 |
| `+0x58` | the time of the current tick | 0 |
| `+0x5C` | frame counter | 0 |
| `+0x60` | smoothed tick rate (ticks per second) | 0.0 |
| `+0x64`…`+0x80` | eight frame masks (§2) | 3, 5, 3, 9, 3, 5, 3, 1 |
| `+0x84` | pause count | 0 |
| `+0x88` | step delta | 100,000 |
| `+0x8C` | step pending (byte) | 0 |
| `+0x8D` / `+0x90` / `+0x98` | auto-step on (byte) / its interval / its timer | 0 / 0 / |
| `+0xB0` | the list of requests made during a tick | empty |
| `+0xB4`…`+0xC4` | the objects removed during the current tick | empty (100 buckets) |
| `+0xCC` + 20·k | the hash set of priority class k (k = 0…3) | empty |
| `+0x118` + 4·k | the time class k last ticked | 0 |

## 2. Classes and rates *[compat]*

A priority `p` (0–255) maps to a class: `p ≥ 0xC0` → 0, `p ≥ 0x80` → 1, `p ≥ 0x40` → 2, else 3. The frame counter is
incremented at every tick and its low three bits pick a mask from `+0x64`; bit k of the mask means "class k ticks this
frame". With the shipped masks class 0 ticks every frame, class 1 every second frame, class 2 every fourth frame and
class 3 every eighth frame (frame indices 1, 2, 3, … use the masks 5, 3, 9, 3, 5, 3, 1, 3, …).

## 3. The interface *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 3 `Init` | `0x00448010` (thunk `0x004484C0`) | once: pause count and step 0, the clock restarted, the service added to the framework (slots 3 and 9); the message is made once: type `0x86AD10EE`, sender this (its slots 5, 12, 33) |
| 4 `Shutdown` | `0x004484D0` (thunk `0x004487C0`) | when initialised: the message released; the service removed (framework slots 10 and 4); the pending requests dropped (each target released); every registered target released and the four sets emptied; state reset |
| 5 `AddTickableObject(target, p)` | `0x00448B70` | during a tick: queued (§4); otherwise added to class(p)'s set with a reference, unless it is already there (true either way) |
| 6 `RemoveTickableObject(target, p)` | `0x00448C70` | during a tick: queued; otherwise with `p` ≠ −1 removed from class(p) (falling back to `p = −1` when it is not there), with `p` = −1 from the first class, 3 down to 0, that holds it; the reference is released; true |
| 7 `FindTickableObject(target, p)` | `0x00448170` | `p` = −1: any class (3 down to 0); otherwise class(p) only |
| 8 `SetNewPriority(target, p)` | `0x00447FD0` | slot 6 `(target, −1)` then slot 5 `(target, p)`, holding a reference meanwhile |
| 9 `DoAnimationTick` | `0x004487F0` | §4 |
| 10 `GetCurrentTickTime` | `0x0040D390` (shared) | `+0x58` |
| 11 `SetMinAndMaxTickTimeDeltas(min, max)` | `0x00447F40` | stored; a 0 minimum becomes 1 and the maximum is raised to the minimum |
| 12 `PauseAnimationSystem(on)` | `0x00447EE0` | on: the count goes up (and the auto-step timer restarts when auto-step is on and the count was ≤ 0); off: it goes down, unchecked |
| 13 `IsAnimationSystemPaused` | `0x00447F30` | count > 0 |
| 14 `EnableAutoStep(on, interval)` | `0x00447F90` | the interval is stored; a change of `on` restarts (on) or resets (off) the auto-step timer |
| 15 `SetStepTickTimeDelta(dt)` | `0x004FAD10` | `+0x88` |
| 16 `StepAnimationSystem` | `0x00447F70` | paused: one step pending; running: `PauseAnimationSystem(1)` (the first step only pauses) |

## 4. A tick (`DoAnimationTick`) *[compat]*

A nested call returns at once. Otherwise:

1. Mark "inside a tick"; frame := frame + 1; `now` := the clock; on every eighth frame (low bits 0) the rate is
   smoothed: `rate += (1,000,000 / (now − previous + 3) − rate) · 0.125`.
2. With auto-step on and the system paused, an elapsed auto-step timer (≥ the interval) sets "step pending" and
   resets the timer.
3. When the system is not paused, or a step is pending, each class k whose bit is set in this frame's mask and whose
   set is not empty ticks: with a step pending, `now := lastTick[k] + stepDelta`; `dt = clamp(now − lastTick[k],
   min, max)`; the message gets `dt` (slot 16) and `dt · 0.001` as a float (milliseconds, slot 21); every target of the
   set that was not removed during this tick receives it (its slot 3, `DoMessage`); then `lastTick[k] := now`. The
   step-pending flag is cleared after the classes.
4. "Inside a tick" is cleared and the queued requests are applied in order: an add as slot 5 `(target, p)`, a remove
   as slot 6 `(target, −1)`; each queued target's reference is released.

The result is the `dt` of the first class that ticked (normally class 0), the minimum delta when the classes ran but
none ticked, and 0 when the system was paused with no step pending.

## 5. Saving *[compat]*

- **Write** (`0x00448650`): version 1 (u16, stream slot 7); for each class 0–3: the count, then each target written as
  a serializable (`SC4WriteGZSerializable`) when it has the interface `0xE4FDA3D4` (`0x005D9690`), else as null; true
  when the stream reports no error (slot 18).
- **Read** (`0x00448AB0`): only version 1; for each class the count and that many serializables (`SC4ReadGZSerializable`
  with IID `0x452294AA`); the objects read are **not registered** *[quirk]*; true when the stream has no error.

## 6. Quirks

- Removing a target during a tick only hides it from the rest of that tick; the removal itself happens afterwards.
- `PauseAnimationSystem(false)` without a matching pause makes the count negative, and the system then ticks even
  after the next pause (count back to 0).
- A clamped `dt` means a long frame is reported as at most 0.5 s and a short one as at least 2 ms.
- Read discards the tickables it reads; receivers re-register themselves when they load.

## 7. Function inventory

`0x00447E40`, `0x00447E60`, `0x00447EE0`, `0x00447F30`, `0x00447F40`, `0x00447F70`, `0x00447F90`, `0x00447FD0`,
`0x00448010`, `0x00448170`, `0x00448230` (hash iterator step), `0x00448250` (hash resize), `0x00448340`, `0x004483E0`,
`0x004483F0`, `0x00448410`, `0x00448420`, `0x00448440`, `0x00448450`, `0x00448470`, `0x00448480`, `0x00448490`
(AddRef thunk), `0x004484A0`, `0x004484B0`, `0x004484C0`, `0x004484D0`, `0x00448650`, `0x00448730` (apply queued
requests), `0x004487C0`, `0x004487D0`, `0x004487F0`, `0x00448AB0`, `0x00448B70`, `0x00448C70`, `0x00448E30`,
`0x004FAD10`, `0x005D9690`.
