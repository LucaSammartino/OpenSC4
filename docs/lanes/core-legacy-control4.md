| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control4 (portable LegacyControl owner) | Codex / core-legacy-control4 | in progress | Audit 45 named entries, independent fixtures, implement and gate |

## Handoff
- Goal: first portable LegacyControl owner from published legacy-control sections 1–12 only.
- Worktree reserved, detached HEAD reused; initial status clean, no commits beyond main.
- Read rules and lane format; no forbidden sources opened.
- Task-specific validation overrides baseline request: no unchanged baseline full build; one focused quick, touched-file fmt, one stable final all.
- No GUI/game/network, C ABI, plugin or capped-family changes permitted.
- Audit complete: exact 45 contracted owner functions; Custom shared stub is additional, not owner credit.
- Frozen independent expected fixtures BEFORE implementation: 393 ordered vocabulary entries plus lifecycle, typed scans, fallback, two-pass, resource, Tree/Grid and boundary effect vectors.
- Done: fixtures commit 02bd46f2 precedes all production source.
- Production milestones: bf469091 (owner implementation), d8d3da6f (22 conformance tests, finalized source and engine-format configuration).
- Done: portable lifecycle/build, 21 vocabulary tables/maps, common + seven published type bodies + Custom shared stub; fragment wrapper/frame/common with explicit LC8 coverage.
- Done: added full-builder fixtures for all seven published widgets, fragment wrapper/common/LC8, FileBrowser defaults/filter index, Tree image/root/font, Grid stale-scan/scratch and resource ref effects.
- Validation pending: first quick lifetime compile failure and second quick formatting-header failure preserved in build-runs/fc4verify-quick-test-{first,format}-failure.log. Fixed both. Third quick queued on Cargo lock (session 75463); do not start another quick.
- Formatting: first invocation used wrong relative paths; corrected absolute paths expose engine rustfmt configuration mismatch. Applying engine configuration only to new source; shared lib.rs must not be mass-reformatted.
- Current quick evidence is NOT a pass: overlapping earlier failed invocations left NUL gaps/old errors in quick-test.log; intact failure snapshots retained. Never trust that mixed log as final source evidence.
- Next: ONE all on committed source via q.py (full wrapper console persisted under build-runs/q); no source edits while it runs. Final touched-file fmt invocation pending in session 60998; new files were formatted with engine max_width=120, shared lib.rs preserved.
- The q.py all transport FAILED BEFORE qbuild started: build-runs/q/20261023-000344-12404.log is UTF16 Access denied, Bash/Service/CreateInstance/E_ACCESSDENIED. Do not interpret this as a cargo/all result.
- Direct PowerShell qbuild.py all will start immediately after this Handoff commit (first actual all); no source edits while it runs. No game requests or queue until stable passing gate.
- Full 45-entry report draft and foreign-helper/domain audit: build-runs/legacy-control-report-draft.md. Gate/count fields deliberately pending; copy into final lane report only after actual evidence.
- LC9/LC10 filed for unsafe/malformed/duplicate domains; allowed indeterminate-field choices labeled in code.
- No baseline/full or game runs performed. Existing shared Git packed-refs.lock warning occurs during commit, but detached commits succeed; do not change shared config/refs.
- Unknown LC4 (12 interpreters), LC5 opaque default parent, LC8 (19 foreign writers) stay explicit.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-legacy-control4-r1-manifest.json.
