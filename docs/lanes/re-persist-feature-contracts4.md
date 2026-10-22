| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-feature-contracts4: PR5, FM3, FM4 native contracts | Codex / `re-persist-feature-contracts4` (RE spec lane; detached HEAD) | ready; source/spec/answers `7a5ea740`; PR5 DIFFERS, FM3/FM4 MATCHES; 16 source-derived vectors plus controls; no ABI or game runs | Coordinator: apply the three answered contracts to the published owners. |

## Report

Answered only the three requested contracts. Reservation: `c0753b35`; source/spec/answer milestone:
`7a5ea740`. Worktree remained at detached HEAD. No engine/core/test reads, implementation changes,
runtime tooling, maps, ABI edits/proposals, annotations, network, agents, merges or pushes.

- **PR5 — DIFFERS.** Persistence-manager §8.1 now specifies that stored and supplied predicates
  both receive the once-copied resource current GetKey; append uses that same snapshot. Stored key
  still determines lookup and iteration position. §10.1a supplies six independent vectors covering
  before/after mutation, each predicate separately, short-circuiting, append/duplicates/order,
  ignored append failure, null output, reference balance and mutation during a predicate.
- **FM3 — MATCHES the stated acquisition/read distinction; fills the streaming gap.** Feature-manager
  §4.1.1 distinguishes section indexing, header reread and entry gathering. Gathered pairs survive a
  stopped entry pass; callbacks follow gathering. A successful earlier 40-byte chunk can leave a
  valid partial-line entry. Successfully acquired packs keep their installed bit; normal cleanup
  closes if open and releases both INI and acquisition references; scanning and initialized state
  survive Open/Read/Close false results. §10.1 has seven phase-specific vectors and acquisition/override
  controls. The spec distinguishes helper success from merely finding a record, and generic scripted
  file failures from actual backend reachability. It makes no empirical failure claim.
- **FM4 — MATCHES.** Feature-manager §3 specifies exact EAX/store bits at AddRef overflow and the
  signed Release branch: no decrement, Shutdown, deleting destructor flag 1, free, then EAX=0.
  Static features/packs survive. §10.2 has three boundary/ownership vectors, with no post-free reads
  and no dependence on source-language signed-overflow UB.

## Primary evidence

All native inputs came read-only from `C:/Users/tlori/FreeCity4`, queried first with
`tools/re_query.py`; decisive operations were checked with `asm --bytes`. No REA was needed.

| Contract | Native evidence and bounded corroborating decompilation |
|---|---|
| PR5 | `0097C2D3`, especially `0097C306–0097C34B` and return/unlock `0097C35E–0097C36B`; `decomp/_unnamed/region_00970000.c:5304` (enumeration body only). No direct callers in export; type wrapper `0097BFCA` dispatches through cache slot 12 at `0097C005`, passing output then predicate. |
| FM3 owner/acquisition | `0046FBF0`, caller Init `0046FD50`; bit precedes reader at `0046FC95–0046FCC3`; ignored enum result/cleanup/continuation `0046FD03–0046FD44`. Helper `005F9670` separates acquisition success and false/null output. Corroboration `decomp/cSC4FeatureManager.c:165–227`. |
| FM3 foreign reader | `0091D91C` EnumEntries, `0091DB73` LoadSectionNames, `0091CA15` GetFileLine; constructor `0091CF46`, destructor `0091CDE3`, Close `0091CE29`; narrowly read corresponding ranges in `decomp/cRZIniFile.c` (GetFileLine 373–452, EnumEntries 1175–1304, LoadSectionNames 1306–1405). Callback mutation verified at `0046FB10`. |
| FM4 | Shared AddRef body `004BDF00–004BDF07`; Release `0046F7F0–0046F814`; destructor `0046F7D0–0046F7EC`; Shutdown `0046F770–0046F77D`. Export identifies feature Release at vtable `00A88274` slot 2; existing documented shared AddRef identity retained. |

## Validation and remaining work

Question index regenerated: PR5/FM3/FM4 moved with their original questions into
`docs/specs/questions/answered/persistence-manager.md` and `answered/feature-manager.md`.
Open counts changed 365→362 and answered counts 1184→1187. FM1/FM2 remain untouched.
Six PR5 + seven FM3 + three FM4 vectors are derived expectations, not executed tests.
No game run or cargo/clippy/ctest gate is required for this documentation-only static proof.
`python tools/questions.py index` and `python tools/questions.py check` passed; the index is
current. Bounded diff review and `git diff --check` passed. A trailing-blank-line warning from
the first diff check was corrected in the final report commit.

The Bash-backed q.py wrapper failed with `Bash/Service/CreateInstance/E_ACCESSDENIED`; bounded
PowerShell reads/queries were used. Git emitted a packed-refs lock permission diagnostic during
detached commits but created both commits successfully; no shared refs/config were altered.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-feature-contracts4-r1-manifest.json`.

Next: coordinator can apply these answers and vectors to the already published owners. No further
RE question, empirical probe or ABI proposal is needed for these three contracts.
