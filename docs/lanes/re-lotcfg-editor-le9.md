| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotcfg-editor-le9: editor integration contracts | Codex; detached reserved worktree re-lotcfg-editor-le9 | ready | Engine: implement §§7.1/23/24; remaining RE: LE3/LE4/LE5/LE7 and foreign RZS2 |

## Report

Answered LE6 and LE8 in `docs/specs/lot-configuration-editor.md`, moved both
entries to `docs/specs/questions/answered/lot-configuration-editor.md`, and
regenerated `docs/specs/QUESTIONS.md`. Added §7.1 Shutdown ordering, §23
window/core composition, §24 portable integration, and vectors LEI01–15.
Corrected §16.2's initial slope display description and supplied the complete
§20.5 error, including two trailing LF bytes. No code or ABI changes.

Measured native facts: compatibility logical lengths 16/4/9 (all false),
growth 1, water 0, required-roads 8; constructor slopes 90/0/90, editor new-lot
90/0/5. Path formatting is `%08x`, slope formatting `%d`. Editor bounds roles
oppose the shared decoder's names; no bridge swap is authorized. Seven cursor
references survive Shutdown. Font is +0x130; exact Release/clear order is in
§7.1. Existing inventory stays 109 bodies/52,646 bytes; five shared bodies/427
bytes remain foreign, with no complete-owner credit.

Evidence: read-only main export queries and targeted decomp slices. Numeric
claims checked against byte-bearing instruction queries at 004797A0,
0047B640, 00480D10, 009EEF04, 0047DDF0, 0047E650, 0047F200, 0090EAC5,
009F4C35 and 0047C510. Three literal byte strings independently read from
main `bin/SimCity4.exe` by ignored `build-runs/read-text-le9.py`; export
corroborates the question/error strings. No REA call, no REA failure, no game
run or shadow equality claim. No ABI-row proposal.

### Owner closure audit

Read only the requested Owner closure table in
`engine/core/src/lotconfig_editor/README.md`; did not edit engine files or
inspect implementation. Each row below had an explicit open residual (plus
the chooser lexical dependency row). Q means a remaining question blocks it;
E means the specified boundary needs engine implementation/integration only.
All LE question IDs below live in `docs/specs/questions/lot-configuration-editor.md`.
RZS2 lives in `docs/specs/questions/rz-strings.md`. No uncovered editor
question found and no duplicate question filed.

| Owner / table row | Classification after this round | Exact remaining boundary / next engine action |
|---|---|---|
| Editor §1 state/selection | Q: LE5 | Title remainder. |
| Editor §2 edit popup | Q: LE4/LE5 | Dependent commands. |
| Editor §3 object popup | Q: LE4/LE5 | Types 3/5, detail markers, edge table, names/help. |
| Editor §4 compatibility | E (finite initialization/defaults); Q: LE3 for invalid numeric primitive edges | Apply §24.1 lengths, defaults and truncation; invalid numeric behavior is already within LE3, not a new LE6. |
| Editor §6 inventory | Q: LE3/LE4/LE5 | Remaining partial bodies; no complete-owner credit or changed byte inventory. |
| Editor §7 interfaces/lifetime | E for Shutdown; Q: LE5 | Apply §7.1 exact fields/order; central PostInit view/alpha setup still unknown. |
| Editor §8.1 new/read/open | E for defaults and typed Fix bridge; Q: LE4/LE5 | Implement manager slot 23 on shared mutable records per §24.3; remaining SetLotSize/PostInit/3D continuation remains blocked. |
| Editor §8.2 save/Save As | E for editor prompt/path contracts; Q: RZS2 for foreign non-C case tables | Apply §24 literal verification/title/options and literal splitter; do not substitute host Unicode folding. |
| Editor §8.4 identities | E for splitting/composition; Q: RZS2 | Original-case name and dotted extension to provider, exact lowered byte equality; non-C CRT mapping remains foreign. |
| Editor §9 record convention | E | Keep raw fields unchanged, use editor-local bounds roles; no decoder rename or normalized bridge. |
| Editor §9.1 edits | Q: LE3 | Remaining numeric cases. |
| Editor §9.2 mover/verify/cleanup | Q: LE3 | Verification unordered predicate; exact prompt now specified. |
| Editor §9.3 add/duplicate/delete | Q: LE3/LE5 | Physical type 7, bounds helpers, deletion names. |
| Editor §9.4 containment/insertion | Q: LE3 | Type-7 allocation; shared constructor remains foreign. |
| Editor §9.5 fences/foundation | Q: LE3 | Invalid conversion. |
| Editor §10 vectors | E for §24 bridge/default vectors; Q: LE4/LE5 | Full Open continuation remains blocked after core Read integration. |
| Editor §11 key/edit mouse | Q: LE3/LE5 | Hit assignment, drag conversion, remaining cycling. |
| Editor §12 selectors | Q: LE4/LE5 | Name/catalogue and sort interpretation. |
| Editor §13 drawing subset | Q: LE5 | Raster. |
| Editor §14 vectors | E for split/provider composition; Q: RZS2 | Implement published provider call shape; foreign non-C lower tables unresolved. |
| Editor §16.1 name/size/path | Q: LE4 | SetLotSize body. Initial path formatting now specified. |
| Editor §16.2 slope/fence | E for literals/formatting; Q: LE5 | Native placeholder must remain a message; preparation/help bodies unresolved. |
| Editor §17.1 hit priority | Q: LE3/LE5 | Integer rectangle/fallback conversion. |
| Editor §18 stopping boundary | Q: LE3/LE4/LE5 | Existing explicit unsupported regions; shared attribution remains foreign. |
| Editor §19 slots | Q: LE5 | Unresolved extension 159, inherited slots foreign. |
| Editor §21 draw/mouse boundary | Q: LE3/LE5 | Raster/edit interiors. |
| Editor §21.1 static/destruction | E for exact Shutdown integration; Q: LE5 | Static parsing remains blocked. |
| Chooser §20.1 lifecycle | Q: LE7 | Service discovery/acquisition; core Close staging itself specified in §23. |
| Chooser §20.2 messages/sort | Q: LE7 | Grid collation/search interpretation. |
| Chooser §20.3 rows/selection/key | E for owner orchestration; foreign parser dependency | Malformed hex belongs to RZAtoUint32HexA, not new chooser policy; this row has no owner-open claim. |
| Chooser §20.4 edit/registration | E for staged acquisition/registration; Q: LE4/LE5 | Invoke actual core Open; preserve its unsupported continuation rather than supply a successful foreign verdict. |
| Chooser §20.5 deletion | E for exact error; Q: LE7 | Backing-store identity/database item scope and search interpretation. |

Whole Editor/Chooser ownership remains incomplete. The next engine lane can
close the E portions above without additional editor RE; it must retain Q
barriers. Existing shared-string RZS2 is explicitly foreign, not silently
answered by this lane's C-locale witness.

### Validation

`python tools/doccheck.py check` PASS (exit 0), foreground output in
`build-runs/doccheck-le9.log`; `git diff --check` PASS. Cargo/clippy/ctest n/a
(spec-only). Questions index regenerated after moving both entries.

## Handoff
- Done: LE8 271102541; LE6 5474cb9c8; owner table fully classified above.
- No further RE planned this round (two-thirds budget boundary approaching).
- Gate: doccheck PASS, diff whitespace PASS. Report final; ready for coordinator.
- Next: enqueue kind re/state ready with actual full detached HEAD; next engine lane uses the E/Q table above.
- Remaining editor questions: LE3/LE4/LE5/LE7; foreign non-C string case tables: RZS2.
- No engine edits, game requests, ABI proposals or whole-owner claims.
