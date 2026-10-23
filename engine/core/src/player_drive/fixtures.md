# Literal fixtures, written before production implementation

Source: drive-tool sections 1-10 only. These are independent expected sequences, not roundtrip or host-computed outcomes. All IDs/commands retain numeric meaning only.

- Construction: ID 2B709676; cursor C16C4E1B; speed bits 3D2C0831; toggles false,false; no current/hover; cell width/inverse 1.
- Choosing, no current: hover drop / cursor AB73510C clear; simulator pause iff raw paused=false; manager43,44,45,46 pause even if simulator already paused.
- Portable driving: disaster false before form lookup; absent form has no driven identity and no pause/resume. Present form: old tracking cleanup, clear hover, minimize UI, show43, select dashboard only (ShowUI unsupported), known sound prefix, acquire driven reference, state52(10,0,BF800000,0,0,0,0), manager34, raw zoom limits, zoom, two button enable effects, toggles true,false, dispatch track, callback39, timer reset, conditional simulator Resume, manager43..46 resume. Unknown table audio/settings origin remain explicit gaps, no full entry success.
- Dashboard selection: type480E6151 => 6BE8846C; other => 0BE8824F; property2BE8FBF3 overrides either including zero.
- Network accepted: network pick/accept, sound8BE8D9AC at raw position, selected cell; NO automaton pick. Rejected/miss permits automaton pick only when no current.
- Validity: 0,2 => valid; 08000000,1,08000001 => invalid independently.
- Numeric command down/up: 0=>slot60(0,1)/event1 start, slot60(0,0)/event1 stop; 1 event2; 2 light8BCB93FC before slot60 then event3; 3 light6BCB93F5 then event3; 5 event6; 7 event4; 6/11/12 slot60 only. Command8 only raw mouse path; not invented keyboard-table row.
- Command4 starting on: down sends (4,0), next down (4,1); ups empty. Command10 starts off: down (10,1), next down (10,0); ups empty.
- Command9 down: slot60(9,1), drive byte229 toggles 0=>1,1=>0; up slot60(9,0) only. Nonboolean byte has no published rule => Unsupported, no normalization.
- Key down repeat40000000: no keymap, callbacks or effects including Escape. Escape without repeat: EndInput prefix and disaster true after cleanup. Mission blocked: no keymap/drive command. Ordinary key: raw script lookup then HandleKeyEffects Unsupported (never invented always-pass).
- Left up: selected cell None first, then valid command7=0 and event4 stop. Right down/up: command8=1/0, no sound. Mission/invalid blocks command effects.
- Stop with state10 flags0/group4006: callback40, dispatch untrack, manager34(None), limits(0,5), engine stop, property sound stop, slot13(0), release held reference. Flag08000000 or group4003 => slot53 instead. Flag1 alone does not select slot53. Non10 state => neither state setter.
- Stop with no current: no effects; this does not make EndInput idempotent. Repeated EndInput still sends callbacks, clears hover/cursor, enables disaster, removes top control.
- Removed identity other than current: nothing. Matching: StopTracking then EndInput. Callback code3 routes same identity gate. CodeB/D for unrelated identity must not stop current audio.
- Callback plan: target order, message0BB324BD, tool identity and exactly supplied two raw values; no guessed second EndInput value or target ownership. Reentrant delivery unsupported.
- Buttons: 4C1A5CC9=>CC1F0457; 2C16C9CB=>AC1F0449; 4992046A=>6A935CB7; E9920494=>6A935CBA. Unknown buttons Unsupported.
- Animation86AD10EE=>Update Unsupported; panel AC1F0449/AC1F045D/CC1F0457 => Unsupported (direction/IDs absent); A6B79602=>both zoom buttons re-enabled literally (DR7), unlike entry's CanZoomIn/Out queries. CodeB alternate-model argument absent => Unsupported, not guessed.
- No tests claim native lifetime, ShowUI, gauges, state10 implementation, audio engine logic, camera tracking cadence, or complete owner conformance.
