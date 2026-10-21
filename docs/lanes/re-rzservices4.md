| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzservices4: spec cRZLanguageUtility (RZ locale service: names, separators, currency, number/money/time/date formatting, attributes, lifetime) | Claude / .claude/worktrees/re-rzservices4 | ready: docs/specs/language-utility.md | engine lane may implement from the spec; RE: answer Q1 (currency flag writer), trace callers (Q2) |

## Report

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
