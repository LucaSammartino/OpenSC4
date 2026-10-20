# Runs glue22-effects-shadow2 / -3 / -trace* (glue22, 2026-10-16): the class replacement faults

Requests with `enable = effects` and `effects_shadow = 1`, once the `effects` class table entry and
`EffectsShadowStartup` call were actually in.

```
verdict: corrupt
AUTOTEST VERDICT: corrupt (foreign_exceptions=1, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0)
```

```
first-chance exception OUTSIDE FreeCity4.dll in 0x0....8023 (no module):
  code 0xC0000005 ACCESS_VIOLATION (writing address 0x0000004D)
  registers: eax=0x00000000 ebx=0x011C8010 ecx=0x005EF720 edx=0x001AFA28
             esi=0x00000000 edi=0x031C801E ebp=0x00000000
  stack[esp+0x024] = SimCity 4.exe+0x9D909
  stack[esp+0x028] = SimCity 4.exe+0xA175F
  stack[esp+0x0C8] = SimCity 4.exe+0xA1A07
  stack[esp+0x10C] = SimCity 4.exe+0x5BF36
  stack[esp+0x130] = SimCity 4.exe+0x558A5
  (FreeCity4.dll frames much deeper: +0xAA63D1, +0xAA5FC7, +0x787765, +0x786F2C, +0x78D34F)
```

Traced runs (`glue22-effects-trace`, `-trace2`, `-trace3`) established:

- `QueryInterface(0xC97CF5CD)` and `Init` run and complete (`AddNotification` returns normally);
- **no other method of ours is entered** — not `Shutdown`, `HasVisualEffect`, `GetStatsString`,
  `GetEffectMaps`, `GetTallestBuildingAltitude`, `Release`, `DoMessage`, or the draw-pass `Update`;
- the fault is ~100 ms after `Init` returns, during city init, in game code.

So the game calls through a vtable pointer it reads from an offset our object does not cover. A
draw-pass sub-object at `+4` (`cFC4ISC4EffectsDrawPass`, `Update` at slot 3) did not change the fault
and `Update` was never called. The original `cSC4EffectsManager` is larger than the one interface
docs/abi and effects.md §1 describe; §2 lists manager state at `+0x1B4`, `+0x1B8`, `+0x1C0`, `+0xB90`,
`+0xBAC`, `+0xC84`, `+0xC8C`.

Consequence: `enable = effects` now fails closed (question 9). `FindEnabledClass` and the `AddCls`
loop skip the `effects` classes unless the run also sets `effects_replace = 1`, and log once:

```
subsystem 'effects': the class replacement is NOT registered (glue22 question 9);
effects_shadow = 1 is the verified path; set effects_replace = 1 to force it
```
