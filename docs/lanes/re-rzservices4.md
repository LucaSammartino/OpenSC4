| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzservices4: round 3: cRZDirectory, cRZOFStream, cRZCmdLine specs (framework-rz-files.md ext / framework-rz-services.md) | Claude / .claude/worktrees/re-rzservices4 | in progress | see Handoff |

## Handoff (round 3, 2026-10-21)
- Goal: specify cRZDirectory, cRZOFStream, cRZCmdLine (then cRZSysServPtr if time); exact byte-supported contracts.
- Done: rounds 1-2 (language-utility.md, framework-rz-files.md §1-5).
- Next: cRZDirectory from decomp/cRZDirectory.c -> framework-rz-files.md §6.
- Facts: language-utility.md untouched (strings lane). No ABI edits.


## Report (round 2, 2026-10-21)

- New spec docs/specs/framework-rz-files.md (commit 193d6a74): cRZFile vtable 0x00ABFED0 (33 slots
  mapped), layout, Open modes/creation/share and .lnk resolution, logical vs physical position, read and
  write buffers (sizes, clamps 4..0x1F4000), Seek/Length/SetLength/Flush/Close, error codes
  (5 -> 0x20000002, 183 -> 0x20000001), path operations and static helpers; cRZIniFile vtable 0x00ACF150
  (20 slots), ownership of the cIGZFile, line reader (CR/LF runs collapse, blank lines invisible), syntax
  (; comments, first '=', space/tab trim, case-insensitive via ASCII or UTF-16 lower), duplicates (first
  section header and first key win; enumeration lists all keys), sscanf %ld numbers, upper-case hex
  binary, open retry (250 ms, 3 s), writeback rules (replace in place, insert before next header, append
  without newline fix-up, new section separators), worked examples.
- Instruction-verified: sleep unit (/1000 at 0x009205A6), retry compare (cmp 3 at 0x0091C681), buffer
  clamps (0x00919081), +0x38 not set by two ctors, format strings ("=", "%ld", "%s = %s%s", "[%s]%s",
  CR LF) read from exe bytes.
- REA: unavailable ("No app is open"); used re_query.py, decomp/ and exe bytes.
- Questions (spec §5, RE-side, none filed): Q2 readers of cRZFile +0x30, Q3 runtime INI round trip,
  Q4 allocator zeroing for +0x38.
- Language spec (language-utility.md, round 1) untouched, no new claims on its owner.

## Report (round 1)

- Owner chosen: `cRZLanguageUtility` (vtable 0x00AD1250, 33 slots; the largest RZ class with no spec, 106 KB
  decomp, only an INVENTORY-M6 F6 mention), plus the parts of `cRZLanguageManager` that create, hold and
  configure it (embedded instance at +0x38, `GetLanguageUtility`, `GetNewLanguageUtility`,
  `SetCurrentLanguage` / `SetCurrentCountry` forwarding). Manager folders/locale stay in data-formats §6.2.
- New spec docs/specs/language-utility.md: §1 identity, layout, lifetime, encoding; §2 language-ID list and
  attribute queries (driving side, measurement, name order, IME, multibyte, code-page tables); §3 currency
  symbol and separators; §4 grouping, number, money, formatted money, formatted number, time, date, case and
  compare; §5 full day/month name tables (57 rows) obtained by running the original getters' instructions
  concretely for language 0..0x2D x index 0..13 (helper build-runs/emu.py, not committed); §6 questions.
- Notable findings: currency flag +0x10 is never set by known code (all money uses `$`); negative
  MakeNumberString returns an empty string for Portuguese, Brazilian Portuguese and IDs >= 0x15; Finnish
  trailing minus; euro symbol stored as `E2 2C 82 2C AC`; Japanese full month name for February missing;
  Russian code page resolves to 1250; MakeFormattedMoneyString prints 0.0 as -0.0; framework code page is the
  constant 8 so UTF-8 conversion is a no-op.
- Instruction-verified: SetLanguage/SetCountry/GetMeasurementSystem bodies, currency tail, money and number
  sign/language branches, formatted-money sign tests, date argument order (default vs D/M groups), case-length
  source, formatted-number call. MakeDateString styles 2..5 for CJK/Japanese and time formats are from
  pseudocode (consistent with the format-string pushes).
- REA: unavailable ("No app is open"); used re_query.py, decomp/ and exe bytes instead.
- Questions: Q1 currency flag writer, Q2 callers/styles, Q3 C runtime output for -0.0 (spec §6; none filed in
  docs/specs/questions/, they are RE-side).
- Commits: cf0401c7 spec; final commit adds the verified Q4/Q5 rules and this report.
