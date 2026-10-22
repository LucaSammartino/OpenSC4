| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-landfill-records4 (independent landfill zone/occupant verification) | `verify-landfill-records4` / detached worktree (engine lane) | blocked: initial published zone calling-boundary check; LG31 filed; no engine/ABI changes, builds or game runs; no verification credit | (1) Resolve LG31 from existing published contracts. (2) Supply a permitted native zone interface or separately scope occupant-only verification. |

## Report

Stopped at the task's required initial boundary check, within approximately 15 shell/edit calls. Milestone `b79b2286` records the question and initial handoff. No engine implementation was attempted after the boundary gap was established.

### Published contract findings

- Landfill section 1 identifies `cSC4LandfillZone` as a private plain refcounted object with no GZCOM CLSID. Its documented message-target-shaped vtable provides QI, AddRef, Release, a no-op DoMessage and a destructor slot.
- Section 7.1 documents native acquisition through developer slot 12, `GetLandfillZoneFromCell`: the result is borrowed, with no AddRef. Taking and releasing one's own reference is supported; this does not provide zone behavior calls.
- Section 7.4 explicitly identifies the zone vtable as nonpublic. Sections 4.4.9 and 6.2 describe zone lifecycle and inline serialization, but do not publish callable public zone query/mutation/Read/Write interfaces. Targeted searches found no zone entries in `docs/abi/interfaces.tsv`, `vtables.tsv`, or `layouts.md`.
- Section 9 describes a consumer reading a private region pointer. That is excluded by this task's prohibition on private native fields. Published implementation addresses are not a public calling boundary.
- Occupants have a separate published GZCOM factory/interface and serializable interface (sections 1, 6.4 and 7.3); existing plugin creation uses them and a documented exemplar-holder layout. That does not establish a public zone interface or justify zone-owner verification credit. No occupant-only implementation was substituted for the required initial stop.

### Question and limitations

Filed **LG31**, `docs/specs/questions/landfill.md`, asking for an existing permitted native zone acquisition/calling/lifetime contract or confirmation that public-interface zone verification is unsupported. `docs/specs/QUESTIONS.md` supplied next-free LG31. The generated index was not edited or regenerated: this lane may edit only its question file under specs and may not run tools scripts. No new RE work is requested.

No native calls, comparisons, constructed cases, natural-call interception, fallback/refusal instrumentation or lifecycle/save/reload observations were performed. There are no comparison measurements and no exactness claim. Existing developer 71/71 was not reused as evidence. No raw-record copy, second Rust oracle, host mutation, private-field access, default/config change, ABI change or game request was introduced.

### Validation and operational facts

Build/test/game: **not run**. The task requires stopping on a missing published boundary and prohibits a code-free baseline build. Only documentation changed; no DLL was built or submitted, and no game request is outstanding. The requested clean2032/ref0/save/reload/shutdown evidence therefore remains unavailable.

The first quiet-shell invocation failed with WSL Bash service access denied, so subsequent inspection used bounded native PowerShell commands. Git emitted a packed-refs lock permission error during the first commit but reported successful detached commit `b79b2286`; no shared ref/config repair or maintenance was attempted.

Resume only after LG31 is resolved with an allowed published contract, or the coordinator explicitly creates a narrower occupant-only task. Preserve detached HEAD. Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-verify-landfill-records4-r1-manifest.json`.
