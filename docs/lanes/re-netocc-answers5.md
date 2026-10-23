| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5 (network owner questions) | Codex / detached `re-netocc-answers5` | ready: verified spec subset; owner questions remain open | Supervisor publishes detached HEAD; land harvest and round-3 rules; continue remaining owner traces separately |

## Report

Final round 3, 2026-10-23. Spec text only; no engine/plugin/capi changes, ABI exports, ABI-table edits or
proposed ABI rows. No new questions filed. NO33, NO36 and UG5 remain OPEN: this report does not claim full
owner closure. Five network vectors (40–44) and five underground vectors (9–13) were added as prose
conformance fixtures, not executed implementation tests.

### Harvest and reconciliation

PR10 a8b30439 was harvested as `3393a1a2`, and a33fa02e as `36a3a300`, retaining amarret authorship.
`c2b2f5ce` reconciled the harvest with the existing NO34/35/37 answers. The second harvest briefly had
conflict text following a failed UTF-8 patch-generation attempt; that intermediate tree is superseded.
The reconciled tree and final specs contain no conflict markers.

Conflict decisions retained the local detailed pole/pylon record and demolition-quote rules; retained
the PR network/underground material; remapped the PR's NO32–37 to NO38–43; unioned symbol/name addresses
with pre-round rows winning duplicates. Inherited coverage-map additions were spec-only with implementation
columns '-'; round 3 did not edit tools/coverage-map.tsv. Old NO34/NO36–37 cross-references left by that
renumbering were corrected to NO40/NO42–43 in the specs.

Earlier complete answers remain: NO34 (`c2d33055`, signed-int64 demolition quote), NO37 (`ddbbc96d`,
pylon record/live offsets, refs, placement and packing), NO35 (`015ea80f`, pole record nesting,
reference IIDs, shared attachment vectors and delegate cleanup). Their answered entries remain archived.

### Round-3 specification results

- `4d54a552`: network §3.6/§3.6.1 and underground §4/tests 9–13 specify exact connection version-2
  acceptance, seven u32-count/24-byte wall lists, both occupant IIDs and attached-reference −4 adjustment.
  Creation now specifies the surface quad, identity matrices, own-cell-first lookup, ordered eight-neighbor
  searches, temporary-result releases, fresh-versus-loaded network requests and final own-cell reacquisition.
  Also specified: linked-ref replacement, model-state notification, callback latch preservation, three
  packing tuples, renderer +28's discarded result, position-only copies and one-instance enumeration.
  Corrected misleading function labels: 00648360 is pipe QueryOccupantInterface, 006540F0 copies position,
  00654620 sets the linked occupant, 00655520 reloads textures and 00659D70 enumerates models.
- `0812ef00`: network §6.2.1/tests 40–42 specify changed/equal enum setters, unconditional bitmap dirtying,
  source/owner AddRef-before-Release order, shared modulo-u32 blink accumulation and the unsigned threshold
  500 with discarded excess, position/reinsertion ordering and visibility forwarding.
- `dfd29578`: network §§4.1/4.6/4.7.1/tests 43–44 specify pole constructor defaults, exact binary32
  attachment-table constants and unsupported-subtype pointer retention; unrotated 10×10 floor geometry,
  UVs, white color, ordered wall edges, retained alpha and unclamped vertical UV scaling by 0.0625.
  Exact diagonal-floor operation order remains open rather than being inferred from decompilation.

### Validation and limits

At the resumed harvested tree and after each spec milestone: questions index/check, doccheck check and
git diff --check PASS. Final validation is repeated with the report commit. Doccheck includes the
clean-room wall consistency check. Manual clean-room review found behavior prose, tables, constants and
address citations, not original code/listings or decompiler-local structure. A corrected capped scan
found no conflict markers, goto/local/param/undefined patterns in the three affected harvested specs;
the existing FUN_00466590 reference is an address label, not pasted code. An earlier broad scan was
misquoted and is not counted as validation.

Numeric widths, branch comparisons, call arguments, reference adjustments and operation order published
this round were checked against main-checkout exported instruction bytes; PE data verified table floats,
UV arrays and scale constants. Main-checkout scoped decompilation was used for orientation only.
Round 2's REA target_unavailable result remains distinct: round 3 used export/decomp fallback and did not
attempt to open a shared session. No original program output was committed.

Cargo/clippy/ctest: n/a (spec-only). Game runs and shadow comparisons: n/a. The ten new fixtures describe
expected behavior; they are not measured engine results. Git emitted packed-refs.lock permission messages
during commits, but commits returned success and produced the recorded detached HEADs; shared refs/config
were not edited.

### Remaining questions and next traces

NO33 still needs full pole connection/cell-walk, model/resource, transform/bounds/Pick/Draw verification.
NO43 overlaps the geometry/drawing work, including precise diagonal-floor rotation arithmetic.
NO36 still needs texture composition, RebuildModel, Draw/Pick, full transform/bounds/sort and message rules.
UG5 still needs descriptor-dependent piece/wall/floor generation, complete Draw, bounds, transform/view
and message decisions. NO40/NO41 remain open for underground geometry and unknown saved-field semantics.
Other previously open questions were not closed by these subsets. See the two per-spec question files;
their progress notes distinguish published rules from remaining owner requirements.

Next lane should start with connection CreatePieceModel/CreateGeometry (00655850/006562B0), pylon texture
composition/RebuildModel (005F12D0/005F1D00/005F20A0), or pole Pick/Draw/cell walk
(0064A870/0064C800/0064A4D0). Validate numeric behavior against instructions, not inferred function labels.
Merge queue: build-runs/merge-queue/re-netocc-answers5.json, kind re, state ready, at the final detached HEAD.
