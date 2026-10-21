| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-strings4: strings and language spec (round 2) | Claude, .claude/worktrees/re-strings4 | ready: strings-language.md reconciled + cRZLanguageManager | spec cRZString (§5..§8), then cSCStringDetokenizer |

## Report (2026-10-21, round 2)

Preflight GO. REA unavailable (no app open); Ghidra export, decomp/ and exe bytes used. No questions filed.

What changed (docs/specs/strings-language.md, commit 6de91727):
- §3 reconciliation: the cRZLanguageUtility contract now lives only in re-rzservices4's language-utility.md
  (canonical, covers all 31 functions incl. MakeDateString, MakeMoneyString, MakeFormattedMoneyString,
  SetCountry). My duplicate §3-§7 tables were removed. Row-by-row comparison (79 name-table rows plus currency,
  separator, flag, number, time tables): every disagreement settled from bytes in favour of language-utility.md
  (Swedish abbr Monday `4D E5 06 45`; index low byte only, 00925a53/009265df; MakeTimeString with negative
  minute). Additions it lacks: charset selector asks 1251 twice (00925780, 0092578c); int64-min MakeNumberString
  gives a doubled minus; currency flag +0x10 written only by the utility ctor (0092849f) within both language
  classes (partial answer to its Q1).
- §4 cRZLanguageManager, all 47 owner functions: service ID/QI (iid 1 not answered), state, ctor,
  SetCurrentLanguage (setlocale + _setmbcp; returns false after switching for IDs >= 0x2D), SetCurrentCountry
  forwarding, available-language set, LANGID maps both ways (full 16-bit table), English/local/directory names,
  ISO 639 codes, runtime locale names, country names and ISO 3166 codes (1..0x38), primary language / default
  country maps, reverse lookups, money stubs (rate 1.0, identity converts).
- Quirks recorded: GetLanguageLocalName's empty test is inverted (every name becomes `Untranslated`/false, so the
  local-name lookup never matches); GetLanguageIDFromEitherEnglishOrLocalName calls itself forever; slot 46 lists
  only every other available language; `Hungray`, `UnitedStates` fallback, `uk` for UK English.

Owners: cRZLanguageManager fully specified (here); cRZLanguageUtility fully specified by language-utility.md
(credit it there, not here). Not started: cRZString (82 fns, 8640 B), cSCStringDetokenizer*,
ConvertStringEncoding/UTF16 helpers. No cRZLanguage* wildcard credit beyond these two owners. No ABI rows.
Tool written: build-runs/vm.py (concrete interpreter for integer/string switch functions; not committed).

Next: §5..§8 cRZString (vtable 00a80810; decomp/cRZString.c), then the detokenizer.
