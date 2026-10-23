| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisors-ad5: advisor record and field contracts | Codex / `re-advisors-ad5` (RE lane) | ready: AD11/AD16/AD17/AD18 answered; doccheck and questions check pass; no ABI changes | Engine lane: register city-situation advice records; preserve strict mismatches and report transition conformance separately; implement required vectors. |

## Report (2026-10-23)

Resolved every open row in `docs/specs/questions/advisors.md`, moved the original entries with answers to
`docs/specs/questions/answered/advisors.md`, and regenerated `docs/specs/QUESTIONS.md` (advisors: 0 open,
18 answered, next AD19). Behaviour changes are in `docs/specs/advisors.md`; no engine, plugin, capi, ABI table
or coverage-map changes were made. No ABI rows are proposed.

- **AD17 (§1.1, §11.3.1):** CitySituationAdvice CLSID `0x8BE3753B` uses exactly the base version-4 advice
  record. The original serializable tables at `0x00A84508` and `0x00A80884` share Write `0x00408090` and
  Read `0x004082F0`. Record-only plain-advice registration is sufficient if it preserves the subclass CLSID,
  reference identity and advice IID. No subclass stream fields are needed; this does not authorize plain
  live city-situation behaviour.
- **AD18 (§11.3.2):** clean advice Read/Write necessarily differs at flags byte 10 because Read sets `0x10`.
  Strict comparison keeps that mismatch. A separately labelled load-transition check may expect flags OR
  `0x10` while checking every other byte. Restoring a bit for diagnostics cannot earn strict-exact credit.
- **AD16 (§6.9.1):** both Init and refresh use Lua ToString and the null-safe string setter. Numeric type 3
  formats a double with `%.14g`; other non-string types become empty. Scratch is cleared per field, and no
  `__tostring` runs. Spec distinguishes fresh empty Init from a registered field becoming empty on refresh.
- **AD11 (§4.2):** static startup constructs the SKU-specific string empty and does not fill it; Sku_Data is
  a different string. Corrected interface/class offset explanation and specified the empty relative
  `advisors\` search. Host integrations must call slot 55, even when empty. No runtime path read is claimed.

Added required conformance cases to §17 items **32–35**: subclass reference graph/framing, dirty and clean
flags including a high bit and an unrelated-byte negative case, malformed/number/NUL text cases, and empty
versus supplied SKU directory. These are implementer-facing vector requirements, not executed engine tests.

Evidence came from main-checkout `tools/re_query.py` queries, bounded decomp/export reads, and a read-only
PE vtable check (ignored helper `build-runs/read-vtables.py`). REA was not called: the brief states no binary
is open. Both available shadow lane reports were read; neither contains the requested Owner closure table
or `open:` rows, so their AD17/AD18 question descriptions were used as the closure targets.

Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS. The first whitespace
review found trailing empty lines in the now-empty question file; those were removed for the final check.
Cargo/clippy/CTest and game runs are n/a for this spec-only lane. Earlier game run
`adv-shadow5-20261023-r1-2032` remains **0/73 strict exact**, as reported by the engine lane; this work does not
reclassify it. No game/build request is outstanding.

Milestones: `0a63ac2c` resolved AD17/AD18; `bc80273b` resolved AD11/AD16. Git printed a shared
`packed-refs.lock` permission diagnostic during commits but created both detached-HEAD commits successfully;
no shared refs/config were changed by this lane.

## Handoff

- RE work complete; no open advisor questions remain.
- Engine follow-up: register `0x8BE3753B` as a base-record adapter preserving captured CLSID and references.
- Keep strict byte comparison; add a separate transition-conformance result if useful. Never count restored flags as strict exact.
- Implement §17 items 32–35; then the engine lane owns its full gate/game rerun and load-date fix.
- No ABI, game or build requests pending; no native malformed-field or runtime SKU-path measurement claimed.
