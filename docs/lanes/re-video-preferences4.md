| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-video-preferences4: native video preference panel contracts | Codex; detached reserved worktree re-video-preferences4 | ready: bounded partial docs publication | VP1–VP5 track remaining redraw/events/construction and primitive qualification. |

## Report

Published [video-preferences.md](../specs/video-preferences.md), the first substantive
native contract for this panel. READY means bounded publication, not complete
replacement-panel coverage. Canonical Windows 1.1.641 export and read-only executable
SHA-256 match: `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Export functions/vtable and paged instruction bytes were primary evidence; local
class decompilation supplied navigation only. Three raw PE tables were read with
ignored standard-library helper `build-runs/video-data.py`. No REA query was needed.

### Qualified results

- Complete normal SetPreferenceOptions: 29 operation rows, byte/dword destinations,
  word comparisons, overwrite versus first-checked policy, sticky marker, normalized
  utility AL and reference/callback order.
- Complete message admission, EndDialog and Release; shared QI/Shutdown attributed
  by actual owner vtable. AL/EAX/output-pointer channels distinguished.
- Complete normal caller-owned DoModalWin chronology, pause/service/root lifetimes
  and ignored foreign modal result, with required-service domain.
- Partial redraw: first enum group, null-P path, final depth group/cleanup.
  Partial HandleControlMessage: four action IDs and one three-ID selection group.
  Partial ShowDialog: prefix, first three bindings and tail; false AL after the
  successful initialization tail versus true AL on already-open short-circuit.
- Save-before-apply, ignored foreign results, marker-gated confirmation, alternate
  unconditional save/close path and no rollback in close branch. IDs remain raw;
  no unproved conventional button labels.
- 24 independent literal PROPOSED/UNEXECUTED vectors: asymmetric full readback,
  saved-word independence, sticky marker, overwrite, repeated selection, admission,
  close/save/default-null paths, lifetime, selector guards and output channels.
  Full Auto/defaults and mode-list edge-case outputs explicitly unsupported.
- Eight native body sizes independently total 14,965 bytes. Neither this match
  nor shared-slot attribution supplies whole-owner coverage credit.

### Verification and boundaries

Questions index regenerated; questions.py check and doccheck.py check passed.
Own publication link check passed (5 local file links); inventory arithmetic passed.
Final whitespace/diff and clean-tree checks accompany publication. These are
documentation/analytic checks, not native behavior execution. Cargo/clippy/ctest
n/a; game runs `[]`; shadow n/a; ABI unchanged; no proposed ABI rows.

No engine, tools, ABI/SDK/maps, game config/device or native execution changes.
Foreign App/AP3–9, persistence, rendering, mode enumeration, widget and modal-loop
algorithms remain closed. CD6 catalogue rebuild timing stays unresolved.

### Questions and next work

Filed [VP1–VP5](../specs/questions/video-preferences.md): remaining redraw CFG;
other events/resource semantics; constructor/middle binding sequence; App callsite
types/default fixtures without reopening its body; modal service types/failure
domain. No prior question answered or archived. Next RE work starts with VP1
`0079020d–00791201` and VP3 first-redraw flag producer, then remaining event groups
before complete default/cancel output vectors. Unknown outputs are not successful
fallbacks. No portable implementation or mock whole-window oracle was used.

### Milestones and actual errors

`0ea8f272`: first substantive mapping/traces. `c199fbd2`: full readback/message/
lifetime. `7cab9c33`: actions/modal boundary and initial 22 vectors. Final report
commit adds two selector vectors and publication evidence.

q.py failed with `Bash/Service/CreateInstance/E_ACCESSDENIED`; used capped
PowerShell thereafter. Initial helper import failed `ModuleNotFoundError: pefile`;
replaced by standard-library PE reads. Two prior question files were absent.
Patch context/validation misses made no edits and were corrected. Git emitted
`packed-refs.lock: Permission denied` while successfully committing detached HEAD;
no shared ref/config workaround. No REA error/retry or native behavior run.

Own authority: main `build-runs/run4-re-video-preferences4-r1-manifest.json`;
last prepublication guard measured 127,767 / 150,000, within budget. Own ignored
queue `build-runs/merge-queue/re-video-preferences4.json`: docs, abi=false,
game_runs=[], actual committed HEAD. Supervisor publishes reserved branch.
