| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv15 | Codex, `re-propdev-pv15` (detached HEAD) | ready: PV15 answered; doccheck passed | Land spec; resume glue-weather-propdev-shadow5 using documented acquisition contract |

## Report

PV15 is resolved in `docs/specs/prop-developer.md` §§3.3, 8.5 with six prose conformance vectors in §12.23–28.
The blocked branch's report was read first; it has no Owner closure table. Its stated open acquisition contract
is closed: active `cISC4View3DWin` IID `0xFA47B3F9` slot 4 returns borrowed renderer IID `0x681BD529`;
renderer slot 9 returns borrowed lighting manager IID `0x681BD52A`. The getters use ECX only, return pointers
in EAX, pop no argument bytes, and have no hidden result or AddRef. Lighting IsNight is slot 9, AL only.

The SDK by-value declaration differs from the native contract. `SL::OnView3DInit` at
`0x006025C6`–`0x006025E7` establishes that the returned manager is the one the prop developer uses, so the
documented path needs no singleton-memory access. Getter-only use incurs no Release; AddRef or successful
lighting QI requires a balancing Release. Renderer shutdown explicitly shuts down, releases and clears the
manager. A retained reference keeps storage alive, not the prior city's operating context.

PostCityInit samples live IsNight before storing it and switching all props immediately, including when the
saved byte already matches. This is not the shuffled dusk/dawn batch sweep. Missing or stale observation
cannot supply a false fallback or certify an exact comparison. The vectors cover borrowed ABI, AL width,
input/output separation, acquired references, city teardown, unavailable observations and equal saved bytes.

### Evidence and validation

- All native ABI, widths and ownership claims were checked against export instruction bytes using
  `tools/re_query.py`; bounded decomp reads supplied context. No REA call was needed or attempted.
- Implementation milestone: `1e0dc409c` (spec, recovered question archive, regenerated index, lane handoff).
- `python tools/questions.py index`: regenerated index; `python tools/questions.py check`: passed.
- `python tools/doccheck.py check`: passed. `git diff --check`: passed.
- Cargo, clippy and CTest: n/a for this documentation-only lane. No engine, plugin, capi, tools or coverage-map
  edits. No game requests/runs; no shadow exactness or through-2032 measurement claimed.
- Git printed a packed-refs.lock permission diagnostic during the milestone commit, but committed successfully;
  HEAD and the clean worktree were verified. No shared ref/config maintenance was attempted.

### Questions and proposed ABI rows

PV15's entry was absent on this base, so its original wording was recovered from
`glue-weather-propdev-shadow5:docs/specs/questions/prop-developer.md` and archived directly with its provenance
and **ANSWERED 2026-10-23 (re-propdev-pv15):** DIFFERS rule. No open prop-developer questions remain here.
When integrating with the blocked lane's branch, retain the answered entry and remove that branch's open PV15
copy before regenerating the index.

Six proposed `interfaces.tsv` rows are in `build-runs/abi-rows-re-propdev-pv15.tsv` in that file's format.
They refine the view and renderer getter returns and add lighting QI/AddRef/Release/IsNight contracts.
No `docs/abi/` file or C ABI export changed.

## Handoff

- Goal: close PV15 for glue-weather-propdev-shadow5 with behaviour specs only.
- Done: `1e0dc409c` supplies the contract and six conformance vectors; question/index and doccheck pass.
- Next: coordinator lands docs, reconciles blocked branch's open PV15 copy, and resumes implementation/testing.
- Proposed ABI rows remain in this worktree's ignored `build-runs/abi-rows-re-propdev-pv15.tsv`.
- No outstanding game labels, no measured shadow verdict, no C ABI changes.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv15-r1-manifest.json`.
