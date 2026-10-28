# ui-script-service: answered questions (archive)
<!-- Archive for [ui-script-service.md](../../ui-script-service.md). Lanes do not read this file. Open questions are in ../ui-script-service.md. -->

<!-- from: re-gzuiscript7 (2026-10-24) -->
- ~~**UIS1 - §8:** the control set's hit dispatch (`0x0094BEBA`) calls each element's `0x5A356159` slot 5 with an
  output rectangle and slot 6 with two arguments `(point, arg)`. For the legacy control (the only element the UI
  scripts build), slot 5 (`0x0094C3F5`) is `SetArea(rect)` (window slot 54) and slot 6 (`0x0094C413`) is
  `GetArea(rect)` (window slot 47, one argument): the dispatch would set the control's area from an uninitialised
  rectangle and unbalance the stack. Who calls the set's second-interface slot 4, and with which element types?
  [static: callers of the `0xFA35615F` interface; until known, a reimplementation should not route points through it
  for legacy controls]~~
  **ANSWERED 2026-10-27 (re-x-ui):** DIFFERS — the legacy control's `0x5A356159` face has seven slots (`0x00AD5344`): slot 5 is `GetArea(rect)` (a rectangle getter) and slot 6 takes the point and returns true, so the dispatch fits; the `SetArea` slot is slot 4. The dispatch itself always returns false, and no code in the executable asks for interface `0xFA35615F` (section 8).
