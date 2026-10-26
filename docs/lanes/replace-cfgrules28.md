| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules28 graphics rules (engine) | Codex / .claude/worktrees/replace-cfgrules28 / detached HEAD; reserved branch replace-cfgrules28 | wip; merged main 8e49c9ce3; provider/native source 2df6aad3f; ABI unchanged; quick PASS 670 tests / native 150 edges; final gate running | Measure original/replacement queries and acceptance (a)-(c); final gate; enqueue only on measured success. |

- GR9 implemented: native twenty-draw FSAA/readback, reversible backing-store presence, executable resource/Windows/CPU acquisition and optional sound strings; unavailable acquisition refuses Init.
- Added GZCOM class 0x0A1D8285/IID 0x0A1D8284 over the production Rust owner; default-off `cfgrules`; ABI exports/version unchanged.
- Query/property tracing and default/enabled/veto registration tests added; original tracing uses the default-off compare registry.
- Quick build PASS: 670 tests, native 150/150 edges; owned Rust fmt PASS. Full qbuild all running; current gate not yet measured.
- Acceptance query off/on: unmeasured.
- Acceptance (a), current house list + cfgrules through 2032: unmeasured; ref_warnings unmeasured.
- Acceptance (b), golden plain and generated-action to 2026-04-01: unmeasured.
- Acceptance (c), every regress shadow key vs regress-train267-shadows.log: unmeasured.
- Existing portable profile limits (UTF-8 input and nearest/even numeric profile) remain; no new question filed yet.
- cfgrules is not ready for the replace list. No ready/train enqueue or coverage credit.
- Next: complete the final gate, measure all game criteria and query parity, then publish/enqueue the actual HEAD.
