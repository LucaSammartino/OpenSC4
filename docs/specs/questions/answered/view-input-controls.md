# view-input-controls: answered questions (archive)
<!-- Archive for [view-input-controls.md](../../view-input-controls.md). Lanes do not read this file. Open questions are in ../view-input-controls.md. -->

<!-- from: re-viewtools (2026-10-20) -->
- ~~**VIC1 - §3, §4.1, §5.1, §6:** which commands of the city view's `ProcessCommand` (`0x007F1C10`) make the label,
  sign and sign-bulldozer tools, which buttons send them, and what the other cases of `RunGlobalTerrainTool`
  (`0x007E75B0`, commands 0–6 beside levelling's 7) do. [tools specified from their own code; makers named only]~~
  **ANSWERED 2026-10-27 (re-x-ui):** MATCHES — the label, sign and sign/label-bulldozer tools are made by `ProcessCommand` for commands `0x0B980587`, `0x0B980581` and `0xABCDEE11` (buttons `0xCB95412C`, `0x0B954123`, `0xABA7ED0A`), with no established-city test; `RunGlobalTerrainTool` is called from the catalog-item handler for kind `0x6A5AC478`, and its items 0-6 are the terrain-type dialog, two terrain operations, raise and lower all, and notice `0x470BD214` with data 0 and 1; 7 is the levelling tool (sections 3, 4.1, 5.1, 6).

<!-- from: re-viewtools (2026-10-20) -->
- ~~**VIC2 - §5.1, §5.4:** what message `0x0BE10A20` is (the sign tool answers it with a screen point while typing)
  and what framework service `0x27EBFFFD` (interface `0x07EC0010`, slot 18 reads text) is. [read as an input-method
  position request and the clipboard]~~
  **ANSWERED 2026-10-27 (re-x-ui):** MATCHES — `0x0BE10A20` is the input-method proxy's poll for a text-caret point (only sender: its tick routine, which builds it at `0x00993DE0`), and `0x27EBFFFD` / `0x07EC0010` is the clipboard manager, whose slot 18 is `GetClipboardText(out, 0)` (sections 5.1, 5.3, 5.4).

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- ~~**VIC4 - section 5.3: encoded text:** which code-page decoder and lead/trail
  validation rules are required, what happens to pending bytes on control keys,
  Escape, a new edit, or shutdown, and does pasted "newline" include CR/CRLF or
  only LF? Can OnCharacter receive values beyond one byte? [complete glyphs are
  raw code-page byte strings; primitive codec supplies segmentation only;
  backspace removes a complete glyph; literal LF is skipped; pending bytes are
  retained unless decoding completes; invalid decoding is Unsupported; byte
  input API makes no claim for wider character values]~~
  **ANSWERED 2026-10-27 (re-x-ui):** DIFFERS — the code page is the constant 8 (UTF-8, `0x00940DA8`), so only the UTF-8 path runs; nothing is validated (a lead byte `0xC0`-`0xFD` collects the next bytes whatever they are, bytes below `0x20` are dropped even inside a character), pending bytes survive Enter, Escape, a new press and shutdown, a paste skips only line feeds (carriage returns are kept), only the low byte of the argument is used, and there is no error path (section 5.3).

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- ~~**VIC7 - section 5.2 release while typing:** a new-sign press sets state 3
  (typing); is its subsequent release meant to leave state 3, or does the stated
  "otherwise idle" apply to it as well? [literal published release rule sets
  idle outside the pressed/empty-drag cases; no exception invented; typing
  operations independently tested from state 3 before such a release]~~
  **ANSWERED 2026-10-27 (re-x-ui):** DIFFERS — a release leaves the typing state as it is and sets idle only in the other states, so the release that ends the press which created a new sign keeps the tool typing (section 5.2).
