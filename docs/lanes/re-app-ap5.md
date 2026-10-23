| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-app-ap5 (AP12–AP15 application owner closure) | Codex / `re-app-ap5` | in progress; AP12/AP15 answered, AP13/AP14 partial | Finish remaining resource/UI tables and preference ordering; rerun doccheck and enqueue when complete |

## Report

Published interface slots 29/31 and six folded getter contracts; complete directory slots 51–58;
case-insensitive Admin Enabled filtering with by-name path dispatch; and obliteration's destructive
failure behavior, independent view checks, cleanup, unreliable success result and invalid no-city call.
AP12 and AP15 moved to the answered archive and question index regenerated.

AP13 is partially specified: packed switches/group syntax, hexadecimal key/class syntax, all fifteen
resource-factory mappings and INI path selection. AP14 is partially specified: filename suffix/reuse
rules, literal SavedCity concatenation and all twelve/fourteen video setting mappings with byte widths,
order and unknown-setting preservation. Seventeen spec-level conformance vectors added in section 16.
AP13/AP14 remain explicitly open; this is not complete owner closure.

Validation: `python tools/doccheck.py check` passed; `git diff --check` passed. Cargo/clippy/CTest and
game runs n/a for spec-only work; no ABI export changes or proposed ABI rows. No game requests.
REA batch failed 7/7 (`target_unavailable`, no app open); all published evidence uses the local export,
available decompilation and instruction/constant checks against the shipped executable. No new REA
tools were added. One temporary helper's initial PE string offset calculation was incorrect and was
fixed before use; only the corrected section-table-based reads support the published constants.

## Handoff

- Goal: answer AP12–AP15 in application spec; no implementation edits.
- Done: `16700d84` getter/plugin/obliteration milestone; `3e0ee22d` video/resources/naming/vectors, AP12/AP15 archive and passing doccheck. Final consistency corrections committed after that.
- Next: finish AP13 malformed parsing, DB directory filters/order and full font/control/cursor tables. AP14 preference ordering and filename filter/cap now published; reconcile summary and archive AP14 at finish. Then regenerate index, doccheck, report and kind=re ready queue.
- Evidence: REA batch failed 7/7 with target_unavailable (no app open); using export assembly plus available decomp only.
- Facts: false is case-insensitive; SaveCityForObliteration always returns true on normal completion; failed temp-name leaves outer result undefined; no-city temp writer dereferences null after successful open.
- Helper `build-runs/inspect.py`: asm ADDR --match REGEX --skip N --take N; data ADDR...; refs ADDR (correct PE sections). It calls re_query --full internally but caps output to requested <=56 lines; do not commit it.
- Resource bodies: 004584A0 LoadRegistry, 00458100 group registration, 00456BA0 CreateDBSegmentCallback (537 instructions), 004572B0 SetupResources (1157). First two scans call slot D4 with -1 then 0 (published); next E4 directory segment; recursive DC then EC, excluding .dll before IsDatabaseFile (finish audit).
- No merge-queue entry: incomplete owner closure, continuation required. Detached HEAD commits succeeded despite a packed-refs.lock permission diagnostic; do not change shared refs/config.
- SetupWinCtrlMgr 0044D810: colours first, Arta sizes 12/20, FontStyle.ini aliases/styles, pictures group46A006B0/type856DDBAC, cursor options at E2F7 onward, captions at end. Full body not yet recorded into behavior tables.
- SetupFontSet 00453A30 has usable decomp at main/decomp/cSC4App.c lines 1020–1405; language fallback switch and Gulim/micross tests need table extraction.
- Preference DoPreferencesChanged 0044CE40 (365 instructions): pause byte first; bool argument controls language/graphics reapply; tail simulator speed and -x removal. Numeric float multiplier at A86D54 bytes c337a23d; verify exact value/rounding before publishing.
- Save sanitizer 005F9BF0 calls MakeFileNameValid 0091917D and IsFileNameCharAllowed 009180D4; whitespace collapsing documented, allowed-char/encoding details pending.
- Factory table and slot offsets verified in instruction bytes; no ABI rows proposed. Last doccheck and diff whitespace checks passed.
- Main: `C:/Users/tlori/FreeCity4`; worktree: reserved detached HEAD, commit normally.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-app-ap5-r2-manifest.json`.
- No game requests or ABI proposals. Engine access limited to expressly requested application README.
