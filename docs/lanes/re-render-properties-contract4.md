| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-render-properties-contract4: RP3–RP5 owner contracts | Codex / `re-render-properties-contract4` (RE; detached worktree) | in progress | 1. Prove lifecycle, byte folding and indexing. 2. Publish spec and question answers; validate. |

## Handoff
- Goal: answer RP3–RP5 with literal instruction evidence; docs only, no engine reads/edits.
- Worktree reused at detached HEAD; initial status clean, no commits beyond main.
- Read lane format, merge train instructions, spec §§1–2 and questions RP1–RP5.
- Done: `77ff12bc` early proof: §1.1 RP3 two-view lifecycle, zero initial count, IID 1 only, failed-output preservation, zero-ref deletion and destructor flag bit.
- Done: §1.3 byte/locale/NUL proof; §2.1 valid index domain and undefined native out-of-domain accesses; §7.1–7.2 conformance recipes.
- Done: RP3–RP5 archived as answered; RP6 filed for remaining non-C/high-byte native profiles; questions index regenerated.
- Next: review spec claims against cited instructions; finish report and queue actual detached HEAD.
- Proven: U=P+4; count P+8; both QI routes return U; AddRef is non-atomic 32-bit increment; Release unsigned count <=1 deletes.
- Main source: `C:/Users/tlori/FreeCity4`; worktree is this lane directory.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-render-properties-contract4-r1-manifest.json`.
- Quiet Bash wrapper fails with Bash E_ACCESSDENIED; use bounded native PowerShell commands.
- Native C-locale tolower folds only A–Z; Find truncates at NUL; language manager can change locale. No universal encoding claim.
- No game runs or ABI proposals. RP1/RP2 untouched; RP6 is new, at `docs/specs/questions/render-properties.md`.
