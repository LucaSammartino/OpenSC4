| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacyctl-lc16 (LC14/LC15 native contracts) | `Codex` / `re-legacyctl-lc16` (RE lane, detached HEAD) | ready: LC14/LC15 answered, LCV44–50; doccheck PASS; shadow12 merged with two implementation conflicts preserved from current HEAD; ABI unchanged | 1. Review merge conflict choices before implementation integration. 2. Implement full replay from sections 6.1–6.2 and 7.2. 3. Continue LC8. |

## Report

Merged `glue-legacyctl-shadow12` (`ab3654c1c`) in `6301e53bb`, bringing its questions and newer run 520 evidence. The merge conflicted in `engine/capi/src/lib.rs` and `engine/plugin/FC4Config.cpp`; both were resolved with the pre-merge current HEAD versions without reading their contents. All other branch changes were retained automatically. This is an integration limitation: the coordinator must reconcile the shadow lane's intended changes in those two files and run implementation gates before treating the merged shadow as build-validated. No implementation files were manually authored by this RE lane. Git reported a denied packed-refs lock during commits, but each detached-HEAD commit completed successfully; no refs/config/maintenance commands were used.

LC14 (`6e0c92613`): section 7.2 now specifies both methods as void x86 thiscall entries on the +8 legacy interface, including equal-pointer and all null paths. Reference ordering and slot identity are observable; EAX/AL residue is not a return value. LCV44–45 exercise those paths.

LC15 (`9d5565e29`): sections 6.1–6.2 identify ScriptTag's confirmed primary-vtable call at `0x0094BADA`, the six-byte SetParameters entry prefix and sole return, cdecl inputs for Common/type callbacks, and byte-verified displaced prefixes for Common plus all 19 non-empty type interpreters. SetParameters has zero exported direct callers and one data reference at `0x00AD536C`; these counts are not an enumeration of runtime indirect callers. Published builder call spans cover Custom's no-op invocation: its shared one-byte return cannot receive a five-byte entry patch and cannot uniquely identify arbitrary callers as Custom. This is a settled identity limitation, not an unanswered property behavior. LCV46–50 cover virtual/direct/standalone calls, no-op/missing callbacks, failure and duplicate observation accounting.

Both questions were moved intact to `docs/specs/questions/answered/legacy-control.md` with dated answers; `tools/questions.py index` regenerated `docs/specs/QUESTIONS.md`. Only LC8 remains open in the legacy-control question file. No new question, ABI row proposal, ABI export change, version bump or coverage-map edit. No REA query was attempted: the session has no binary, so evidence comes exclusively from main-checkout `tools/re_query.py`, exported instruction bytes and targeted decomp ranges. Local evidence: `build-runs/lc16-boundaries.log` and helper `build-runs/lc16-boundaries.py` (untracked RE scratch, not deliverables).

### Owner closure table audit

Read the specifically requested main-checkout `engine/core/src/legacy_control/README.md` Owner closure table first, before native investigation. Every row still labeling a question open is listed below; the README itself was not changed. Except LC8, these labels are stale relative to the existing answered-question archive and spec rules. This audit reports documentation labels, not implementation conformance.

- Section 6 Build sequence: LC5 (opaque default parent), already answered in section 6.
- Section 7.5 Property readers: LC9, already answered in sections 7.5/13.3.
- Section 7.6 Fragment serialization: LC8, genuinely still open for exact per-type writer emission conditions/value sources/repeated records.
- Section 9.2 TreeView: LC10, already answered in section 9.2.
- Section 9.4 Image/font metadata helpers: LC13 (named-font branch), already answered in sections 9.4/9.9.
- Section 9.5 Grid: LC10, already answered in section 9.5.
- Section 9.9 Shared drawing inputs: LC13, already answered in sections 9.4/9.9.
- Section 9.11 TextEdit: LC11, already answered by section 13.3's unsupported-edge policy.
- Section 9.14 Btn: LC11, already answered by section 13.3.
- Section 9.15 BMP: LC11, already answered by section 13.3.
- Section 9.17 LineInput: LC11, already answered by section 13.3.
- Section 9.18 Spinner: LC11, already answered by section 13.3.
- Section 9.19 Ticker: LC11, already answered by section 13.3.
- Section 9.20 SC3/OptGrp: LC11/LC12, already answered in sections 13.3/13.1 respectively.
- Section 11 Open questions: LC5, LC8–LC13; only LC8 remains open.
- Section 13 Message/drawing/failure boundary: LC9–LC13, all already answered; see sections 7.5, 9.2, 9.4, 9.5, 9.9, 13.1 and 13.3.

### Validation and limits

`python tools/doccheck.py check` PASS, foreground, captured in `build-runs/lc16-doccheck.log`; `git diff --check` clean after removing an extra question-file trailing blank. Initial q.py launch failed before doccheck with Git Bash CreateFileMapping error 5; the corrected foreground Python runner `build-runs/lc16-gate.py` completed exit 0. No cargo/clippy/CTest/game gate was run by this RE lane. Inherited shadow12 run `legacyctl12-owner-2032-971d183e3-r1` (520) was clean through 2032, owner-only `legacyctl 72/72` exact, compared=7976, builds=775, build_getters=775. Those are inherited measurements, not measurements of this merge or full property replay. No game request is outstanding.

## Handoff

- Goal complete: LC14/LC15 behavior and native observation contracts published; no split question required.
- Commits: `6301e53bb` merge, `6e0c92613` LC14, `9d5565e29` LC15.
- Next: coordinator reviews two preserved-current implementation conflict choices and gates the integrated shadow; implementer uses sections 6.1–6.2/7.2 for full replay.
- Only LC8 remains open; owner closure README's other open labels are stale (all enumerated above).
- No game request outstanding; inherited run 520 is owner-only, 72/72 exact.
- Doccheck passed; queue tip is the final report commit, full hash in build-runs/merge-queue/re-legacyctl-lc16.json.
