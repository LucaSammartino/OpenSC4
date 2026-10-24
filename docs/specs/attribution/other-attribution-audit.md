# Remaining final-catch-all ownership audit

Measured 2026-10-24 by lane `re-other-attribution9`, against GOG 1.1.641's Windows export and the lane's initial coverage map.

## Scope and interpretation

[other-owners.tsv](other-owners.tsv) contains every function whose current subsystem is exactly `other`: 155 functions, 12,549 body bytes. Rows descend by size, with address order breaking ties. These are the final `*` map row's remaining functions, not the separate `cSC4*`/`cSC*` or `fn:*` catch-alls. The brief's approximately 1.21 MB and 600 KB target no longer describes this baseline: all three catch-alls together contain only 48,740 bytes (448 functions). Separately, 831 functions / 85,977 bytes have no inferred owner and are reported as `unattributed`; these are outside this table.

This is an attribution ledger, not a new behavior contract or a claim of implementation/verification coverage. Most entries already have class or namespace names; their missing piece is a specific subsystem row. Count each exact body once. Vtable membership establishes an owner using that body, not exclusive semantic ownership of folded code. Template arguments are omitted from owner keys to match the coverage tool's naming convention.

## Evidence and confidence

Sources are the main checkout's `ghidra/export/windows`, `re/names_windows.tsv`, RTTI/vtable tables consumed by `re_query.py`, and the corresponding class files in `decomp/`. The initial ranked inventory was generated using `tools/coverage.py --owners`. Export queries preceded decompilation reads. No REA session was needed, and no numeric behavior is inferred from pseudocode.

- **high:** high-confidence named symbol, or independently identified class-vtable slot. Small forwarding bodies are only upgraded where their class table corroborates them.
- **medium:** medium-confidence name transfer, an unnamed dependency reached exclusively by an owner under the current coverage algorithm, a neighboring body, or a named folded thunk without evidence of exclusive class identity. Such dependencies are proposed accounting ownership, not invented method names. The table deliberately retains this weaker grade.
- `Named` in an evidence phrase refers to the symbol ledger, not a claimed particular name-transfer algorithm. `A`, `B0`, etc. are not inferred from method names.

Important cross-checks:

| Body / group | Corroboration and implication |
|---|---|
| `00475330`, `cDumpSubTreeVisitor::sVisit` | High-confidence name; string/variant formatting calls at `004754C5` and `00475562`. Assign tree diagnostics without assuming which tree implementation it visits. |
| `0063DDF0`, anonymous `cParser::NextToken` | Bridge specialization parsing calls it repeatedly, e.g. `0063EABA` and `0063EB33`. This is the bridge parser's token helper, not an arbitrary HTML/graphics parser sharing a short name. |
| `006046D0`, `nsTexUtil::SetupTexture` | Called by dispatch view `0046CAC8`, pylon texture builder `005F15D8`, and renderer background-picture path `007C2686`. A rendering utility, not a dispatch-owned implementation. |
| `005CF610`, `LinearRandomIterator::Reset` | Construction placement calls at `005CF82B`/`005CF921`; accident path also calls it at `0071869A`. Proposed occupants/props accounting, retaining medium confidence rather than exclusive construction ownership. |
| `00734B90`, terrain dialog | `cSC4View3DWin::ChangeTerrainType` calls at `007E5F84`; UI vtables at `00AB3868`, `00AB3884`, `00AB38A4` and `00A8ECE0` corroborate the class family. |
| Effect-command bodies `0059DF50`, `0059E260`, `0059E4D0`, `0059EB50` | Slot 6 of RTTI tables `00A9F998`, `00A9F9B4`, `00A9F9D0`, `00A9F944`, respectively; the tables also contain the corresponding named `RegisterCommands` and `Parse` entries. Preserve the export's full local-class owner spelling. |
| `0090AD26`, `cGXCOMDirector` | Slot 13 at `00ACE460`; instructions return the director ID. `CreateGimexFactory` is named at `0090AD2C`, supporting the drawing subsystem. |
| `007B6500`, bitmap-view message forwarding | Slot 3 at `00AB9980`; instruction is a jump to base `cWinBMPView::DoMessage` (`007BFDE0`). The low-confidence transferred name alone was insufficient; the vtable supplies the stronger evidence. |
| `0093A42E` / `0093A1C0`, source debug information | Called by HIT debug-info methods at `0093A4B8` / `0093A1FB`. This belongs with audio, not Lua debugging. |
| `008F88C7`, cookie data | Cookie-manager callers `008F8977` and `008F8C1A` establish web/network context. |
| `004F0D40`, `cGroup_SC4RH::Clear` | Called by audio `cGroupLayer::Clear` at `004F0DFB`; the contained hash-table dependency's incidental lot-type label does not establish simulation ownership. |
| `005FCA70`, Lua table reference | Automata-script import callers include `0051EEF7`, `005216B8`, `00521C8B`. The utility remains a Lua-host helper, not a particular automaton class. |
| Matrix-row bodies | Constructor `009AB299` calls `009AACB8` at `009AB2AB`, which reaches recursive helper `009A9CBB` at `009AACF9`. These remain medium-confidence row dependencies. [The existing matrix ledger](../rz-sparse-matrix.md#9-exact-body-ledger-and-remaining-attribution) explicitly excludes these named dependencies from algorithm credit. |

## Proposed map application

[proposed-map-rows.tsv](proposed-map-rows.tsv) has four columns: pattern, subsystem, area, spec. The coordinator should review these exact owner patterns and insert accepted rows before the catch-alls, supplying `-` for core/shadow/replace. No coverage-map or ABI file was edited. All spec cells remain `-`: references to helpers or a single parsing operation do not establish an owner-level behavior specification. This audit itself must not be used as the spec cell.

One owner correction needs separate handling: `004179A0` is named free function `gAdviceMessageBox`. The coverage tool currently absorbs it into neighboring `cADVString` with rule `free-neighbour`. The ledger proposes `fn:gAdviceMessageBox`, preserving the actual symbol's ownership rather than treating it as a string method. Adding its map row alone will not defeat that propagation rule. Both proposed owners map to advisors, so subsystem totals are unaffected by that distinction.

The largest owners lacking an owner-level behavior specification are `cDumpSubTreeVisitor` (768 bytes), `cWinBMPViewGVExtra` (706), `cSTEChangeTerrainUI` (701), bridge-local `cParser` (686), `nsGZWinUtils` (682), `RZ` service accessors (639), and `nsTexUtil` (625). Existing mentions of the bitmap helper, service accessors, or texture release do not specify those classes/namespaces as subjects. `cADVString` parsing is partially described in advisors AD12, and MatrixRow dependencies have an attribution-only ledger; neither is new whole-owner specification credit here.
