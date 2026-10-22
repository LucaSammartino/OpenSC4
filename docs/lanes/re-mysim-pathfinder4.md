| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-pathfinder4: MS16 factory defaults | `re-mysim-pathfinder4` / RE spec lane, detached worktree | ready; MS16 answered; questions index/check and whitespace PASS; cargo/clippy/ctest n/a; game n/a; ABI unchanged | - |

## Report

Answered only MS16 in [mysims.md §4.7.2](../specs/mysims.md#472-inherited-commute-pathfinder-configuration-ms16-compat),
with the §7.2 cross-reference and §12 case 26. Existing MS15 commute behavior is unchanged apart from
replacing its unresolved-default cross-references. No implementation, tool, ABI table or game-file changes.

### Proven contracts

- Factory `0x0070D860`–`0x0070D89F`: fresh 0x3160-byte object; constructor, AddRef, Init, borrowed traffic
  pointer, output publication, AL=true. Init's return is ignored; no request overrides or cached object.
- Init `0x006D78B0`–`0x006D7951`: first call builds zero-filled X+2/Z+2 distance vectors using global-city
  dimensions; repeated Init returns false without resetting configuration.
- Factory defaults: flags 0, destination strategy ID 3, destination byte 0, allowed/starting uint16 masks
  `0x0002`/`0x0002` (Car ID 1), null costs, float32 maximum trip +0.0, density 0, uint32 maximum work 0.
  CheckCommute changes only flags/density to 1/1 and explicitly sets strategy 3 again.
- Null costs select nine float32 traffic costs at `+0xC78` through `+0xC98`; flag 1 bypasses the cost-budget
  rejection; zero maximum work disables timed suspension. Strategy 3 differs from strategy 0's standard
  destination pre-search. No guessed tuning constants or saved native path results.
- The returned reference count is 1. Final Release shuts down and deletes the pathfinder. The native
  factory's allocation-null and null-out paths dereference null; they are not ordinary false returns.
  The supplied-service null-output behavior already specified for CheckCommute remains distinct.

### Evidence and limitations

Queried the static export first. The factory has no decoded function, and re_query's fallback failed with
missing capstone (pefile is also absent). REA returned `target_unavailable: No app is open`; no target was
opened and no extra REA tools were used. Recovered the complete factory bytes with installed MSVC
`dumpbin.exe /disasm:bytes /range:0x70d860,0x70d89f` against main `bin/SimCity4.exe`. Constructor, Init,
setters, ownership and relevant FindPath branches were verified with `re_query.py asm --bytes` and local
MSVC byte disassembly. Specifications contain prose, tables and address citations, no assembly/decompiler listings.

Executable SHA256: `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Ignored local evidence: `build-runs/ms16-factory-bytes.txt` and `build-runs/ms16-findpath-bytes.txt`.
No native runtime traces or in-game runs were produced. No remaining MS16 contract gap or further RE round.

### Validation and delivery

- `python <WT>/tools/questions.py index`: regenerated the index; mysims now has 0 open / 16 answered.
- `python <WT>/tools/questions.py check`: PASS; `find MS16` resolves to the answered archive.
- `git diff --check 40fda7bb^ HEAD`: PASS; worktree clean before final report.
- Cargo, clippy and ctest: n/a (documentation only). Game/shadow: n/a. ABI exports/rows: unchanged, no proposals.
- MS16 moved to `docs/specs/questions/answered/mysims.md`, marked MATCHES; no new questions.
- Commits: `40fda7bb` first spec/evidence milestone; `a2b601b3` limit semantics, cases and MS16 answer;
  this final report is committed separately. Reserved lane branch remains supervisor-owned; commits use detached HEAD.

Next steps: coordinator may merge the docs-only queue entry. No additional lane work is required.
