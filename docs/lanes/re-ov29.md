| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ov29: OV16 native texture-source ABI; kind docs | Codex / `.claude/worktrees/re-ov29` / reserved branch `re-ov29`, detached HEAD | ready; merge `60f2fe7f1`, spec `317f10578`; ABI: no authored export changes or proposed rows; docs whitespace passed; cargo/clippy/ctest n/a; game/shadow n/a; OV16 answered | 1. Land queued docs tip, including the requested replace-overlays28 merge. 2. Resume native source adapter implementation from §2.7.1. |

OV16 is answered in `docs/specs/overlay-manager.md` §2.7.1 and archived in `docs/specs/questions/answered/overlay-manager.md`; question index regenerated.
Slot 3: ECX=source, one output pointer, 4 argument/cleanup bytes; complete output 8 bytes, alignment 4, unsigned width/height. Native string source leaves EAX=output address; adapter consumes memory, no scalar result.
Slot 4: ECX=source, zero argument/cleanup bytes; EAX=opaque driver handle. SetTexture's pending stage zero survives the source call.
Two instruction-derived worked calls specify numeric outputs, buffer canaries, stack balance, cached-source behavior and aspect 0.25; these are paper vectors, not executed engine tests.
Evidence: original PE vtable `00AA2A60` slots 3/4 resolve to `005D8130`/`005D8020`; instruction bytes checked for both bodies, overlay callers, manager return paths and SetTexture cleanup. Export/decomp suffice; REA was not needed.
Validation: base-to-tip `git diff --check` passed; `questions.py find OV16` reports only the answered archive.
Global `questions.py check` fails on the existing NT36 open/answered duplicate at `questions/network-tools.md:120`; the main checkout's check gives the identical failure. No unrelated question was edited.
The required merge imports replace-overlays28 `17c5a755fbe49f3bd0f5f5b2d27bcd4d665eace9`, including its engine/test files. They were neither read nor edited by this RE lane; inherited build/ABI/runtime results were not audited or rerun.
Authored changes are spec/question/index text and this lane record. Queue kind is docs as requested; the coordinator must account for the imported engine work when landing.
Remaining: implementer integration and runtime verification of native source adapters; no remaining OV16 ABI or buffer-size gap.
