| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov12: native record observation and replay contract | `Codex` / `re-overlays-ov12` (RE, detached worktree) | ready; OV12 answered; 12 paper vectors; doccheck/questions checks pass; no ABI change or new game run | 1. Publish detached commits. 2. Extend shadow observer/replay using specified inputs; retain separate coverage counts |

## Report

Resolved **OV12** in [overlay-manager.md](../specs/overlay-manager.md), §§2.4–2.7/3.8, with twelve paper conformance vectors in §9.1. This is spec text only: no engine, plugin, C API, ABI tables, coverage map or tool changes.

- Arena headers: overlays at manager `+0x98`, handles `+0xAC`, groups `+0xC0`; each is begin, exclusive constructed end, allocation end, free head and free tail. Strides are `0xB4/0x30/0x24`. Constructed ranges include freed records; capacity storage is not enumerable payload. A live first word is −1, a freed tail's word is zero, and free traversal stops at the recorded tail, including index zero.
- Complete overlay/handle/group field tables include byte draw-state, ignored padding, collection stamp, both matrices, retained aspect, group targets and source/binding ownership. Free payloads remain retained but are not live overlays. Public IDs, group indices and native pointers have distinct meanings.
- Static index at `+0xF0/F4/F8` stores list-head pointers, not IDs. Eight-byte nodes contain next pointer then direct ID. Order/duplicates are preserved; current rectangles cannot reconstruct insertion/re-placement history. Observation must not mutate stamps, references, draw state or counters.
- Binding metadata includes exact factory-map/node/record and info-output layouts, dimensions, levels, frames and current resource identity. Source interfaces have owner-observed dimension/handle slots, not a universal implementation layout. Binding handles are opaque driver IDs; upload image data is not retained in overlay/binding records.
- **Snapshot-only full replay is impossible.** Model shadows discard geometry, projected pairs, descriptor and lighting inputs and have no placement handle. Other records lose key/flip/setter/allocation history; cell lists and runtime image contents also depend on history. §3.8 specifies original creation/mutation/resource/lighting inputs needed from before allocation and prohibits crediting fed-back computed native matrices/rectangles as independent verification. Existing OV1/OV7/OV8 boundaries remain explicit.

### Evidence and validation

Read `glue-overlays-shadow5:docs/lanes/glue-overlays-shadow5.md` first. It contains **no Owner closure table or `open:` rows**; its explicit full-shadow blockers were used instead. Its measured getter subset remains **73/73 exact**, not new evidence from this lane and not live-record coverage. The report's branch question file supplied the exact OV12 wording because this worktree's base predates that entry.

Queried the main checkout's export with `tools/re_query.py` before inspecting narrowly selected owner/factory decompilation. Verified field widths, free/allocate operations, matrix/reference extents, list nodes, binding-info output and driver upload against `asm --bytes`. Principal address intervals are cited in the spec. REA was not used: the brief says this session has no binary open; no REA failure or fallback is claimed.

- `python tools/doccheck.py check`: **PASS**.
- `python tools/questions.py check`: **PASS**, after regenerating `docs/specs/QUESTIONS.md` with `index`.
- `git diff --check`: **PASS**.
- Initial q.py launch of doccheck failed before Python: Git Bash `CreateFileMapping`, Win32 access denied. The subsequent direct PowerShell launch passed; the failed launch is not a failed documentation check. Assembly verification corrected the draft upload slot to **61** (byte offset `0xF4`) before the final passing checks.
- Cargo, Clippy, CTest, game runs: **not applicable**, documentation-only RE lane. No game request or verdict is outstanding. No new full-shadow or pixel-data result is claimed.

### Questions, commits and landing

OV12's original text and dated **DIFFERS** answer are in [the answered archive](../specs/questions/answered/overlay-manager.md). No new question was filed. The local open file had no OV12 to remove; it still contains OV1/OV7/OV8/OV10/OV11. The regenerated index counts seven archived answers and next free ID OV13. **If the coordinator lands shadow5's added open OV12 from its other base, remove that now-answered open entry during reconciliation**; do not reopen or count it twice.

Milestones: `afc2794d4` initial handoff; `8dbb458c7` contract/vectors/archive/index; `21316411a` assembly verification and passing checks. No proposed ABI rows are needed. Commits are on the reserved detached HEAD; the supervisor publishes them to `re-overlays-ov12`. Queue kind is `re`, as requested by this brief.

## Handoff

- Spec task complete; publish commits and consume §§2.4–2.7/3.8/9.1 in the engine shadow lane.
- Reconcile shadow5's open OV12 if its branch lands separately; this base lacks it.
- Next implementation step: quiescent bounded record/cell/metadata observer; full replay additionally needs original ordered call/resource/lighting/upload inputs, not snapshot-seeded outputs.
- Preserve measured scope: prior getter shadow 73/73 only; no new game/record/image verdict here.
