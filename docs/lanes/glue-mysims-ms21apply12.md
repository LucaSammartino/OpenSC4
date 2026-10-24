| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysims-ms21apply12 (live dispatch owner) | `Codex` / `glue-mysims-ms21apply12` (engine lane) | in progress | Apply MS21/MS22; gate and measure house city to 2032. |

## Handoff
Goal: apply MS21/MS22 live-trip dispatch rules and compare live headers in the default-off shadow.
Done: applied MS21 selection, extraction, failed-leg recovery, callback 5 and terminal-route installation; MS22 Init/default comments; eleven §12.30 conformance cases. Formatting passes for ten owned Rust files.
Next: collect focused test session 36212 to terminal (initial inputs changed while compilation ran; retain this evidence); resolve failures, rerun focused if necessary; then stable-source all and coordinator verify gate; house run to 2032.
Facts: no game request outstanding. Existing MS20 §7.2.3 explicitly precludes independent live-trip replay through public ABI; no duplicate question filed. Shadow adds two semantic constructor auxiliary fields and live range contracts, not replay.
Evidence: build-runs/focused-dispatch.log is the outstanding session output. q.py failed before cargo due Bash CreateFileMapping Win32 error5; native Python focused.py runs Cargo and owns its log. Latest run number observed 519; reserve/check a free number at run recording.
Manifest: main build-runs/run4-glue-mysims-ms21apply12-r1-manifest.json. Detached HEAD: commit normally, do not update shared refs. Regress profile edit is coordinator work (tools/ is forbidden to this lane; no merge main authorized).
