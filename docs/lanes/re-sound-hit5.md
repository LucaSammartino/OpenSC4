| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sound-hit5: low-level sound and music specification | Codex; detached re-sound-hit5 worktree | ready: partial behavior spec, doccheck passed | Land docs; schedule SS1–SS11 follow-up before full core implementation |

## Report

Added [sound-system.md](../specs/sound-system.md), covering all 13 requested
owners with interface inventories, construction/state observations, direct-call
and callback dispatch, nonvisual IO boundaries, failure observations and links
to the existing SC4 audio specification. Added 24 prose conformance vectors;
none are executed engine fixtures. No engine, plugin, C ABI, ABI table or
coverage-map edits. No proposed ABI rows.

This is ready to land as a **partial specification**, not a complete low-level
sound implementation contract. Eleven explicit research items remain. The HIT
bytecode decoder, freshness selection law, full resource formats, unnamed channel
slots and several lifecycle/failure paths cannot yet be implemented solely from
this document. The inventories distinguish observed slots from unresolved names.

Measured instruction facts include the signed register upper-bound check at
0x0093A7EB, unguarded 32-bit track load/unload count at 0x0093AF41/0x0093AF5D,
two-stage signed 32-bit cell volume arithmetic at 0x00945820, manager partial-init
flag behavior at 0x00968BA7 and missing system-sound lookup result at 0x009692FC.
Other observations come from exported owner routines; the spec identifies numeric
and format laws that still need instruction verification rather than guessing them.

Validation: `python tools/doccheck.py check` passed after the owner-section
milestone. Final documentation and question-index checks are recorded below.
Cargo/clippy/ctest: n/a, documentation only. Game runs: none; shadow comparison:
n/a. No REA calls: the brief explicitly states that session has no binary open;
all RE evidence used the local export/decomp and re_query.py. No REA error or
corrected-source game verdict is being claimed.

Questions: SS1–SS11 in [questions/sound-system.md](../specs/questions/sound-system.md).
Existing audio questions were not answered or moved. The new ledger is indexed
with the repository question tool. A final review qualified the channel's apparent
no-source success branch because UpdatePatch can dereference the missing source
before that branch; the vector now uses a valid loaded source.

Milestones: fec8df29 lane record; 3a9d7046 HIT boundary/state; 7cb745b8 remaining
owner sections and vectors. Git committed successfully on detached HEAD despite
a packed-refs.lock permission diagnostic; no ref/config maintenance was attempted.

## Handoff

- Next: coordinator can land the documentation; assign decoder/selection/format follow-ups SS1, SS5, SS6 first.
- Confirmed: CheckedRegId uses signed >74; track count can underflow; dependent sounds extend IsPlaying.
- Confirmed: cell volume has two signed divisions and 32-bit intermediate products; preserve truncation order.
- Conformance vectors are prose only, with no game or engine verification.
- Source checkout: C:/Users/tlori/FreeCity4; private query helpers remain ignored under this worktree's build-runs/.
- No ABI exports changed and no game-request labels are outstanding.
