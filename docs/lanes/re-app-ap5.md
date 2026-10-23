| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-app-ap5 (AP12–AP15 application owner closure) | Codex / `re-app-ap5` | ready; AP12–AP15 answered; doccheck passed | Implement published contracts and conformance vectors; update core-app5 closure table |

## Report

Completed the requested cSC4App behavior specification in `docs/specs/application.md`.
All four questions AP12–AP15 are now in `docs/specs/questions/answered/application.md`,
marked ANSWERED 2026-10-23 (re-app-ap5): DIFFERS. Regenerated `docs/specs/QUESTIONS.md`.
Other application questions AP3–AP11 remain outside this closure task.

### Published contracts

- AP12: case-insensitive Admin Enabled=false, exact zero comparison, by-name dispatch
  to path loading, adjacent INI/existence ordering and bypass flag.
- AP13: Resources.ini section/key/class syntax, hexadecimal-prefix conversion,
  first-semicolon split, permissive malformed IDs and entry-local failures, group-stack
  behavior including invalid unmatched END_GROUP, and command-line packed switches.
  Database source order includes eight passes, including the final slot-59 directory
  segment. Recursive enumeration excludes .dat case-insensitively, probes other files,
  preserves OS enumeration order, emits root files before depth-first child traversal,
  and does not deduplicate passes. Published all fifteen resource-factory mappings.
  Published font source/removal rules for five language IDs, two Arta defaults/five
  control-font bindings, FontStyle.ini selection and callback gates, nineteen colours,
  fifteen picture sources, cursor options/group mappings/exclusions, and eight captions.
  GetAppIniFilePath uses literal slot-51 candidate then slot-58 fallback concatenation.
- AP14: slots 29/31 and six folded getters, including feature/cheat-manager identities;
  save filename sanitation/encoding/256-byte cap, collision suffixes and existing-city
  ID reuse; literal SavedCity plus suffix; twelve video writes and fourteen reads with
  exact field widths/order and unknown-value preservation; preference application order,
  language fallback defect, graphics decisions, simulator-speed conversion and cheats.
- AP15: directory slots 51–58, including distinct slot 56; destructive obliteration
  failure behavior, view defaults, cleanup and invalid no-city access.

Section 16 supplies spec-level conformance vectors for implementers. These include
success, ordering, malformed input, preserved state, unsupported native invalid inputs,
failure cleanup and the observed native defects. No executable implementation or tests
were supplied by this RE lane.

### Corrections established during final verification

Font-service +0x44 removes font registrations/caches rather than adding aliases.
The lowercased cursor option cannot match mixed-case fullColor. Temporary font export
failure clears the path, so cleanup receives an empty path instead of the original name.
Preference application uses the cheat manager and a COPY of the framework command line:
it processes named x values, and never removes switches from the original. Its independent,
case-sensitive lowercase-x erasure can repeat a value or fail to make progress. This
supersedes the earlier draft's claim of one-shot original-command-line erasure.
Simulator speed converts through signed 64-bit _ftol2, then interprets the low 32 bits
as signed before clamping; values beyond signed 32-bit range do not saturate.
The previous application summary's direct configuration-rule evaluation claim was removed;
implementers should follow the instruction-backed preference contract in section 13.3.

### Validation and limits

- Final spec/archive/index: `python tools/doccheck.py check` passed.
- `git diff --check` passed after correcting an extra EOF blank line reported by the
  first archive check. The first doccheck also passed; no failed doccheck was hidden.
- Cargo, clippy and CTest: n/a, spec-only. Game requests/runs/shadow: n/a.
- C ABI exports unchanged; no proposed ABI rows and no ABI-version change.
- REA batch from the earlier continuation failed 7/7 with target_unavailable (no app
  open). Evidence comes from local re_query export, bounded decompilation reads, and
  instruction bytes/constants in the shipped executable. No tools or servers were added.
- Read the expressly requested application README Owner closure table and core-app5
  report. No implementation source was read or edited. No plugin/capi/coverage-map edits.
- Detached HEAD commits succeeded despite a packed-refs.lock permission diagnostic;
  shared refs/config were not changed. The supervisor publishes this HEAD to the lane.

### Milestones

Earlier rounds: `16700d84` getter/plugin/obliteration contracts; `3e0ee22d` video,
resources/naming/vectors and AP12/AP15 archive; `ecf78b6b` preference/filename draft;
`ea656019` continuation evidence. This round: `daf2c2aa` complete resource/UI tables
and preference corrections; `c5d2ed13` AP13/AP14 archive, conformance cases and passing checks.

## Handoff

- Goal complete: AP12–AP15 answered in normative application sections and archive.
- Next: coordinator lands the ready RE queue entry; application owner implements the
  vectors and updates its Unsupported/closure table, especially copied x-cheat behavior.
- Foreign font parser/resource factory internals remain separate owners; AP3–AP11 remain open.
- No outstanding game requests, builds, ABI proposals or local operations.
- Worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-app-ap5` (reserved detached HEAD).
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-app-ap5-r3-manifest.json`.
