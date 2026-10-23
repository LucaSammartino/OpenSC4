| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-font-renderer4: first cGZDrawFontRenderer contract | Codex, `re-font-renderer4` (detached reserved worktree) | WIP: REA preflight unavailable; no behavior contract published | Coordinator must restore the shared REA target before separately authorized research; resolve conflicting renderer identities first. |

## Report

FIRST1 stopped on 2026-10-23 before a substantive spec milestone. Both calls to the configured
`mcp__rea__procedure_pseudo_code` for `0x00897722` returned `target_unavailable`, category `unavailable`,
retryable false: **No app is open.** The service suggests `open_binary`, but that operation is outside this
lane's explicitly configured two-tool allowance. No service was added or opened. The task's instruction
to never downgrade/bypass failed service controls over the general export-fallback advice in the workspace
notes. The second preflight batch also issued redundant export constructor checks before its error was
assessed; no further research followed. There is no automatic retry request.

### Preliminary evidence and attribution conflict

The export was queried before REA, as required. These are preliminary original-instruction observations,
not a completed renderer contract:

- `0x00895F65` allocates **8 bytes**; on success it tail-jumps to `0x00897722`, on failure it returns zero
  (`0x00895F65`–`0x00895F7A`).
- `0x00897722` clears the dword at object +4 and writes **vtable `0x00AC55B4`**, not `0x00AC14BC`
  (`0x00897725`, `0x00897729`). It conditionally calls `0x0089618D` under byte `0x00B5C418` and returns
  the object (`0x0089772F`–`0x00897747`). The helper's behavior was not investigated.
- The auto-labelled `cGZDrawFontRenderer::cGZDrawFontRenderer` at `0x00829F70` instead writes
  `0x00AC14BC`, initializes storage through +`0x444`, and requests class `0x08CDA30B` with IID
  `0xA8BC2A75` into its member at +`0x43C` (`0x00829F77`, `0x00829FBC`, `0x00829FDA`–`0x00829FE6`).
  Its label cannot establish that this larger object is the 8-byte factory product. The actual owner and
  complete vtable boundaries remain unresolved.

These observations expose a conflict in the inherited identity tuple. Existing drawing-fonts.md claims
and exclusions were preserved verbatim. No byte credit was reassigned from the inherited 4,444-byte owner.

### Scope accounting and validation

| Accounting category | Result |
|---|---|
| Fully specified renderer bodies | 0 |
| Partial instruction observations | 3: factory `0x00895F65`, constructor candidate `0x00897722`, uncertain constructor label `0x00829F70` |
| Proven foreign bodies | 0; the large constructor is a conflicting attribution candidate, not a concluded reassignment |
| Remaining renderer-owned bytes/bodies | Unknown; inherited 4,444 bytes were not accepted as an audited inventory |
| Typed slots, dispatch, target/clip/pen/color/palette/source locking, glyph algorithms | Unresolved |
| Literal discriminating vectors | 0; no contract from which to derive them |
| Native/portable/replacement/runtime/whole-family credit | None |

No font-renderer spec, question entry, ABI proposal, or cross-reference was published: research stopped
before a coherent behavior section or bounded per-body checklist existed. No questions were answered or
moved, so the question index did not need regeneration. No engine implementation/tests were read or changed.
No tracked ABI tables, generated coverage, or tools were changed. No builds, tests, or game runs were performed.

The `q.py` wrapper also failed with Bash `E_ACCESSDENIED`; subsequent bounded shell reads used PowerShell.
This shell failure is separate from the REA blocker. Documentation `git diff --check` passed;
the merge queue is **wip**, with no game runs and no ABI changes.

### Next steps and retained context

Coordinator action: restore/warm the shared original Windows 1.1.641 target through an authorized service
owner. Any continuation requires separate authorization; FIRST1 did not complete its requested contract.
First research step after that authorization: resolve the factory/vtable conflict using original vtable data,
QueryInterface bodies and actual font callback callers before assigning any of the 4,444 bytes.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-font-renderer4-r1-manifest.json`.
Worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-font-renderer4` (reserved detached HEAD).
