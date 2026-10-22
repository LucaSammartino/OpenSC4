| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span565-context4 | Codex / `re-span565-context4` | ready | integrate docs; exercise public-setter vectors |

## Report (2026-10-22)

Resolved SR565-7/8 in `docs/specs/span-render-565.md` sections 1.3 and 6.2. Audited the preserved partial
section from `16e1adcc` / quota-pause `7949c81e`. Real-work commit: `9ecfda9b` (public setters and texture route).

- SR565-7 **MATCHES** the bridge's stated conventions: draw-system slot 4 `0x0082BAB0` is thiscall,
  `(buffer,out)`, pops 8, returns byte AL. Context slot 4 Shutdown `0x0082FF20` takes no arguments,
  pops none, and has no API return value, including its tail-Release path.
- Published exact setter slots, argument widths, pops, returns and field provenance: colour/ARGB 7,
  alpha 8, modulation 9, gradient 12, texture 13, tiling clear/set 14/15. Vtable `0x00AC17D0` has 23 context
  entries. Type-5 buffer converter `0x00825DE7` packs `0x80FF8040` to `0xFC08`, retaining ARGB in `+0xB4`.
- SR565-8 **DIFFERS** from the premise of a stored context source record: `+0x24` holds a referenced buffer
  interface. Slot 22 builds six source-surface dwords on its stack from buffer vfuncs after Lock(`0x800`),
  then adjusts them for the selected mode and calls RenderBlt. Slot 13 fills placement/scales; slots 14/15
  control tiling. Null source returns 0; interface refusal returns 1 without a texture or output-rect write.
- Reused main's landed span4444 sections 1.3/1.3.1 and span1555 gradient precision proof. Applicability follows
  from type-5 Init jump entry `0x008308BC` and renderer store `0x008307A0`/`0x008307EE`; setters and slot 22
  remain those of the same 2D context. `draw-context.md` specifies the separate 3D object. Main's section 1.3.1
  is newer than this detached base; referenced its landed proof without copying unrelated spec edits.
- Corrected broad rectangle-rounding shorthand and distinguished buffer vfunc offsets from object fields.
  Added discriminating setter and plain/stretch/tiled sampling vectors. They are specified, **not executed**;
  no test was fitted to current core behavior and no engine source was read.

## Evidence and validation

Used main-checkout `re_query.py asm --bytes` and raw image dwords via preserved `build-runs/vt.py`.
No REA call was required. Constructor `0x00830690` lacks exported instructions; re_query's fallback failed
because capstone is unavailable. Its raw bytes independently confirmed the constructor stores. All remaining
needed instruction pages were exported. No tools, decompilation output or game bytes were committed.

`python tools/questions.py index`, `python tools/questions.py check`, and `git diff --check` pass.
SR565-7/8 are archived with dated MATCHES/DIFFERS rules and address anchors in
`docs/specs/questions/answered/span-render-565.md`; the questions index is regenerated. No new questions.
Only docs changed. No engine/tools/ABI edits or ABI proposals; cargo/clippy/ctest and game runs are n/a.
Git emitted a packed-refs lock permission warning during commit; the detached-HEAD commit succeeded and the
tracked worktree was subsequently clean. Shared refs/config were not changed.

## Remaining work

Implementers can compare the public-setter/slot-22 route with the specified vectors. Label any resulting run
constructed native context; this lane produced no runtime shipped-path evidence. Existing SR565-1/5 runtime
questions and SR565-6 lock-policy question remain open and were outside this task's scope.
