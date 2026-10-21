| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-terrain4 (run4 terrain unblocker: TR38/TR39/TR40 crash blockers, TR35) | `re-terrain4` / (docs lane) worktree `.claude/worktrees/re-terrain4` | **ready** (docs only): TR38, TR39, TR40, TR35 answered (all DIFFERS) in terrain.md §15.5.12, §15.5.11 items 15a/18/19, §13.2; TR33, TR34, TR36, TR37 still open | glue-terrain3: fresh message per post + run 0x0074FE40 + Load from Read, then gt3-repl-6; RE: TR33, TR37, TR34, TR36 |

## Report (re-terrain4, 2026-10-21, run4 round 1)

Sources: re_query.py export and decomp/ only (REA not needed). Numeric and calling-convention facts are from the
instruction bytes. Commits: a65deae2 (TR38, TR40), 7c33c5a3 (TR39), a9e0b498 (TR35), then this report.

Questions: TR33-TR40 copied from the glue-terrain3 question file (build-runs/evidence-run4/terrain.md) into
docs/specs/questions/terrain.md. TR35, TR38, TR39 and TR40 moved to answered/terrain.md; QUESTIONS.md regenerated.

- **TR38 (gt3-repl-4 fault at 0x00755C2B), DIFFERS.** The fault is not a missing precondition. The heights copy
  `FUN_007569F0` loops over the delivered rectangle and clamps nothing. The fault registers show `x = 0x4102`,
  `z = 0`, `side 2` and `n = 129`, so the view's statics were correct. The loop ran past `x1 = 128`, which means
  the message the view received had `data1 & 0xFFFF >= 0x4102`. `Init` already copies the whole city itself. The
  rule is in §15.5.12.1: one fresh `cRZMessage2Standard` per post, set up the same way as helper `0x00743380`
  (new 0x2C, AddRef, SetType/Data1..4, MessagePost(msg,0), Release). Never reuse or rewrite a message after
  posting it. Rectangles must lie inside the grid. The array layouts and formulas of `FUN_00755B60` are written
  out there.
- **TR40 (gt3-repl-5 fault at 0x007540A5), DIFFERS.** This is not edge-layer state and not slot 67.
  `DrawSideCliffs` reads texture id 0x11 from the per-detail-level table `0x00B4C670[level]`. That table has
  `0x00B4BC44` entries and is filled from `0x00B4BCD8[terrainType]`. Both statics are written only by
  `cSTETerrain::StaticInit` `0x0074FE40`, which `cSTETerrain::Init` calls. The rule (§15.5.12.2, item 18): run it,
  or reproduce those statics, before the first view Init. §15.5.12.3 lists the view's own edge-layer setup.
  §15.5.12.4 describes slot 67 `DoLighting`. Two §15.5.9 rows were corrected: the tables are per level, and
  BC44 is the texture table length.
- **TR39, DIFFERS.** Nothing outside the terrain calls slot 96. The terrain's own `Read` (`0x00745380`) reads the
  version and the view reference into `+0xE8`, then calls `Load(SC4GetIGZDBSegment(stream))`. So the records
  load during Read (§13.2). Note: `Read` writes the saved view reference into `+0xE8`, and §15.5.1 should be
  checked against that on the load path.
- **TR35, DIFFERS.** The terrain's slot-4 `Shutdown` calls view Shutdown with `ECX` = the view base, then Release,
  and clears `+0xE8`/`+0xEC`. The view destructor also calls Shutdown (item 15a).

Next: implementer (glue-terrain3) fixes posting, runs 0x0074FE40 and calls Load from Read, then sends
gt3-repl-6. RE: TR33 (0x00743B60 = CalculateNormalsAndAssignCliffTextures, likely also the +0x16 writer for
TR36), TR37 (call at 0x007411E1), TR34, and the Read-time view reference vs §15.5.1 creation.
