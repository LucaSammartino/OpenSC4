| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotcfg-editor-le10 (editor residual behavior) | `re-lotcfg-editor-le10` / RE lane, detached reserved worktree | ready: partial closure of LE3/LE4/LE5/LE7; 12 vectors LER01–12; questions check and doccheck passed; ABI unchanged; no game runs | Implement the measured contracts; residual RE questions LE9–LE18. |

## Report

Spec-only partial closure, committed in `a8c8748da` and `036b61177`.
The requested two-thirds research cutoff was observed: no new tracing after
the 94,328-unit guard reading; subsequent work was spec writing, question
splitting, verification and reporting. The guard read 114,985/150,000 after
the second milestone commit. No complete-owner or additional byte-coverage
credit is claimed; the inventory's historical classifications remain intact.

- §25: type-7 payload requests 12 bytes and has size/capacity three, yet writes
  a fourth word beyond capacity. Insertion copies only three words. The shared
  small allocator uses a 16-byte class including its four-byte header. Fixed
  adjacent-heap corruption/crash outcomes are not a portable behavior promise.
- Water/terrain commands only switch to modes 6/7, clearing pending logical
  payload on a mode change and requesting the cursor. Same-mode requests do
  nothing, including retaining any pending payload.
- Both movement overloads and unconditional boundary/dirty/title/draw/invalidations
  are specified. Position deltas stay in x87 registers rather than spilling
  to binary32; keyboard steps are 0.25/1/0.5/2/2/8/4/16 for masks 0–7.
- SetTitle uses the current configuration name, prefix `Lot Editor -- `,
  suffix ` *` for any nonzero dirty byte, then invalidates unconditionally.
- §26: chooser service IID `0x856C8CF7` is the directory-file segment interface.
  Successful query references are retained without another AddRef; enumerated
  provider references are Released. Empty-set retry, enumeration-array leak
  and duplicate-query acquisition leak are explicit native quirks.
- Deletion targets one exemplar key `(0x6534284A,0xA8FBD372,configuration ID)`
  on the identity's retained persistent segment. The directory-file backend
  erases its mapping before file removal and does not test removal success.
  Misleading exported ReadRecord labeling is called out with instruction evidence.
- Foundation/outside popups now have exact row and dispatch contracts.
  Foundation removal accepts only `0x5301814C`, unlike chooser `0x5301814A`.
- Twelve prose conformance vectors LER01–LER12 accompany these rules.

LE3/LE4/LE5/LE7 were moved, with their original scope and explicit partial-answer
markers, to `docs/specs/questions/answered/lot-configuration-editor.md`.
The narrower open questions LE9–LE18 cover hit conversion, rectangle conversion,
the two selectors, SetLotSize, object popup/wrappers, shared grid/name semantics,
static/view setup, input, raster/help/3D and shared attribution. Questions index
was regenerated. No previously answered question was reopened.

Existing LE6 settles the README owner rows for §4 initial fractional conversion
and default lengths; §8.1 defaults and reconciliation bridge (not the remaining
PostInit/size/3D work); §8.2 prompt/path edges; §8.4 splitter/non-ASCII behavior;
§9 record naming; §14 provider/path details; and §16.2 literal placeholder text.
Existing LE8 settles §7 and §21.1 exact Shutdown fields/order and retained cursors,
§20.5 full file-error text, and staged §20.4 core/window composition subject to
the remaining dependencies. These answers do not settle the whole §6 inventory
or view/alpha and ordinary rendering rows. The engine README was only read for
its requested owner-closure table; it was not edited.

Validation: questions index exit 0, questions check exit 0, doccheck check exit 0,
and `git diff --check` clean. Foreground output is in
`build-runs/questions-index.log`, `questions-check.log`, and `doccheck.log`.
The first q.py invocation failed in Git Bash CreateFileMapping (Win32 error 5)
before any gate ran; it is a transport failure, not a failed doccheck.
`build-runs/check-docs.py` then ran the actual gates synchronously and saved
separate logs. Git printed a packed-refs.lock permission diagnostic on commit,
but both commits succeeded on detached HEAD; no shared refs were changed.

No cargo/clippy/ctest or game runs apply to this prose-only change. No C ABI,
ABI proposal rows, engine/plugin/capi, coverage map, game files, decomp output
or external assets were changed. Only local export/decomp queries and read-only
literal byte inspection were used; no REA session or other MCP service was used.

## Handoff

- Ready partial closure; research stopped, no outstanding gates or game requests.
- `a8c8748da`: §25, LER01–06. `036b61177`: §26, LER07–12, question archive/splits/index.
- Questions LE9–LE18 are the exact remaining scope; no complete-owner credit.
- LE6/LE8 owner-row consequences are recorded above; do not reopen them.
- Next: coordinator may land these docs; implementers can consume §§25–26.
- Queue uses actual full detached HEAD; supervisor owns publishing the reserved branch.
