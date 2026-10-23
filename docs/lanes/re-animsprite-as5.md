| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-animsprite-as5: animated sprite owner contracts | Codex / `re-animsprite-as5` (RE, detached worktree) | Partial spec ready: AS4/AS5/AS11 answered; doccheck passed; no ABI changes or game runs. Full owner closure not achieved. | (1) Resolve AS1/AS2/AS3/AS6/AS7; (2) resolve AS10 and residual AS9; (3) engine lane completes owner qualification. |

## Report

This round closes **3 of the 8 requested questions**, AS4, AS5 and AS11, in
`docs/specs/animated-sprite.md`. Their original entries and dated DIFFERS answers
are archived in `docs/specs/questions/answered/animated-sprite.md`; the question
index is regenerated. AS1/AS2/AS3/AS6/AS7 remain open. The changes are ready to
review and land as partial documentation, not full owner-closure credit.

- AS4: rechecked the existing five-zoom acquisition, retained-reference and
  destructive-prefix rollback contract against the instructions. Partial loads
  return false; the old overwritten prefix is not recovered. Resource type and
  requested IID are distinct. No animation/controller reset is implied.
- AS5: added signed frame-count limits, byte wrapping, reverse ping-pong,
  single/empty-frame, stop, literal rate sentinel and non-progress examples.
  Tick has no convergence guard; a safe diagnostic is not a native return.
- AS11: published IID `0x9AD0E7E3`, zero options, resource slot 4 `(0, output)`,
  release/transfer and unchanged-output failure behavior. Added independent
  proposed UNEXECUTED factory conformance cases in §10.

Validation: `python tools/questions.py index` succeeded;
`python tools/doccheck.py check` passed. No cargo/clippy/CTest gate is applicable
to this documentation-only change. No game execution, numeric goldens, shadow
comparison, C ABI exports or proposed ABI rows. No engine, plugin, capi,
coverage-map, main-checkout or shared-ref edits were made. Git committed on the
reserved detached HEAD. It emitted a packed-refs.lock permission warning but
created commit `3e6ef2a2`; the resulting worktree was clean.

The explicit two-thirds research limit ended investigation: the first milestone
guard reported **101275 / 150000** measured context units. Remaining questions
were not relabeled answered. The report/queue are a partial-spec delivery.
No REA calls were necessary; local export and decomp were available. The q.py
wrapper failed to start Git Bash (CreateFileMapping access denied), so bounded
PowerShell commands were used instead. A scratch PE reader initially failed on
an unbacked BSS address; it was corrected to distinguish virtual zero-fill from
file bytes. Its results do not prove the global's runtime value.

## Handoff

- Goal remaining: AS1/AS2/AS3/AS6/AS7 (AS10 also blocks the owner).
- Done: `3e6ef2a2` loading/clock/factory contracts; final report commit adds §10 factory cases and full archived question text.
- Next AS1: reconcile model-instances §6.2 table reader with animated-sprite §4.1; table +0x1C is entry count, not sequence data. Constructor sets controller +0x34 null at 007FDD20; LoadFromResource does not install it. Installation/provenance is still unproved.
- Next AS2: enumerate shipped DBPF headers/exemplars with main/tools/dbpf.py. Located DATs at C:/Program Files (x86)/GOG Galaxy/Games/SimCity 4 Deluxe Edition (EP1, Intro, SimCity_1..5, Sound). No data census was run.
- AS2 caller leads: GetAnimatedModelFromResourceKey has 12 direct calls, including automaton/model-maker/vehicle/dispatch/train-junction paths; constructor callers are 007FED83 and 00808A49. These are not proof of particular shipped occupants.
- AS3: build-runs/evidence-re-animsprite-as5.py reads PE vtables and bounded export extents; selected local intervals give 61 entries/10420 bytes, including two global factories. This is NOT the requested 60/10482 reconciliation.
- AS3: 00B4E65C is loader zero-fill; constructor copies its current word to all cached corners. Runtime immutability not proved. Do not promote file zero to a native runtime golden.
- AS6/AS10: Draw yaw uses hardware FPATAN (007FEB27), stores binary32 (007FEB50); asin uses __CIasin (009F03B0), stores binary64 for classification but retains x87 input, then stores result binary32 at 007FEB5D. Its helper can change/restore CW (009FAB25/009FABBB). Needs complete primitive/whole-call qualification.
- AS6: __setdefaultprecision 009F6C30 calls __controlfp with value 0x10000/mask 0x30000 via __fpmath. This alone does not prove renderer-entry PC/RC; no universal native mode was published.
- AS7: existing finite quad/cache/draw/pick rules remain; malformed/nonfinite/trap and whole-call mode qualification remains open.
- No outstanding operation, game label or ABI proposal. Continue only from committed docs and ignored local scratch evidence.
