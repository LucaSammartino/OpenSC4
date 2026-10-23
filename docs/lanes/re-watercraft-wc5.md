| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5 (remaining watercraft contracts) | Codex / `re-watercraft-wc5` (RE) | in progress: WC1/WC2 answered; WC3–WC5 partial; doccheck passed | Resume remaining owner/inherited/AI/crash-transform audit from Handoff; no owner-closure claim. |

## Report

Round 2 supplies verified spec text and conformance vectors; complete watercraft
owner closure remains unfinished. WC1 and WC2 moved to the answered archive.
WC3–WC5 stay open with explicit residuals. WC6–WC8 were not re-audited.

- `67a4f8bf`: external idle-mode writer and spawn RNG/group semantics, model
  footprint construction/failures, all 32 inanimate corner-direction cases.
- Current milestone: animation-false caller removal request, remaining tuning
  uses/unused entry 34, AI target slew, full normal visual-basis composition.
- Corrected the earlier spec's normal bow cap and lift from p[24] to p[18],
  including existing vectors. Instructions verify +0x38C for cap and lift.
- 26 new named vectors (116 total); prose vectors for an engine lane to encode.
- Final geometry milestone: §3.2 specifies endpoint outputs, maximum occupant
  extent and outside-city order/results; it preserves the broader WC3 residual.
- `python tools/doccheck.py check`: passed. `python tools/questions.py index`
  and `check`: passed. `git diff --check`: passed before this report update.
- No cargo/clippy/ctest/game runs: spec-only work. No engine files read or
  changed; no C ABI exports or ABI rows changed. No REA invocation/failure.
- Evidence came from read-only export queries, local decomp and PE constants;
  no evidence scripts or original output are committed. Detached commits print
  a packed-refs.lock permission error but succeed; do not change shared refs.
- No merge queue while continuation is required. The final ready report and
  queue at final HEAD remain the successor's responsibility.

## Handoff

- Goal: resolve remaining WC1–WC5 using export and original instructions; spec text only.
- Final round-2 checkpoint: §3.2 and WC3-G01–G03 add four helper contracts; 116 total named vectors. Outside-city corner order differs from inanimate: (min,min),(max,min),(min,max),(max,max). Continue remaining audit; no merge queue until final completion.
- Round 2 milestone: WC1 answered; manager spawn writer 0057272C, mode draw >0.5 and groups 4417/4418. Added model-footprint §5.7.3, inanimate corner tables §5.7.4; corrected normal bow cap/lift to p[18]. Added 15 vectors; questions index/check and diff check pass.
- Worktree reused with detached HEAD as instructed; do not change refs.
- Done: `7b25ce6c` records initial handoff; read prior report, rules, question list, sections 2–3, 5.1.1–5.2 and 10.2. Prior WC6–WC8 must not be redone. No spec changes yet.
- Round 2 resumed 2026-10-23; new manifest guard is within budget. Previous reboot STOP is historical. No merge queue yet.
- Next: WC3 remaining signatures/failures/inherited contracts, WC5 crash axes/contact (0056A440/0056AF90), WC4 full AI target selection, inherited animation/player-drive, wakes/model updates and far approach. WC1/WC2 are answered; do not redo.
- Current milestone: §5.1.3 covers false-return caller, p13/p14 slew, p26 contact-height gate and unused p34. §5.1.1 now supplies normal visual basis and corrected p18. Added 8 more vectors; 113 total. Questions/index/check and doccheck passed.
- Verified WC2 caller: 0050A7B0 at 0050A89C calls boat slot72; false calls manager slot8(boat,false), watercraft target 0056F800 (table A9DF38). No implicit fade state.
- Crash starting evidence: decomp region_00560000.c 2013–2283 (sink), 2285 onward (shore heel/contact). P26 strict abs(terrain-cell-height minus runtime y)<p26 at 0056B2D4–0056B2F1; proposed horizontal slide=dt*p4*v*heading; contact fallback clears +2CA at 0056B585. Contact basis still NOT specified.
- AI decomp at cSC4Watercraft.c 3161 onward has badly recovered stack arguments; use asm, not its pointer-looking arithmetic. AI helper takes one float dt (RET4), returns void. Target slew verified at 005721C4–0057230A; full target selection remains open.
- Important candidate correction: existing §5.1.1 calls bow cap/lift p[24]; local decomp `region_00560000.c:1847` uses +0x38C (p[18]) for cap and half-height lift, +0x388 (p[17]) for gain. Assembly page skip 50 verifies gain at 00569F6F and stack cap comparison at 00569F92; still verify initial stack cap load and lift instructions. Do not treat this as a completed answer.
- Useful original source offsets: `decomp/cSC4Watercraft.c`: inanimate 543–742, AI selection 2878, AI motion 3161, docking 3464, animation 3653, state tick 3833. `decomp/_unnamed/region_00560000.c`: 66D60 at 824, 67000 at 913, 67A50 at 1068, 67AB0 at 1091, 67B10 at 1114, 67ED0 at 1191, 67FD0 at 1216, 681B0 at 1288, 68230/682F0 following, 690D0 at 1634, 69940 at 1677, 69E70 at 1847, 6A310 at 1959, 6A440 at 2013, 6AF90 at 2285.
- Observed helpers needing spec/signature verification: 67A50/67AB0 output P minus/plus slot-17 scalar times heading; 681B0 checks four cell corners; 690D0 is manager-receiver collision search forwarding five footprint arguments to 50A310, returning first hit or null. Inanimate uses inherited +4 slot 11 cell rectangle, tests corners for occupant-grid hit, then navigability unless docking. Jump tables still require asm.
- Query export first: `python C:/Users/tlori/FreeCity4/tools/re_query.py`; vtable name lookup caps before primary table (need narrower exact target; numeric table address is interpreted as target function, not table). `callers 00572CF0` finds no direct callers; caller must be virtual.
- No REA invoked or failed. No external evidence scripts or game runs. Initial commit printed packed-refs.lock permission error but succeeded on detached HEAD; do not change shared refs/config.
- Evidence roots: main checkout `C:/Users/tlori/FreeCity4`; changes only in this worktree.
- No engine, plugin, capi, ABI-table or coverage-map edits. No game runs or ABI changes planned.
