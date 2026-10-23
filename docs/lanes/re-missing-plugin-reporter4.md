| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-missing-plugin-reporter4 (original reporter contract) | Codex / `re-missing-plugin-reporter4` (RE spec lane, detached supervisor publication) | ready: bounded docs only; PE/export identity verified; docs audit and diff check passed; builds/game n/a; ABI false; MPR1–MPR3 open | (1) Land own exact-tip docs queue; (2) coordinator may index questions; no automatic further RE round. |

## Report

Published first dedicated original behavior in `docs/specs/missing-plugin-reporter.md`, with a narrow caller-contract link in the existing `ui-windows.md` dialog identity row. Commit `f26773d7` established the first original-supported decision contract before call 20; `1ef122ae` added effect order, lifetimes, traces and questions. This final report accompanies the explicit listener type and documentation review. No earlier same-reporter body contract was found; the public dialog row supplied caller identity only.

### Measured source and scope

- Working PE: main `bin/SimCity4.exe`, machine `0x014C`, image base `0x00400000`, FileVersion `1.1.641.0`, SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, exactly matching the canonical Windows export summary.
- Read primary exported instructions through `re_query.py asm --bytes`. Independently compared every exported instruction byte in the target ranges with mapped PE bytes: post-init **532**, message **33**, equal. These are identity/byte checks, not behavioral execution counts or a 565-byte completion denominator.
- Necessary intrinsic support only: listener adjustment/QueryInterface, pointer-release helper, list-node cleanup, scalar-property adapter, set construction/growth/dedup/cleanup and deallocation thunk. Automatic helper names are explicitly corrected or bounded by instructions. Local reporter decompilation served navigation only; no REA query was needed.
- `core-local-grid-helper4` dependency is operational merged-claim provenance only. No engine, Rust, C API, SDK/plugin implementation, foreign worktree, native/game/runtime comparison, build, test, process/server/RPC change, ABI proposal/export change or subagent work occurred.

### Contract results

- Listener `R+8`, IID `0x452294AA`, exact `0xEA8AE29A` selector, conditional callback with `this-8`, and unconditional returning message AL one.
- Initial fresh set and true byte; optional two raw filter-word stores precede nullable-manager gate. Manager slot 38 receives list out-reference, null bounds, filter and `0xFFFFFFFF`; its boolean result is ignored.
- Forward occupant traversal, borrowed property views, property pointer versus scalar output channels, property `0x6A871B82`, AL-only conversion gate, unsigned full-word set dedup, and one Release per returned occupant before next-link traversal.
- Nonempty set then popup AL gate; dialog allocation/initialization/AddRef/foreign call/AL capture/Release order. Empty or denied popup returns true; dialog return byte propagates only from direct post-init. The null-allocation edge still calls the foreign dialog with null receiver; no graceful return is invented.
- Explicit list-node/sentinel and set-node/bucket cleanup, cached-service output storage and temporary Release, no inferred RAII or extra cache reference. No own once-only latch, installed-pack lookup, sorting, persistent selection or direct outgoing notification in these bodies.
- **15 literal PROPOSED / UNEXECUTED** witnesses cover asymmetric IDs/state, empty/single/multiple lists, ignored-query-boolean partial outputs, null lookup pointer, conversion-false output sentinel, duplicates, zero/max unsigned values, AL/full-register distinctions, dialog zero/nonzero result, repeated messages and bounded provider/allocation edges. No native execution credit is claimed.

### Verification and tool outcomes

`git diff --check` passed. Ignored `build-runs/reporter-doc-audit.py` passed: 15 distinct sequential trace IDs, all local spec links present, and exactly four permitted documentation paths changed. Its first version misread an inline-code call as a Markdown link (`12`); corrected it to omit code spans and reran successfully. This is a documentation audit, not a build or behavior test. Cargo/clippy/ctest and game/shadow checks are **n/a**, game labels `[]`.

Preserved tool limitations: q.py's Bash process failed `Bash/Service/CreateInstance/E_ACCESSDENIED`; default qgrep encoding failed on Unicode, corrected with Python UTF-8; optional `pefile` import was unavailable, replaced by a successful stdlib-only ignored analytic helper. Git commits printed a shared `packed-refs.lock` permission error but did create detached commits; no shared ref/config fix or maintenance was attempted. No NoApp or live-provider result was obtained or invented, and no host/admission preflight was rerun. The own manifest/guard remained binding and reported within context budget at both milestones.

### Questions and next steps

Filed **MPR1–MPR3** in `docs/specs/questions/missing-plugin-reporter.md`: provider/allocation error channels; external cache/reference/reentrancy lifetime; invalid property views and failed/successful conversion output guarantees. No existing question was answered or moved. The task's explicit ban on shared QUESTIONS edits takes precedence over generic index regeneration, so `questions.py index` was not run; the coordinator can index at integration.

Ready means this bounded first-round documentation is reviewable, not that the whole owner or foreign primitives are complete. Application/UI/dialog/network caps and known gaps remain intact. This is FIRST 1 of 3; the other two rounds remain unclaimed, with no automatic continuation. No proposed ABI TSV is needed. Own normal queue uses the final detached HEAD, `kind: docs`, `abi: false`, `game_runs: []`.
