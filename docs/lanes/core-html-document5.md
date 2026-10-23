| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-html-document5: cHTMLDocument owner closure | Codex; core-html-document5 (detached HEAD reserved for lane branch) | blocked: corrected Rust tests/clippy pass; native configure stalled; HD7/HD8 exactness open | Coordinator native gate; settle HD7 and HD8. |

## Implementation and owner coverage

Extended `engine/core/src/html_document`; no duplicate parser, plugin wiring or C ABI changes.
The module README now has an Owner closure table mapping every section/subsection of the owner
spec to concrete code and named tests. The spec has no separate conformance-test section; its worked
examples and boundary cases form the conformance suite. Expanded the module from 14 to 27 tests.

Corrected the two SGML cursor resets while preserving push-back/style state; SELECT's missing-close
result; numeric base detection, whitespace/sign processing and saturation; signed FONT SIZE indexing
and 32-bit address wraparound; process-wide one-time colour construction; the complete slot-15 byte;
and DBCS character-boundary scans, including quoted attributes, CSS and HR width. Init retention and
QueryInterface's primary AddRef now have explicit foreign host hooks. Named-anchor registration
retains only kept anchors, as the spec recommends. Fonts, surfaces, windows and opaque 32-bit layout
words remain foreign services. No arbitrary native address is read for malformed FONT SIZE values.

Source milestones: `4acae585` compatibility audit/fixes; `82b4c094` conformance coverage;
`723e944d` final character-boundary coverage; `9991f01b` clippy correction.

## Measured validation

- Focused suite: **27 passed, 0 failed**. Earlier 21-test pass and expanded 25/27 fixture failure
  are separately recorded in `build-runs/focused-evidence.md`, including the literal-spec diagnoses.
- Final explicit-file formatting: **PASS**, 3 owned Rust files, 0 diff lines. Formatting edits used
  apply_patch on rustfmt check diffs, never rustfmt stdout source rewrites.
- Full gate attempt 1: Rust workspace **5944 passed, 0 failed, 0 ignored**, summed across all 27
  `test result:` lines. Clippy failed on `skip_while_next`; this was fixed using `find`.
  Native CMake configure stalled at MSVC C compiler ABI detection for over ten minutes. Collected
  cancellation as terminal exit 1 from session 33428. Full original logs retained in
  `build-runs/qbuild-test-attempt1.log`, `qbuild-clippy-attempt1.log`, and
  `fc4verify-configure-attempt1.log`.
- Full gate attempt 2, stable corrected source: Rust workspace **5944 passed, 0 failed, 0 ignored**
  across all 27 result lines; **clippy -D warnings passed** (Finished dev profile).
  Native configure reproduced the same stall; its log stayed at ABI detection from 15:47:08
  through 15:49:21 and cancellation. Collected terminal exit 1 from session 95892.
  Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-configure.log`.
  **CMake/CTest incomplete; no full-gate PASS is claimed.**
- q.py initially failed before running Cargo (Git Bash CreateFileMapping Win32 error 5), so focused
  Cargo commands ran directly in PowerShell. Native process-detail inspection and taskkill were
  denied by the environment; exec-session cancellation succeeded and was collected.
- Game: **n/a**, no request issued and no outstanding label. This core module has no game adapter;
  neither shadows nor game behavior were measured. ABI: **unchanged**.

## Questions and limits

- Existing **HD7**, §7.11: reads beyond the original object's extent are undefined. Provisional zero
  is implemented and tested; this case remains open.
- Filed **HD8**, §7.7: explicit BODY loop bullets stop only on EOF/handler failure, whereas the worked
  example stops on `/BODY`. Implemented and tested the literal bullets, with a code comment.
  Exact BODY termination and the resulting §5.5 `/HTML`/push-back behavior remain open.
- Both live in `docs/specs/questions/html-document.md`. No other owner sections are intentionally
  unimplemented or Unsupported. In-object malformed FONT SIZE reads use modeled scalars or the
  explicit `HtmlHost::document_layout_word` compatibility projection.
- The generated QUESTIONS index was not edited or regenerated: engine lanes may not run tools/.
  The coordinator should regenerate it with the normal question-index workflow.

## Next steps

The lane is queued **wip**, not ready. The coordinator must complete the native CMake/full CTest
gate in an environment where compiler ABI detection runs, and obtain spec decisions on HD7/HD8
before claiming exact whole-owner closure. The worktree remains at detached HEAD as instructed;
the supervisor publishes the commits to the reserved lane branch. Nothing was merged or pushed.
