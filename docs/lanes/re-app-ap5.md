| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-app-ap5 (AP12–AP15 application owner closure) | Codex / `re-app-ap5` | in progress; all requested rules published, archive/vectors/checks pending | Add conformance cases; archive AP13/AP14, run doccheck and enqueue |

## Report

Published interface slots 29/31 and six folded getter contracts; complete directory slots 51–58;
case-insensitive Admin Enabled filtering with by-name path dispatch; and obliteration's destructive
failure behavior, independent view checks, cleanup, unreliable success result and invalid no-city call.
AP12 and AP15 moved to the answered archive and question index regenerated.

AP13 is partially specified: packed switches/group syntax, hexadecimal key/class syntax, all fifteen
resource-factory mappings and INI path selection. AP14 is partially specified: filename suffix/reuse
rules, literal SavedCity concatenation and all twelve/fourteen video setting mappings with byte widths,
order and unknown-setting preservation. Seventeen spec-level conformance vectors added in section 16.
AP13/AP14 remain explicitly open; this is not complete owner closure. Commit `ecf78b6b` adds the full
preference-application order, its language fallback defect, graphics driver/depth decisions,
simulator-speed conversion, and filename filtering/encoding/256-byte cap. AP14 still needs summary
reconciliation and archiving; AP13 research below is not yet published as normative spec text.

Validation before `ecf78b6b`: `python tools/doccheck.py check` passed; `git diff --check` passed.
These checks have not yet been rerun on `ecf78b6b`. Cargo/clippy/CTest and
game runs n/a for spec-only work; no ABI export changes or proposed ABI rows. No game requests.
REA batch failed 7/7 (`target_unavailable`, no app open); all published evidence uses the local export,
available decompilation and instruction/constant checks against the shipped executable. No new REA
tools were added. One temporary helper's initial PE string offset calculation was incorrect and was
fixed before use; only the corrected section-table-based reads support the published constants.

## Handoff

- Goal: answer AP12–AP15 in application spec; no implementation edits.
- Done: `16700d84` getters/plugin/obliteration; `3e0ee22d` video/resources/naming/vectors and AP12/AP15 archive; `5bfec58d` consistency; `ecf78b6b` preference ordering and filename filtering. Last two checks predate ecf78b6b.
- Next EXACT: AP13 tables and AP14 corrections are now published; add vectors, archive AP13/AP14, questions index, doccheck, final report and ready queue.
- Evidence: REA batch failed 7/7 with target_unavailable (no app open); using export assembly plus available decomp only.
- Facts: false is case-insensitive; SaveCityForObliteration always returns true on normal completion; failed temp-name leaves outer result undefined; no-city temp writer dereferences null after successful open.
- Helper `build-runs/inspect.py`: asm ADDR --match REGEX --skip N --take N; data ADDR...; refs ADDR (correct PE sections). It calls re_query --full internally but caps output to requested <=56 lines; do not commit it.
- Resource bodies: 004584A0 LoadRegistry, 00458100 groups, 00456BA0 class/path callback, 004572B0 setup. Scan order: registry CN_DB first; slot53(-1) *.dat nonrecursive, slot53(0) *.dat nonrecursive, slot57 directory segment class090F8619, slot55 recursive '*' then slot55 directory segment, slot59/+EC recursive '*'; factories last. Verify tail beyond 457D40 before publishing completeness.
- IMPORTANT correction to previous handoff: recursive exclusion is case-insensitive .dat, NOT .dll! String at A870D8 is '.dat'; initialized header stack+68, compared at 457A53–64/457CD0. Other extensions pass IsDatabaseFile(path,1), accepted unless result -1. Packed class086D4A89; Init/SetPath/Open(1,0)/resource registration with shutdown/release on failures; nonrecursive scans do not precheck signatures.
- Directory order evidence decomp/cRZDirectory.c lines175–290: ReadDirectorySpecificEntriesIntoStringList appends FindFirst/FindNext order, no sorting; recursive lines716–830 appends current-folder files before recursing into enumerated subdirs (depth first); paths prefixed, recursion argument1 includes root files. Setup attempts CreateDirectory for absent recursive roots; creation result ignored.
- Malformed hex: RZAtoUint32HexA 0090FCEF skips leading ctype whitespace, optional0x/0X, reads at most8 hex digits, stops on first invalid, no digits=0; ignores extra digits/trailing junk (no signed syntax). Callback does not reject zero key/class or check separator validity before factory request. SplitToken 0090E98C missing ';' returns empty token and leaves source untouched; with separator returns prefix and erases prefix+separator; remainder (including extra semicolons) is path. Class0 creation typically fails/entry skipped; no whole-file rollback.
- Groups are bit flags plus a stack, NOT robust nesting validation: BEGIN pushes name and sets matching bit; END ignores supplied value and uses/pops latest stack name, clears its bit (so duplicate nesting loses outer condition). Unknown group names still push/pop but set no bit. Unmatched END accesses empty stack unchecked (458309–45). Unclosed groups continue gating later entries. Registry failures skip individual entries. LoadRegistry returns true regardless of missing file/enum results. Directory-files section stores raw key/value in CN_DBSDF, no class parsing.
- No merge-queue entry: incomplete owner closure, continuation required. Detached HEAD commits succeeded despite a packed-refs.lock permission diagnostic; do not change shared refs/config.
- WCM colours RGB (call order 44D835–DA23): 08963F6C=255,0,255;68963F6F=189,188,188;C8963F7A=255,255,255;68963F7D=0,0,0;28963FC5=128,0,0;A8963F82=0,128,0;48963F84=128,128,64;68963F8A=0,0,128;A8963F8C=128,0,128;68963F8F=0,128,128;08963F91=128,128,128;28963F93=192,192,192;E8963F95=255,0,0;28963F98=0,255,0;E8963F9A=255,255,0;C8963FA2=0,0,255;08963FA5=0,255,255;28963FA8=255,255,255;BB5B2EC6=124,144,255.
- Font setup WCM: font system slot1C(3), optional class8B3255F9/IIDA8BC2A75 via serviceEB2EA3CB/IID4B2EA3B3 presence; app primary+28 SetupFontSet; Arta(font,0,size,2,1), sizes12/20; colour serviceC416025C/IID0073283C +88(0,0,0,08000000). Size12 binds IDs68963C4C,C8963C5C,68963C54,A2CAE9DF; size20 binds28963C70. FontStyle.ini: slot55 candidate, fallback slot53(0); if neither exists temp-export key(type0,group4A87BFE8,inst2A87BFFC), export -1 clears path; temp removed. Font Aliases enum via4499D0 forwards(value,key) to FO+50; Font Styles via44D4D0 parse spec(value,outID), register WCM+18(ID,font), +9C(ID,key-string), optional table+0C(key,ID); malformed style skips.
- WCM picture sources type856DDBAC/group46A006B0, ordered instance→controlIDs (44DEB7–E2A6):46A006A2→A8963EB8,E8963EF1;144161EA→68963EBE,A8963EF3;A→C8963EF6;82B99D9D→68963ED6;62B99D31→68963ED8;42E55FD4→48963EEE;E78FFC90→E8963EDF;C2B66DAA→E8963EE5;46A006A4→C8963ED3;46A006A6→E8963EC7;46A006A7→88963EC1;46A006A8→A8963EC4;46A006A5→A8963EEA;E2B14588→E2B14588;62B19CE9→E2B14589. Null load skips that binding only.
- Cursor options (44E2F7–E4F7): framework options CustomCursors lowercased off/disabled suppress specific IDs only; cursors option lowercased off/disabled→setting15=0,bw→1,color16→2,color/color256→3; comparison literal fullColor is mixed-case and unreachable after lowercasing! Unknown leaves setting15. Setting15 values0..4 map cursor group0,1,4,8,32; other values add4A708EFB wrapping. Enumerate typeAA5C3144; matching group only; CustomCursors disabled skips IDs1,3,15–18,43401C71,68024B0E,88024B02,C8024B17,E8024B13. Register nonnull cSLCursor result by instance with WM cursor manager+18. Does not suppress all cursors.
- Captions WCM+90 IDs→SC4LoadString group6A231EAA instance (44E657–E8C7):5301814C→0A56B09E;53018149→8A56B0AF;5301814A→4A56B0B4;53018147→6A56B0BB;53018146→2A56B0BE;5301814B→AA56B0C3;53018148→CA56B0C8; final5377BE31 gets literal UTF8 ellipsis E280A6. No checked load result.
- SetupFontSet 453A30: no FO→return; FO+3C first slot55 path, then data base whole-app+A4 character pointer with Fonts appended. Special language IDs0F/11/12/13/14 only: common name substrings 'arial mt','arta','courier ld','stoclet'; extras0F='mhei gb ' (trailing space),'mhei b5','hy round gothic','cordiaupc';11='hg-gothic','mhei b5','hy round gothic','cordiaupc';12='hg-gothic','mhei gb','hy round gothic','cordiaupc';13='hg-gothic','mhei gb','mhei b5','hy round gothic', plus 'cordiaupc' only if system micross.ttf exists and FO+38 succeeds;14='hg-gothic','mhei gb','mhei b5','cordiaupc', plus 'hy round gothic' if system Gulim.ttc exists and FO+38 succeeds. Enumerates <=64 font-name records stride136 via FO+48; lowercase each and substring-match each list entry; calls FO+44(name) for every match (can repeat). Determine +44 semantics next! Vtable00AC5488 +3C=894864 AddAllFontsFromDirectory,+44=894A3A,+48=893E5B,+50=89595D AddSubstitution.
- Round 3 corrections: font +44 removes fonts; SetupResources ends with slot59 directory segment. App slot32=feature mgr,33=cheat mgr. DoPreferencesChanged processes named x cheats on a COPY, not original erasure; lowercase-x index1 removal can repeat. _ftol2 returns int64, app clamps signed low32. Font Styles gated by service +20 object and table creation.
- Main: `C:/Users/tlori/FreeCity4`; worktree: reserved detached HEAD, commit normally.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-app-ap5-r3-manifest.json`.
- No game requests or ABI proposals. Engine access limited to expressly requested application README.
