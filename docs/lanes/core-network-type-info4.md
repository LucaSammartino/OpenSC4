| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-network-type-info4 (bounded static record lifecycle; FULL network-occupants + network-tools scope) | `Codex` / `core-network-type-info4` (engine lane) | WIP: implementation 07d6b4ea; 18-body boundary 10 specified / 8 partial; fmt 3 files PASS; focused logs 3340 unit + 13 doc passed on 80b4cfa1, wrapper stalled in owned stats child; final all NOT RUN; ABI unchanged; no game | 1. Collect existing session 85444 after stats stall. 2. Canonical all on committed engine 401165a4. 3. Qualify gates before readiness. |

## Report

Core-only FIRST bounded TypeInfo lifecycle implementation, dependent on landed research merge 3ff8a10a.
Scope keys include FULL network-occupants and network-tools (the outer reload belongs to NetworkTool), with network-geometry integration. This is not the earlier occupant/record continuation or a completed NetworkTool/TypeInfo owner.

### Changes and boundary

The new network_geometry::type_info module owns a finite thirteen-record process-array model: static zero origin, constructor-only derived words, forward construction/exit registration, reverse explicit disposal, loaded byte, metadata/widths, content Clear, retained logical backing identities, placement admission/writes and ordered outer reload. It never dereferences native pointers. Foreign calls accept opaque identities and return raw tokens, callback entries, individual property writes and booleans; they cannot return a cleared/reloaded owner.

NativePlacement reuses PlacementParams semantic fields in private storage, with a typed radians interpretation and an explicit degree_view boundary for existing geometry consumers. TypeRecord::geometry_view reuses TypeInfo and substitutes the owned derived words; old tunnel arithmetic retains its previous approximation qualification. Existing degree-valued callers/defaults are unchanged. Logical buffer identities establish retention/free order, not native Vec layout or capacity.

The loader preserves absent scalars and P+28, resets A4/A5/A6/A7 lengths individually, retains partial writes on false results, appends A4-count fallback elements to false A5/A6 results, performs repeated admitted angle conversion, applies exact defaults and preserves the cross-field EA86E3B5 failure effect. False whole acquisition leaves placement untouched and does not release the raw out token. Normal false acquisition still reaches derivation and completion in the supported zero-angle domain.

TI1 wrong/malformed property variants, TI2 nonreturning/null allocation/unwind behavior, TI3 nonzero/nonfinite derived arithmetic and x87 environment, and TI4 support-key duplicate policy remain explicit Unsupported/foreign domains. Nonzero derivation returns a source offset and completed-store count, retaining earlier stores and withholding later publication. No host f64 tangent is advertised exact. V27 asserts only the reached AddRules(null) prefix, never safe successful null completion.

### Per-body ledger

The published inventory remains exactly 18 bodies = 10 specified / 8 partial. Bounded implementation/effect tests below do not upgrade specification status or count thirteen records as thirteen owners.

| Body | Published status | Implementation / effect evidence |
|---|---|---|
| 00641F70 constructor | specified | bounded implemented; V01 literal static/constructor state |
| 00A73D10 static initializer | specified | bounded implemented; 0..12 support setup and exit registration |
| 00A7C140 reverse disposal | specified | bounded implemented; V04 reverse record disposal |
| 006421D0 Clear | specified | bounded implemented; V02/V03 order, null skip, repetition and retained backing/state |
| 00643510 destructor | specified | bounded implemented; V04 placement/bucket/bridge/evaluator storage free order |
| 0063BF90 LoadComplete | specified | bounded implemented; first/later byte visibility and partial publication |
| 00623310 outer reload (NetworkTool) | specified | bounded implemented within admitted primitives/zero numeric domain; exact order/metadata/widths; power line skips content/derive |
| 0063CBE0 placement loader | partial | bounded writes effect-tested; TI1 conversions foreign/unknown; nonfinite conversion outside admitted domain |
| 0063D460 placement/derivation wrapper | partial | false-acquire continuation and exact zero stores effect-tested; TI3 nonzero Unsupported with partial ledger |
| 0063D360 evaluator loader | partial | 16-byte allocation/factory/AddRules/append effect-tested, including returning false and direct duplicates; TI2 null reached prefix only |
| 0063F780 supports wrapper | partial | forwarding/raw insertion and subsequent ordered clear effect-tested; TI4 duplicate/parser internals foreign/unknown |
| 006437A0 bridge loader | partial | callback accumulation, false enumeration, temporary destruction and specialization order effect-tested; parser/specializer content foreign |
| 0063D3E0 conflict wrapper | partial | forwarding effect-tested, no preliminary clear; never called by outer reload; callback contents foreign |
| 0063FAF0 verifier wrapper | partial | separate-store clear-before-enumeration effect-tested even with zero callbacks; contents foreign |
| 00624F70 placement-vector destruction | specified | bounded implemented; A0,6C,60,54,28 backing frees effect-tested |
| 00641EB0 bridge full-range erase | specified | bounded implemented; forward per-element destruction effect-tested |
| 006412B0 general bridge suffix move/copy | partial | empty suffix only in Clear; no general algorithm implemented/credited; unknown |
| 0063D760 support clear | specified | bounded implemented; ascending buckets/chain order, +14 then +8 then node free, null skipping/repeated clear effect-tested |

Evaluator factory 00622A90, foreign bridge destruction/parser/specialization, support parsing/map duplicates and global verifier/conflict callback internals remain foreign/unknown here. No old twenty-row/byte reach credit, solver/evaluator/model/INI algorithms, tool replacement or whole-owner credit.

### Validation and failures

Thirteen production-calling Rust tests (executed on 80b4cfa1 before the final borrowed-holder refinement; the final-source assertion awaits the full gate) use literal/trace expectations based on TI-V01–27, not a test-side owner or owner-returning host. These are bounded conformance cases and variants, not original-game differential execution. They cover constructor/power-line retention; evaluator reverse/null skipping, bridge forward and support free order; repeated Clear and logical backing retention; first/second loaded visibility; partial publication; false acquisition versus admitted absent-angle reconversion; defaults/cross-field conflict; absent/empty/short vectors; partial false A5 [99,0,0]; duplicate direct loaders; final verifier clear; typed geometry view and TI1/TI2/TI3 boundaries.

Baseline session 72044 returned RC1: workspace Rust 5385 passed, clippy 0 diagnostics, native build 217/217 edges PASS, ctest 67/68 FAIL after source moved. New ctest caught an incorrect initial 35-degree test literal; fixtures now use the published V16/V17 30-degree words. This mixed-source baseline is not a qualified pass.

First quick session 25431 returned RC1: no successful test result lines, 30 test/15 dependent compiler error lines, native 1/4 edges FAIL. The formatting helper had included rustfmt's filename header in new source. Headers were removed and the helper fixed. Source movement in that failed quick is preserved. Lossless failure archives and per-log SHA256 headers are under build-runs/baseline-evidence/ (logs.pickle.zlib.b64 and quick-failure.pickle.zlib.b64). No validator was canceled, killed, or replaced with a concurrent duplicate; process/parent CIM inspection was denied by the sandbox.

Formatting: qbuild fmt on all three touched Rust files passed, exit 0. The canonical final-source all gate has NOT started: the prior focused wrapper remains active in its optional sccache stats step, and duplicate checks/RPCs and process cancellation are forbidden. Readiness is withheld.

### Questions and next steps

No new questions filed. Existing TI1–TI4 bound unsupported domains. No C ABI, CAPI/plugin, default enablement, maps, generated coverage, shared-state, native game, shadow or promotion changes. Game runs: n/a by task scope.

After qualified gates, review/land only the bounded core implementation. TI1–TI4 and foreign content/numeric bodies need separately scoped specification/implementation before extending the envelope. No native replacement or promotion is requested.

### Blocking gate handoff

State: WIP, not ready to land. Final implementation source commit 07d6b4ea; engine tree 401165a4445644af9c1a8f4c54ba47760bb168c5. Working source is committed and formatting passed. No final-source Rust/clippy/native/ctest PASS is claimed.

Owned focused terminal session **85444** is still active. Exact read-only ownership proof is in `build-runs/type-info-gate-stall.json`: qbuild PID **32164**, parent **38048**, created epoch **1792730117.38301**; child fc4verify PID **27760**, parent **32164**, created **1792730117.4194903**; its child sccache PID **26980**, parent **27760**, created **1792730250.704298**, command `sccache.EXE --show-stats --stats-format json`, zero user/system CPU at capture. Full command lines, UTC creation times, source identities and actual source/log SHA256 hashes are in that file. No additional cache RPC, checker, kill, or validator cancellation was issued.

The focused Rust log reports **3340 unit + 13 doc tests passed**, dependents compiled, and native linked 4/4, all before the final holder refinement. Native completion alone is NOT a wrapper PASS. The run launched on 80b4cfa1 / engine b35ff68f84c8d9fe2e84987056a4961a447b24f7 and source subsequently moved to the explicit holder refinement; preserve that qualification even if the wrapper later returns.

Next exact steps: (1) collect the existing session 85444 to terminal when the external stats stall resolves; do not duplicate or cancel it; (2) verify engine source still equals 401165a4445644af9c1a8f4c54ba47760bb168c5; (3) run the ONE canonical `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-network-type-info4 all` on unchanged committed source and collect its actual terminal Rust/clippy/native RC/ctest verdict and log hashes; (4) only then revise the row/queue to ready if qualified. This handoff does not authorize canceling the stats child or starting a concurrent validator.
