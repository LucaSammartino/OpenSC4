| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-fontsys4: completed cGZFontSys verification | `verify-fontsys4` / detached reserved worktree (engine lane) | in progress | 1. Establish published owned factory/QI/lifetime route within 15 calls. 2. Verify production FontSys or stop on precise contract gap. |

## Handoff
- Goal: verify only completed cGZFontSys, drawing-fonts section 12/12.1, against independent original.
- Worktree reused, detached HEAD; initial status clean and no commits beyond main.
- Baseline qbuild all launched; inspect build-runs/qbuild-* and fc4verify-* for result.
- Published route established: spec section 1 factory creates a new 0x70-byte instance; SDK cIGZCOM GetClassObject, service IID 0x287FB697 then primary QI 0x00673960 (this+4 relationship); section 12.1 supplies QI/refcounts/Shutdown/destruction. Typed SDK MSVC calls supply thiscall; no private getter.
- Added opt-in owned route probe, balanced acquired/created views, Init/idempotence, negative QI, empty catalogue/cache, Shutdown/re-init/final Release; awaits build and dynamic check.
- Next: complete baseline, quick-build the owned probe, then dynamic factory check and production Rust bridge.
- No game request yet; no plugin changes, native calls, ABI changes, or binary commits.
- FN18-FN22 excluded. Do not read analysis sources or run tools/ scripts.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-fontsys4-r1-manifest.json.
