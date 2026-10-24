| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov15 — ordinary decal flags | Codex / re-overlays-ov15 | ready: OV15 answered; doccheck passed | Land spec; implement ordinary creation-time bit 7 and retain full-flags comparison. |

## Report

Spec milestone **6497e84f3** adds [§3.2.2](../specs/overlay-manager.md#322-ordinary-decal-flags-and-setter-preservation-ov15): successful ordinary binding adds start at full flags `0x91`. Move/alpha/RGB/draw-state preserve that word; self-lighting 1 produces `0xD1`. Bit 7 is established unconditionally during creation, not inferred from the resource or later setters. Ordinary source and keyed adds share the rule. Snapshot eligibility does not override the existing dynamic-manager snapshot skip.

The proof uses the exported instructions: default flags 1 at `0x007360F1`, binding OR `0x90` at `0x0073C011`, source OR `0x90` at `0x0073B0CF`, placement's repeat-only mask at `0x0073AC15–0x0073AC24`, and self-lighting OR `0x40` at `0x0073656F`. Setter stores, allocation copies and the terrain invalidation helper were checked as well. Numeric flag operations and field widths were verified with `asm --bytes`; no pseudocode or assembly listings were included in the spec. No REA query was needed.

§9.3 adds **four instruction-derived conformance vectors**: each complete flags word through the controlled suffix, manager/resource/quality independence, ordinary creation families/null binding, and preservation across false self-lighting/repeat changes. These are paper expectations, not executed native tests. They explain the sibling lane's reported `0xD1` versus `0x51` mismatch without masking bit 7 or attributing it to missing history.

OV15 was copied verbatim from the unmerged glue-overlays-live6 question file into [answered/overlay-manager.md](../specs/questions/answered/overlay-manager.md), marked **ANSWERED 2026-10-24 (re-overlays-ov15): DIFFERS**. This base had no open OV15 entry to remove. `tools/questions.py index` succeeded. No new questions; existing OV13 capture limits and unrelated open questions remain outside this answer.

Validation: **`python tools/doccheck.py check` passed**; `git diff HEAD^ HEAD --check` passed. Cargo/clippy/CTest: n/a (documentation only). Game: n/a; no new game run requested or performed. The sibling lane report was read as existing evidence, not claimed as a new test or full rendering verdict. No ABI rows proposed; no C ABI changes.

The requested Owner closure table was absent from the sibling lane report and overlay spec. The explicit prohibition on reading `engine/` prevented inspecting a module README there; the OV15 entry and sibling report identified the exact blocking mismatch. No engine/plugin/capi/ABI files were read or changed. The q.py wrapper failed to start Bash with a CreateFileMapping permission error; subsequent commands used native PowerShell successfully. Git emitted a packed-refs lock warning during the milestone commit but created detached commit `6497e84f3`; no shared refs/config were changed.

## Handoff

- Completed spec/answer/vector milestone: `6497e84f3`; doccheck and whitespace validation passed.
- Next implementer action: set ordinary creation flags to include bit 7, then check full words `[0x91,0x91,0x91,0x91,0x91,0xD1]` through the live6 suffix.
- Keep full-flags comparison; this answer does not claim a new fixture run or complete replay/rendering verification.
- No outstanding operation, game label or question from this lane. Queue uses actual full detached HEAD; supervisor publishes the reserved branch.
