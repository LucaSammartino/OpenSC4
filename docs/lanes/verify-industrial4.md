| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-industrial4: constructed native industrial comparison | `verify-industrial4` / `.claude/worktrees/verify-industrial4` (reserved lane, detached HEAD) | ready, round 2/3; constructed 263052/263052 exact; cargo4938/clippy0/ctest68; 3 final game controls clean2032/ref0; ABI unchanged | Coordinator review/land narrowed verification tooling. IN3 natural interception remains excluded; no replacement promotion. |

## Report

Implemented an opt-in, constructed-native industrial comparison. With `industrial_shadow = 1` and industrial replacement disabled, each suite acquires an independent ORIGINAL through public GZCOM CLSID `0xA990C022`, IID `0x477A9E7D`, and creates a separate Rust `fc4_industrial`. Native operations call the published typed interfaces; Rust operations call its existing C ABI. No original operation implements Rust behavior, and no second Rust instance serves as an oracle.

### Scope and implementation

- New `engine/plugin/FC4IndustrialShadow.cpp/.h`; additive startup/observer hooks and CMake sources. No core behavior, C ABI exports/version, defaults, maps or ABI tables changed.
- New `engine/plugin/compat/FC4IndustrialCompat.h` validates the identified GOG 1.1.641 support image (timestamp/base/size), published primary/message/serializable vtable identities, code-page membership and published method addresses before native operations or initialized-byte snapshots. It uses only the published sections 1, 2.2 and 9 layouts; no executable-file inspection, scanning or table mutation.
- Industrial replacement enabled: native comparison REFUSED before factory acquisition. Shadow off: no probe. Factory/image/view/core unavailability has an explicit refusal counter and cleanup path.
- Successful factory/QI/AddRef references are audited and balanced; borrowed city getter pointers are never released. Native final Release is compared with Rust logical destruction, then the Rust allocation is destroyed. No native pointer is used after final Release.
- Every suite runs before Init record reads, all 65536 version values, empty/one-byte truncation, trailing bytes, pre-existing stream errors, output capacities 0/1/2/3 with clean/error streams, valid/invalid records across constructed/repeated Init/repeated Shutdown/re-init states, eight message IDs per state, CLSID/QI views/unknown-IID untouched outputs, canonical identity and reference returns.
- Each suite ends initialized and checks final Release=0 plus Rust implicit shutdown. Native internal destructor calls are covered by the published final-Release contract, not by an interception claim. Release from a raw zero-reference constructor and counter overflow are not driven: the supported public factory supplies an owned reference.
- Stream fixtures are independent on each side. The expected Rust adapter segment-interface negotiation is classified as setup, separate from owner operations; any other unexpected stream operation remains a mismatch.

### Measured constructed results

The final native run executes four independent suites: load, before full save, before fast save, and PreCityShutdown. Every suite owns a new native object distinct from the borrowed city component.

| Measurement | Per suite | Four-suite total |
|---|---:|---:|
| Checks | 65763 | 263052 |
| Mismatches / refusals | 0 / 0 | 0 / 0 |
| Record Reads | 65562 | 262248 |
| Record Writes | 56 | 224 |
| Messages | 56 | 224 |
| Owner reference return checks | 19 | 76 |
| Failed Reads, native / Rust | 65553 / 65553 | 262212 / 262212 |
| Failed Writes, native / Rust | 42 / 42 | 168 / 168 |
| Consumed input bytes, native / Rust | 131106 / 131106 | 524424 / 524424 |
| Output bytes, native / Rust | 56 / 56 | 224 / 224 |

Read/Write returns, positions, errors, primitive call counts, bytes, initialization state and reference returns match. Input failures are deliberate invalid versions/truncations/errors; output failures are deliberate capacity/error cases. No comparison differences remain, including turbo runs; no normal-speed discrepancy investigation is required.

### Natural evidence and excluded scope

Public city getter slot 60 supplied four read-only snapshots in the native run. All validated as the original industrial owner with initialized=1, at load, both presaves and PreCityShutdown. These are snapshots only. Earlier natural Read/Init, actual city/store call forwarding, destruction interception and complete natural lifetime coverage remain unobserved. Existing IN3 in `docs/specs/questions/commercial-industrial.md` stays open and excluded. No full-natural verification or replacement promotion is claimed.

The replacement-enabled refusal control separately logged the existing Rust city Read/Init/Write/Shutdown path. That integration control is not an original comparison.

### Gates and game evidence

- Initial `qbuild all`: cargo4938 passed/clippy0; CMake configuration failed, so no baseline ctest credit. Subsequent focused builds passed. First completed full gate was67/68 because the new native Init/Shutdown comparisons needed local `[abi-ret]` comments for the published AL=1 contract; those annotations were added without ABI-table edits.
- Final `qbuild all` on engine commit `ff173d78`: cargo4938 passed,0 failed; clippy0 diagnostics; CMake2/2; ctest68/68 PASS (fc4verify98s). Focused quick also PASS. No Rust files changed, so rustfmt check is n/a. Git diff whitespace check passed.
- Final DLL is the read-only artifact `build-runs/dll/industrial-r2-ff173d78/FreeCity4.dll`, SHA256 `A52DF4AC728379280876A1C2E1A46C20D6217BED0B9620F252605337AA8DB1CB`.
- Shared save script `build-runs/industrial-r2-save.script`, SHA256 `9439F4054BFAC9916012CCFADE129EAA1378E9F2E5E44104CE352FDAA896A325`, schedules a full save near2026-02 and fast save near2031-12. Every request uses a unique label, turbo/end_date2032-01-01, refaudit enabled and full quit/shutdown; at most one request was outstanding.

| Label | Result | Comparison / saves |
|---|---|---|
| `ind4-r2-native-92f61086-01` | clean2032-01-05/ref0 | Initial probe incorrectly counted adapter setup as unexpected:262472 reported differences; superseded, no exactness credit. Full/fast saves succeeded. |
| `ind4-r2-native-ff173d78-02` | clean2032-01-01/ref0 | 263052 exact checks,4 suites,0 refusals; full/fast saves succeeded; shutdown suite and final Release completed. |
| `ind4-r2-off-ff173d78-03` | clean2032-01-18/ref0 | Shadow OFF,0 suites/0 refusals; original industrial; full/fast saves succeeded. |
| `ind4-r2-refuse-ff173d78-04` | clean2032-01-03/ref0 | Replacement enabled:0 suites/1 refusal, no native oracle acquired; full/fast saves and replacement shutdown succeeded. |

Verdicts/logs are in `build-runs/game-results/<label>.*`; full gate logs are `build-runs/qbuild-*.log` and `build-runs/fc4verify-*.log`. Final log scans include the shutdown tail, beyond the autotest verdict, for last-reference/over-release warnings. No requests remain outstanding.

### Delivery and next steps

- `1d8106c9`: initial constructed comparison; `ff173d78`: corrected stream setup accounting, complete outcome counters and return-channel annotations. Intervening documentation commits record milestones.
- No new questions. Existing IN3 only concerns excluded natural interception.
- Coordinator may land this opt-in verification tooling and retain the narrowed evidence. No default replacement or full-natural coverage change is requested. Unsupported-platform/factory-allocation refusal branches exist but were not forced in the supported game.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-industrial4-r2-manifest.json`. Worktree stays detached for supervisor publication; no merge, push or shared-ref maintenance.

## Coordinator measured-shadow audit (2026-10-22)

industrial_constructed_source 263052/263052; industrial_constructed_joint97 131526/131526.
Independent original factory owner versus actual production fc4_industrial CAPI,
4source suites and2joint regress suites (load/shutdown); joint totals counted once,
not cumulative reports added twice. Zero mismatches/refusals/resync; source controls
clean2032/ref0 and official train97 gate68/regressPASS.
This qualifies measured exact same-process shadow under COVERAGE.md; IN3 natural
interception, unobserved factory failures and exhaustive natural lifetime remain
unproved. No additional replacement or default-enable promotion.
Raw joint evidence/hash: build-runs/run4-train97-native-audit.json.
