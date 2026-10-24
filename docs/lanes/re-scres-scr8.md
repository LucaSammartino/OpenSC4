| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr8: resource capture contracts SCR8/SCR9 | Codex / `re-scres-scr8` (reserved branch, detached HEAD) | ready: SCR8/SCR9 specified; doccheck PASS | Engine lane can implement route-specific capture; natural coverage still requires nonempty game observations. |

## Report

Answered SCR8/SCR9 in `docs/specs/sc-resources.md` section 10; moved the
original questions and their run evidence into the answered archive and
regenerated `docs/specs/QUESTIONS.md`. Added twelve normative vectors,
SCR-V69–80. Work milestones: `26d54b9bf` (initial handoff), `b8d6bf033`
(contracts, dispatch, questions and first passing doccheck); final report
commit also corrects the type-info record description and a section reference.

Native ABI findings include stack order/width, AL/EAX/void result channels,
interface adjustments, descriptor/value/index token access, text const-reference
writes, image resource/factory paths and ASCII fields/graph/stream arguments.
Text adapter f64 consumes one address (RET 4); ASCII f64 consumes name plus
by-value double (RET 12). ASCII string slots 22/23 are S/char-pointer in that
order. ASCII type info is a twelve-byte data record, not an interface.
ParseCommand's ECX−0x98 is the base command state, not an arbitrary derived
object start; derived thunks must be preserved.

Natural-route findings: startup graphics rules use cRZTextFile and parser input
slot 5; token adapter tables see only token protocol operations. Macro/eval
dispatch and image decoding include direct calls. Token factory slots 10/11
select the token protocol, including the already-consumed-cookie read path.
Moving token table installation to OnStart does not expose file-parser traffic.
There is no lifecycle forcing a token resource or ASCII dump in every city.

There is no universal non-mutating raw-byte getter on token record/ASCII writer.
Specified capture observes existing raw IO or the original output sink;
read/seek snapshots remain a guarded fallback requiring successful exact
cursor restoration. GetBuffer is cursor-neutral but retains its result;
the buffer pointer is not encoded bytes or a pixel pointer.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py
check` PASS; `git diff --check` PASS. Rust/clippy/ctest n/a (spec-only changes).
No game runs requested or performed, no shadow verdict claimed. Prior run
labels remain historical evidence: `scres5-r1-2032-a` / `scres5-r1-2032-b`
empty; `scres5-r2-constructed-birch` exact 56/56 only for constructed adapters.
Natural subsystem coverage remains unestablished.

Evidence used main-checkout re_query export first, then targeted decompilation
and instruction-byte checks. No REA query was attempted (session has no binary).
No engine files read or edited; no C ABI exports or docs/abi rows changed or
proposed. No external/game/decompiled content committed. No new questions.

## Handoff
- Goal complete: SCR8/SCR9 answered in section 10, with SCR-V69–80.
- Contracts do not turn empty/constructed adapter evidence into natural coverage.
- Next: glue-scres-shadow5 can implement startup parser capture, direct image and token-factory boundaries using these contracts, then request real traffic runs.
- Follow the original stream/record and exact operation context; never serialize live objects again for capture.
- Queue kind `re` follows this brief's explicit requirement; branch is supervisor-reserved, HEAD remains detached.
