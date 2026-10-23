| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5 (remaining watercraft contracts) | Codex / `re-watercraft-wc5` (RE) | blocked: coordinator STOP for reboot; handoff only, no spec changes or checks | (1) Resume WC3/WC5 audit after STOP lifted; (2) verify bow-cap offset discrepancy; (3) finish WC1–WC5 and doccheck. |

## Handoff

- Goal: resolve remaining WC1–WC5 using export and original instructions; spec text only.
- Worktree reused with detached HEAD as instructed; do not change refs.
- Done: `7b25ce6c` records initial handoff; read prior report, rules, question list, sections 2–3, 5.1.1–5.2 and 10.2. Prior WC6–WC8 must not be redone. No spec changes yet.
- STOP: coordinator guard at 2026-10-23T14:32:16 requests all lanes stop for reboot (elevated sandbox logon-session leak). No merge queue written.
- Next: finish instruction checks for `0x00569E70` before writing any behavior; then WC3/WC5, WC4, WC2, WC1. Doccheck not run.
- Important candidate correction: existing §5.1.1 calls bow cap/lift p[24]; local decomp `region_00560000.c:1847` uses +0x38C (p[18]) for cap and half-height lift, +0x388 (p[17]) for gain. Assembly page skip 50 verifies gain at 00569F6F and stack cap comparison at 00569F92; still verify initial stack cap load and lift instructions. Do not treat this as a completed answer.
- Useful original source offsets: `decomp/cSC4Watercraft.c`: inanimate 543–742, AI selection 2878, AI motion 3161, docking 3464, animation 3653, state tick 3833. `decomp/_unnamed/region_00560000.c`: 66D60 at 824, 67000 at 913, 67A50 at 1068, 67AB0 at 1091, 67B10 at 1114, 67ED0 at 1191, 67FD0 at 1216, 681B0 at 1288, 68230/682F0 following, 690D0 at 1634, 69940 at 1677, 69E70 at 1847, 6A310 at 1959, 6A440 at 2013, 6AF90 at 2285.
- Observed helpers needing spec/signature verification: 67A50/67AB0 output P minus/plus slot-17 scalar times heading; 681B0 checks four cell corners; 690D0 is manager-receiver collision search forwarding five footprint arguments to 50A310, returning first hit or null. Inanimate uses inherited +4 slot 11 cell rectangle, tests corners for occupant-grid hit, then navigability unless docking. Jump tables still require asm.
- Query export first: `python C:/Users/tlori/FreeCity4/tools/re_query.py`; vtable name lookup caps before primary table (need narrower exact target; numeric table address is interpreted as target function, not table). `callers 00572CF0` finds no direct callers; caller must be virtual.
- No REA invoked or failed. No external evidence scripts or game runs. Initial commit printed packed-refs.lock permission error but succeeded on detached HEAD; do not change shared refs/config.
- Evidence roots: main checkout `C:/Users/tlori/FreeCity4`; changes only in this worktree.
- No engine, plugin, capi, ABI-table or coverage-map edits. No game runs or ABI changes planned.
