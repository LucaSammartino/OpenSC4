# budget: answered questions (archive)
<!-- Archive for [budget.md](../../budget.md). Lanes do not read this file. Open questions are in ../budget.md. -->

## From the budget shadow differential (filed by the coordinator, 2026-10-13; merged as `6ca4b65`)

About `docs/specs/budget.md`. Run 39 matched on every value for five months, so these are about the edges of what the
shadow proves, not about a known mismatch. The lane's third question (does the original budget's QI(0x452294AA)
pointer equal its month agent?) was answered by run 39 itself: the agent `139BE6F0` was matched through its
`cIGZMessageTarget2` interface, 4 bytes into the budget object `139BE6EC`, not by the name fallback.

<!-- from: From the budget shadow differential (2026-10-13) -->
~~**BQ1: can the original's own lot list differ from `GetLotsInCellRect(…, false)` over the whole city?**~~
**ANSWERED 2026-10-13** from the exe and written into budget.md §5.2. **No, on every known path.**
- `ComputeTax` walks the lot manager's master list through a safe iterator. Its only filters are: no building, not
  livable, and an untaxed cell.
- The cell map and the list change together in `CreateLot`/`DeleteLot`, and a load rebuilds one from the other.
  Highlight-mode lots are in neither set.
- Right after a load the two **orders** are identical too (first cell, x outer).

The only divergences are a direct `CreateLot` over an existing lot, or a null-list insertion point. Neither is
known to occur.

<!-- from: From the budget shadow differential (2026-10-13) -->
~~**BQ2: at the January 1 settlement, what do GetYTDIncome/GetYTDExpenses return?**~~ **ANSWERED 2026-10-13** from
the exe and written into budget.md §5.1 and §15 test 4. **Both return 0.** The settlement adds the month first, then
records history, and only then resets the year (`0x00692A4E`), so December's amounts are added and wiped in the same
call. The January 1 amounts are readable from history series `0x6A51178B`/`0xCA5BA3FB` (written before the reset),
and their net from the funds change.

## From the budget shadow (coordinator, run 86)

<!-- from: From the budget shadow (coordinator, run 86) -->
- ~~**BQ3 — budget.md, the monthly settlement total:** in run 86 (`regress rbc7f`), 30 of 31 months match. In
  2028-08 every one of the twelve tax lines is equal (1762, 767, 123, 98, 101, 30, 83, 34, 0, 1547, 290, 0), but the
  total income is 4839 in the original and 4840 in ours (net 792/793). The lines sum to 4835, so the total is not
  their integer sum: it comes from unrounded values. Which values does the original add for the total (float
  per-lot sums, or per-line floats before display rounding), in which order and width, and how is the total
  rounded? First seen in this run; runs 80–83 had none in 93–95 months. [ours as budget.md reads today]~~
  **ANSWERED 2026-10-14** from the exe (`ComputeTax` `0x00691E20`: sums at `0x0069205C`–`0x00692070`, truncations at
  `0x006920B4`–`0x006920EB`) and written into budget.md §5.2 ("The total is not the sum of the lines", worked
  example). **The bracket matches what budget.md already said, now stated exactly.** The total is a separate running
  sum over the **per-lot** tax products, not over the lines or the group sums. Each lot's tax is added to `total`
  first, then to its group's sum. Both are doubles in memory, but each `FADD` rounds to 24 bits, so they equal
  `float32` running sums. The order is the lot manager's master list. The total is truncated toward zero
  (`_ftol`, 64-bit), and so is each line. The lines drop their fractions and the total keeps them, hence 4835
  against 4839. A one-simoleon difference with all twelve lines equal is the lot-order effect: the worked example's
  four lots give 6567 in list order and 6566 reversed, with the same lines. Matching it exactly needs both the
  master-list order (creation order for lots made since the load) and a total formed lot by lot, not from the
  group sums. Which of the two ours lacks cannot be seen from the exe side.

## From applying the residential, budget, demand, land value and flammability answers (filed by the coordinator from the `apply7a` lane's report, 2026-10-14)

RG23 (+RG17), BQ3, DQ10–DQ12, LV1–LV4 and FL1–FL6 are applied: the proximity seed writes only `z = 0`; the budget
total is a float32 running sum in master-list lot order; the land value shadow orders developers by pointer and
snapshots maps while slot 10 is true; the flammability shadow polls the plumbing watered grid. The current answer is
in brackets.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**BQ4:** BQ3: releasing the safe lot iterator. The lane unlinks its nodes by hand, the same as `std::list::remove`
  at `0x00809580`, and frees each with allocator B, instead of calling `0x00809580`. [an equivalent unlink; the block
  is leaked, not freed, if no node is found]~~
  **ANSWERED 2026-10-14** from the exe (`ComputeTax` `0x0069207C`–`0x006920AF`; `std::list::remove` `0x00809580`) and
  written into budget.md §5.2. The unlink matches: the original removes every node whose value is the iterator and frees
  each with B. It then frees the block with C unconditionally (`0x006920AF`), whether a node matched or not, and also
  when the owner is null. The bracket's leak differs; *[choice]* free the block with C in every case, as the original
  does (recommended).

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**BQ5:** BQ3: does ComputeTax hold references on the lots it visits? [borrowed, no AddRef]~~
  **ANSWERED 2026-10-14** from the exe (loop `0x00691EB0`–`0x00692077`; `cSC4Lot::GetBuilding` `0x006BFA30`) and written
  into budget.md §5.2. Borrowed: the lot is the list node's value and the building is returned without a reference; the
  loop has no `AddRef` or `Release`. The bracket matches.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**BQ6:** BQ3: the walk requires the master lot list object at lot manager + `0x5C`. [otherwise refused, with the
  `GetLotsInCellRect` fallback]~~
  **ANSWERED 2026-10-14**: no original behaviour for the refusal; the implementer's choice stands. Facts from the exe,
  written into budget.md §5.2: the original's slot 15 (`0x00734850`) always registers on its member at `+0x5C`, and
  `ComputeTax` reads no lot manager field itself. With a null owner in the block it visits nothing and frees the block.
  A walk through slot 15 needs only the block and node layout, not the manager's.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**BQ7:** BQ3: line and total values outside the int32 range. [the low 32 bits of `_ftol`]~~
  **ANSWERED 2026-10-14** from the exe (`_ftol` `0x009EEF04`; return `0x006920FF`–`0x00692101`; group stores
  `0x006920DC`–`0x006920DE`) and written into budget.md §5.2. The original never narrows: the total and every group's
  monthly tax are 64-bit `_ftol` results, so values outside the int32 range keep all their bits. Only at or beyond
  ±2^63, or for a NaN, does it give `0x8000000000000000`. The bracket (low 32 bits) differs above 2^31; *[choice]* carry
  64 bits, as the original does (recommended). Normal play never reaches 2^31 simoleons a month.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**BQ8:** The land value and flammability host tables grew (new last callbacks) without an ABI bump, so a table of
  the old size is now refused by the `struct_size` check. [no bump; refused]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The host tables and their
  `struct_size` check are FreeCity4's plugin ABI; the original has nothing to compare with.
