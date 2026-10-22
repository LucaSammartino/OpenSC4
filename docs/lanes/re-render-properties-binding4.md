| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-render-properties-binding4 (RP6 native CRT adapter binding) | `re-render-properties-binding4` / detached worktree (RE docs lane) | ready; native binding contract and 7 independent discriminator recipes published; questions/index and whitespace PASS; cargo/clippy/ctest/game n/a; RP6 locale-profile evidence remains open; ABI unchanged | (1) Implement the documented native primitive binding; (2) obtain controlled native RP6 profile evidence; graphics GR6 logger grammar remains separately owned. |

## Report

Published the remaining adapter-facing binding contract in render-properties §1.4 and
seven independent native discriminator recipes in §7.3. Initial substantive milestone:
`c6243251`. This final report commit adds acquisition-proof citations, request-string
limits and the independent short-name oracle. Completed owner/registry contracts were
not rewritten; no implementation, runtime promotion or additional owner credit is claimed.

The active services are private executable CRT `isupper` at `0x009EF5A9` (RVA
`0x005EF5A9`) and `tolower` at `0x009F0543` (RVA `0x005F0543`), both
`int32_t __cdecl(int32_t)`. Inputs are signed-byte promotions; mapping retains the low
eight result bits. Classification is construction-only; mapping occurs during both
construction and Find. Each primitive consults the executable CRT's current thread/global
locale state, without a whole-operation snapshot. Stored names survive later locale
changes unchanged. The spec gives exact lookup/ownership rules and an explicitly labeled
adapter policy for unavailable bindings; zero/negative primitive results are not a new
error protocol. Native thread-state failure can terminate through the CRT fatal path.

## Primary evidence

- Direct PE inspection of main `bin/SimCity4.exe` matches the export's SHA-256
  `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
  PE32/i386, preferred base `0x00400000`; both targets are in executable `.text`.
  No CRT DLL occurs in the import directory. The two named exports are
  `GZDllGetGZCOMDirector` and `__export__GetMemoryReport`; neither primitive is exported.
  Therefore the spec permits a verified build-specific main-module/RVA binding, not
  GetProcAddress of fabricated exports or substitution of the plugin's CRT.
- CRT instruction-byte queries prove signatures, locale synchronization, signed arguments,
  construction order, OS conversion dependencies, low-byte results and mapping/classification
  failure behavior. Deciding addresses are cited in the spec; no code/assembly was pasted.
- The actual public SDK in main `external/gzcom-dll/gzcom-dll/include/` supplies the
  application/properties route and byte-pointer signatures, with no ctype-provider slot.
  Native application QueryInterface at `0x004492A7–0x004492C2` confirms IID `0x26CE01C0`
  and reference acquisition. Getter `0x0044B660–0x0044B666` returns a borrowed properties
  pointer. Existing §1.1 remains authoritative for P/U; no object layout was invented.
- Language-manager slot 13 selects CRT request literals, including `english-us`, `japanese`
  and `turkish`; these are not evidence of accepted profiles or installed game languages.
  Its `setlocale` failure branch can still return language-selection success. The system
  codepage helper can replace C with system-default locale without restoring C.
- Read-only evidence helper/log remain ignored in this worktree's `build-runs/` as
  `native-binding-pe.py` and `native-binding-pe-evidence.txt`. They are not deliverables.

## Questions and limits

RP6 remains open in `docs/specs/questions/render-properties.md`, now with a dated binding
evidence addendum. Its primitive-binding portion is specified; accepted per-language
LC_CTYPE/codepage, startup construction order and observed I/i/high-byte outcomes are
unmeasured. No question was falsely archived as fully answered, and RP1/RP2 were untouched.
The independent recipes distinguish original-CRT calls from adapter outputs, signed from
unsigned bytes, first-NUL from EOF, construction from lookup and live from cached locale.
Native short-name expectations use direct classification/mapping of published source
spellings; no guessed internal string layout or unavailable short-name getter is needed.

The spec explicitly carries arbitrary native name bytes across the properties boundary
without assuming UTF-8. This informs graphics-rules GR6 but does not answer its separately
owned logger grammar, formatting or I/O-failure questions. No graphics question was closed.

## Validation and delivery

- `python tools/questions.py index` and `python tools/questions.py check`: PASS.
- Whitespace validation with `git diff --check`: PASS.
- Cargo, clippy, ctest, game/shadow runs: n/a (docs-only); no native tests were executed.
- Only this report, the render-properties spec and its open-question file changed.
  Engine was never read or edited. No tools/maps/ABI tables were edited. No C ABI export
  changed and no ABI proposal TSV is needed: these are internal CRT addresses, not new
  GZCOM methods or exported FreeCity4 functions.
- REA was not needed; export queries, instruction bytes, direct PE reads and public SDK
  were sufficient. No network, external communications, subagents or game runs were used.
- Environment: q.py's Bash launch returned E_ACCESSDENIED; bounded PowerShell was used.
  Git reported a packed-refs lock permission diagnostic while successfully creating the
  milestone commit; actual detached HEAD was checked. No branch/ref/config manipulation,
  merge, push or self-restart was attempted.

Coordinator follow-up: publish the committed detached tip to the reserved lane branch;
review the docs-only queue entry. Implementers can bind the two documented native
primitives now. Any claim covering shipped-language profiles still needs the native RP6
observations; graphics GR6 remains a separate contract task.
