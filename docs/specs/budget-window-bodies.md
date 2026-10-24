# Budget-window native bodies (BW17)

Companion to [budget-window.md](budget-window.md), sections 1, 4, 5.2, 5.6 and 8. The panel's balance-sheet
drawing, formatting helpers, taxes and loan windows are §4.2–§4.3 and §8–§10 (re-budgetwin, 2026-10-22).
Evidence: Windows 1.1.641 export, function inventory, vtables and instruction bytes; decompilation is used only
as a navigation aid. REA batch decompilation of `0x0077C660` on 2026-10-22 returned `target_unavailable`
(no app open); that function also fails in the saved decompilation. Its rules below come from instructions.
No original listings or pseudocode are reproduced here. State offsets are relative to the owning object.

## 1. Native owner inventory

The six entries of sub-budget vtable `0x00AB5494` are, in slot order:

| Slot | Entry | Meaning |
|---|---|---|
| 0 | `0x004F1F30` | QueryInterface, folded shared body |
| 1 | `0x005F47F0` | AddRef, folded shared body |
| 2 | `0x00769400` | Release; export's unrelated template name is not a semantic contract |
| 3 | `0x0062C980` | inherited DoWinProcMessage bridge |
| 4 | `0x0078BCA0` | DoWinMsg |
| 5 | `0x0077E420` | deleting destructor; non-deleting cleanup is `0x0077C4C0` |

The panel has a primary interface at object +0, a window-procedure interface at +8, and a message-target
interface at +12. The latter two QI thunks subtract 8/12 from `this` (`0x00763790`, `0x007637C0`).
Do not treat the secondary tables as extra independently owned panel objects.

| Table | Slots, in order (entry addresses) |
|---|---|
| primary `0x00AB4A40` | 0 QI `00774560`; 1 AddRef `0057EB00`; 2 Release `0057EB70`; 3 deleting destructor `00764CD0`; 4 decrement-reference-without-delete `0090D981`; 5 read dword at +4 `009D7E63`; 6 ShowWindow `00768670`; 7 HideWindow `00764AD0`; 8 Update `00764830`; 9 IsVisible `00764820` |
| window procedure `0x00AB4A2C` | 0 `00763790`; 1 `007637A0`; 2 `007637B0`; 3 DoWinProcMessage `00768750`; 4 inherited DoWinMsg `0041D4C0` |
| message target `0x00AB4A1C` | 0 `007637C0`; 1 `007637D0`; 2 `007637E0`; 3 DoMessage `00768620` |

Slots 4/5 in the primary table are tiny shared bodies, not balance-sheet operations: `0x0090D981` decrements
the unsigned +4 dword only when nonzero and returns it; `0x009D7E63` just returns that dword. The export labels
(`cRZUnknown::RemoveRef`, `cRZString::ToChar`) do not establish the public interface names.

### 1.1 Direct owner methods and exact missing-body boundary

This lists every function exported under either owner name, excluding STL container specializations, and
the already identified unlabelled owner helpers. An address is a native body, not an acceptable host input.
"Existing" refers to the main lifecycle/routing spec; it does not assert implementation completion.

| Sub-budget methods | Entries | Contract coverage |
|---|---|---|
| constructor; CreateSubBudgetWindow; DestroySubBudgetWindow | `007862B0`; `00779E50`; `0077A1A0` | Existing lifecycle |
| on_init; on_shutdown; DoWinMsg; on_button_activated | `007876B0`; `0077E440`; `0078BCA0`; `0078B120` | Existing routing; delegated bodies must use rules below |
| set/get_window_indiv_item | `007795A0`; `00779600` | Existing property contract; setter reference rules still need audit |
| add_text; add_comboBox; set_comboBox | `00779660`; `007798C0`; `0077DAB0` | Widget creation/options; set_comboBox is owner logic, not a row provider |
| add_Bitmap; add_image_button1/2 | `0077A250`; `0077B7B0`; `0077B960` | Presentation/factory helpers; failure/ref rules not yet fully specified |
| GetBudgetSegment | `0077A390` | Resource-backed bitmap segment creation; not a budget data provider; full failure/reference contract BW25 |
| SetupWindow; SetupWindowBig; SetupWindowIndiv; SetEmptyWindow | `0077A480`; `0077A6F0`; `0077A960`; `0077BEC0` | Setup/failure body gaps |
| init/updateOrdinanceBudget | `0077C660`; `0077ADA0` | Section 2 population, scale and refresh; widget failure contracts remain BW25 |
| init/updateNeighborBudget | `0077E600`; `00781C90` | Section 3; remaining rendering/refresh audit recorded below |
| init/updateIndividualBudget | `00786690`; `00785740` | Section 5 enumeration, restore acquisition and viewport; service-query arithmetic remains open |
| UpdateValues | `007898A0` | Section 6 raw providers and A traversal/reuse slice; B and slider rebuilding remain open |
| RestoreStartSettings; SetOccupantFundingLevel | `0077C5E0`; `0077BC90` | Existing sections 5.1/5.5 |
| DoIndividualBudgetWindow; end_dialog | `00786530`; `00779850` | Existing nested-dialog and modal/modeless contracts |
| ApplyDealEdits (unlabelled); non-deleting destructor | `0077AB40`; `0077C4C0` | Existing apply/ownership contracts |

| Panel methods | Entries | Contract coverage |
|---|---|---|
| constructor; deleting destructor | `007658A0`; `00764CD0` | Construction §8, §11.1; deleting destructor §11.5 |
| Init; RegisterMessageNotification | `00766E10`; `007648D0` | §8, §11.2 |
| IsVisible; Update; ShowWindow; HideWindow | `00764820`; `00764830`; `00768670`; `00764AD0` | §11.5; ShowWindow and HideWindow §8 and budget-window.md §7; ShowWindow invokes owner refresh |
| DoMessage; DoWinProcMessage; DoSubBudgetWindow | `00768620`; `00768750`; `00768460` | Existing command/modal/dirty routing |
| InitBalanceSheet; CalculateValues; UpdateBalanceSheetInfo | `00764DF0`; `00767270`; `00767990` | Section 4 computation and the complete control mapping (§4.2); InitBalanceSheet labels §11.1 |
| SetFormattedAmountInTextControl; PrintPercentageString | `00765800`; `00764D50` | §4.3 |
| DisplayLoanWindow; UpdateLoanDisplay; RemoveLoanWindow | `00765C30`; `00766680`; `00764CF0` | §10, §11.3, §11.4 |
| SetTaxes; DisplayTaxesWindow | `00766D00`; `00768130` | §9 |

Three unlabelled panel helpers are named by re-budgetwin (2026-10-22): `RestoreTaxRates` (`00764850`, §9),
`CloseTaxesWindow` (`007680D0`, §9) and `IssueNewLoans` (`00764BE0`, §10).

The executable's contiguous address region also contains unlabelled helpers. These must not disappear from
an owner-completion audit merely because the name importer omitted them:

- `00779B80`, `00779CA0`: widget/factory helpers (call WCM/GS/WM); names and full contracts pending.
- `00779FE0`, `0077A030`, `0077A080`, `0077AC20`, `0077AC60`, `0077ACA0`, `0077ACE0`,
  `0077C380`, `0077C3C0`, `0077C480`, `0077D7E0`, `0077D8C0`, `0077D9B0`, `0077E550`, `0077E5A0`:
  container/element support candidates; classify ownership and callers before claiming coverage.
- `00786F62` is an exported fragment amid individual setup, with an invalid decoded first call target;
  do not count it as a separate reliable method or ignore its bytes. Reconcile its boundary with on_init
  and the preceding initializer in the individual-body round.

Thus this is an exact exported method/slot inventory with explicitly unclassified helpers, not a claim
that every address in the region is an independent C++ member or that either owner is complete.
Section 5 now classifies `0077ACA0`, `0077D8C0` and `0077C480` from that support list as restore-record
copy, grow/append and release/free helpers respectively; the other listed support candidates remain open.

## 2. Ordinance population (`initOrdinanceBudget`, `0x0077C660`)

### 2.1 Raw providers, allocation and ordering

Use the ordinance simulator (native global `0x00B43D90`): slot 11 (+0x2C) GetOrdinanceAvailableCount,
slot 12 (+0x30) GetOrdinanceAvailableArray(buffer, &count), slot 20 (+0x50) GetOrdinanceByID(id), and
slot 8 (+0x20) IsOrdinanceOn(id). Count and IDs are unsigned 32-bit. Allocate count*4 bytes for IDs at
+0xA4, count*2 bytes for {original, edited} state bytes at +0xA8; +0xA0 is the in/out count supplied to
the array call. Allocation failure is not checked before use (`0x0077C6BA`–`0x0077C6F7`).

For each returned ID, in provider order: obtain the ordinance and call slot 5 (+0x14, current monthly income,
signed 64-bit EDX:EAX); obtain it again and call slot 22 (+0x58, IsIncomeOrdinance, low-byte boolean).
Increment countA (+0x124) for true, countB (+0x128) for false. There is no amount-zero or on/off filter here.
Then call IsOrdinanceOn twice: store the first result byte as original and the second as edited. They are
two observations, not a copied single result (`0x0077C700`–`0x0077C77D`). The initializer increments the
existing counts; its caller/initial state must provide zeroed counts. GetOrdinanceByID returns a borrowed
map pointer without AddRef (`0x0080E040`–`0x0080E06D`); these loops neither retain nor release it.

Only available IDs are enumerated; do not additionally query all IDs, IsEnabled, IsAvailableButOff, or filter
by income sign. If both counts are zero, use SetEmptyWindow with LTEXT group `0x6A3FF01C`, instances
`0x8A579F31` and `0x0A5CB1A1`, then leave the population path (`0x0077C77F`–`0x0077C824`). Otherwise
SetupWindow receives min(countA,9), min(countB,9); the full counts remain stored for scrolling.

### 2.2 Ordered lists and row map

Build A first by scanning the complete returned ID array and selecting IsIncomeOrdinance true. Then scan
the complete array again for B, selecting false. Each scan re-reads the signed 64-bit monthly income before
the category test (`0x0077CA10`–`0x0077CA52`, `0x0077CEA0`–`0x0077CEE2`). For each matching item among
the first nine of its category, create its checkbox and info controls and append the **array index**, a
32-bit integer, to the row-map vector +0x74..+0x78. It is not the ordinance ID. Initial map order is therefore
first nine A indices, then first nine B indices. No map entry is created for an off-screen item.
Map append sites: `0x0077CA93`–`0x0077CAC3`, `0x0077CF21`–`0x0077CF51`.

The checkbox IDs start at 300 and info IDs at 500 and follow the appended map entries, continuously across
A then B. The button handlers use id-300/id-500 to look up the source index in the map (main spec section 5.6).
The edited state controls the
checkbox and amount inclusion; the original state is only the apply-change comparator. Enabled items beyond
the ninth still contribute to category totals: the off-screen A branch is `0x0077CCFB`–`0x0077CD36`; B is
`0x0077D209`–`0x0077D24E`. A adds the helper's two-word amount; B subtracts it. The helper is the signed
64-bit multiplication entry `0x009F0D10` (its imported template name is misleading): multiply monthly income
by the signed-64-bit scale at +0xB0/+0xB4, retaining the low 64 product bits. A adds that product; B subtracts
it, including the off-screen branches. Enabled visible A rows display the product through SC4MakeMoneyString;
disabled rows display an empty string (`0x0077CC2E`–`0x0077CCB0`). Enabled visible B rows display the
**negated low64 product**, using two-word negation before SC4MakeMoneyString, and subtract the unnegated
product from the total (`0x0077D11B`–`0x0077D164`). This is not absolute-value formatting: a positive
monthly income classified as expense displays a negative amount. Disabled B rows have empty amount text.
The scale is owner state, not a refreshed total supplied externally; see §2.3.

Discriminators: available IDs [10 expense/on, 20 income/off, 30 income/on] produce counts A=2, B=1 and map
[1,2,0], including the off ordinance and all zero amounts. With ten income ordinances, only indices 0..8
have initial controls/map entries, but index 9 contributes to the total if edited/on. A provider that changes
IsOrdinanceOn between the two initialization calls can produce original != edited before user interaction.

### 2.3 Period scale and ordinance refresh (BW17)

Initialization chooses signed-64 scale 1 for period +0x28 == 0, and 12 for **every nonzero** period,
with high word 0 (`0x007878C8`–`0x007878D1`, `0x007879A9`, `0x00787A8A`–`0x00787A94`).
The constructor's default is 1; CreateSubBudgetWindow stores the period but does not calculate this scale.
These rules apply to the common init preceding mode selection. Do not restrict the ordinance scale to
period exactly 1 merely because normal buttons supply only 0 or 1. Multiplication and signed two-word
addition/subtraction retain low64 bits, including overflow; there is no saturation or floating conversion.

Refresh `0x0077ADA0` obtains the available-ID array again through ordinance simulator slot 12 with the
**existing** +0xA4 buffer and in/out +0xA0 count (`0x0077ADED`–`0x0077AE0B`). It neither re-allocates the
buffer/state pairs nor calls IsOrdinanceOn to replace the original/edited bytes. It clears the row-map
logical end to its begin (+0x78 := +0x74, retaining capacity, `0x0077AE0E`–`0x0077AE3A`). The stored
A/B counts gate their respective passes; refresh does not recompute those counts. Stable available-ID
order/count and classification are therefore preconditions for interpreting the saved state pairs as
the same ordinances. **Quirk:** refreshed IDs are associated with edits by array position, not by ID;
a provider that reorders the array reassigns those edits positionally. No new reconciliation is implied.

For each enabled category pass, scan all returned IDs in order. Read signed-64 monthly income (slot 5)
and classification (slot 22), through separate borrowed GetOrdinanceByID lookups, as in initialization.
For a matching category, let q be its zero-based ordinal, independent of the source-array index. Render
only q >= its offset (+0x11C for A, +0x120 for B), at most nine rows. This viewport quota advances only
for rows at/after the offset; it is not the unusual group-refresh item quota described in §6.
Tests are unsigned (`0x0077AE70`–`0x0077AEC5`, B `0x0077B354`). Reuse the existing controls, set the
name from ordinance slot 7/+0x1C, update the checkbox from the edited byte, and replace amount text
with the money string or empty string. Append each rendered row's **source-array index** to the row map,
A first then B. Checkbox/info IDs still start at 300/500 across the combined rendered list; scroll does
not change their interpretation into ordinance IDs (`0x0077B4EC`–`0x0077B539` for B append).

Each nonempty category pass resets +0x90/+0x94 to zero and recomputes its total over **all** matching
edited/on entries, including entries before the scroll offset and beyond the nine rows. A adds the low64
scaled amount and displays it directly. B subtracts it and displays its two-word negation
(`0x0077AF3A`–`0x0077AF7C`, `0x0077B427`–`0x0077B474`; off-screen B `0x0077B570`–`0x0077B586`).
Off entries contribute nothing. Each category's total is formatted and assigned to its total text control
before the next category starts; +0x90/+0x94 is reused scratch, not storage for both totals. A total
format call is at `0x0077B28B`; B's reset is `0x0077B2EC`–`0x0077B2F2`. This refresh does not apply
edits to the simulator; application still compares original versus edited pairs (§5.6 of the main spec).
Factory/QI failure contracts and exceptional stale controls remain part of BW25, not raw row inputs.

Conformance vectors (money formatter inputs below; locale chooses the actual currency text):

| Raw input / action | Required result |
|---|---|
| Available [expense/on -3, income/off 7, income/on 5], period 0 | Counts A=2/B=1; initial map [1,2,0]; A row texts empty then money(5); B text money(3); total inputs A=5/B=3 |
| Same inputs, period 7 | Scale=12, A total=60/B total=36; off amount remains empty, not money(0) |
| Expense/on with income +3, period 1 | B row and B total receive money(-36); no absolute-value conversion |
| Ten income/on IDs followed by two expense/on IDs; A offset 8/B offset 0 | Refresh map [8,9,10,11]; A total still includes all ten incomes; four checkbox IDs 300..303 refer to these source indices |
| Same array, change only edited byte from on to off, then refresh | Original byte unchanged; amount becomes empty, total excludes it, map/category count unchanged |
| Reverse two same-category IDs during refresh, saved edits [on,off] | The newly first ID uses on, the newly second uses off; no ID-keyed edit restoration |
| Signed amount 0x4000000000000000, scale 12, enabled A | low64 product=0; row/total formatter input 0, with no overflow error |

## 3. Neighbour population (`0x0077E600`)

### 3.1 Providers and section enumeration

Acquire the neighbours simulator through application global `0x00B43CEC`, slot 62 (+0xF8). This is an
external service accessor, not a populated budget result. Use simulator slot 27 (+0x6C) SetUIDisableFlag(1),
slot 25 (+0x64) ProposeDeals(&list,1), and slot 30 (+0x78) FindDeals(type,&list). Counts are 32-bit unsigned;
monthly deal amounts are returned in EDX:EAX. The temporary list is the owner's list at +0x130, with each
node's deal pointer at node +8. Traverse provider order; do not sort it by city or amount.

At entry zero countA/+0x124 and countB/+0x128. Erase all +0x50 rows (releasing their deal members), reset
the +0x5C end to its begin, clear held +0x68 entries and the temporary list. Then call ProposeDeals(&list,1),
traverse its results calling each deal's slot 8 (+0x20), and clear the temporary list again. That slot is a
plain dword type getter (`0x004ED900`), not accept/cancel or Release; do not invent proposal acceptance here.
Setup order is `0x0077E6A6`–`0x0077E752`.

FindDeals is invoked six times, clearing the temporary list between calls. It selects exact type equality,
not active status (`0x006D0783`–`0x006D079B`). Each returned deal contributes one row regardless of flags or
amount, and each nonempty type contributes one additional section header:

| Enumeration order | Type | List | Count sites / scan start |
|---|---|---|---|
| 1 | `0x4773FFE1` | A | `0077E761`–`0077E7A7` |
| 2 | `0x4773FFE0` | B | `0077EA2A`–`0077EA79` |
| 3 | `0x4773FFE3` | A | `0077ECFC`–`0077ED4B` |
| 4 | `0x4773FFE2` | B | `0077EFDC`–`0077F034` |
| 5 | `0x4773FFE5` | A | `0077F163`–`0077F1B4` |
| 6 | `0x4773FFE4` | B | `0077F2E3`–`0077F334` |

For every result append {type u32, deal interface pointer} to the held vector +0x68..+0x6C (8-byte entries).
The append acquires a temporary +1 and an element +1, then drops the temporary; moving/growing the vector
copies retained elements with AddRef and releases the old retained elements. Example byte sites:
`0x0077E7AD`–`0x0077E8CC`, temporary release `0x0077E9EE`–`0x0077E9F8`. Do not use the export's SliderInfo
template name to infer the semantic contents: here the pair's first word is deal type, not a slider ID.
FindDeals itself AddRefs each matching pointer before adding it to the output list (`0x006D0797`); the full
balance of the temporary list's returned references remains an explicit audit item, not an assumed borrowed list.

Only if flags (deal slot 36, +0x90) contain bit 1 (`flags & 2`) does the initializer add slot 22 (+0x58,
signed 64-bit monthly value) to that type's total, copy slot 10's dword into slot 13, copy slot 16's dword into
slot 15, and append {cityID, type} to +0x5C..+0x60. CityID is deal slot 7 (+0x1C) followed by city slot 27
(+0x6C). These are raw public interface calls; +0x5C is an owner-built list of 8-byte scalar pairs, with no
city reference retained. First type: `0x0077E8D1`–`0x0077E9EE`; second: `0x0077EBA6`–`0x0077ECC0`;
the other four repeat the flag test. A flag-zero deal is still counted and retained, but is not included
in this initial total or active-city/type list. Do not filter it out of the dialog.

After all six scans, both counts zero selects SetEmptyWindow with LTEXT group `0x6A3FF01C`, title
`0x2A579AD2`, body `0xCA5CB52D` (`0x0077F455`–`0x0077F51A`). Otherwise create the two lists with a
nine-row limit per list; headers consume rows. Render A by types E1,E3,E5, then B by E0,E2,E4, scanning
the held vector for each type, retaining order within each type. The first header consumes a row before
its deal rows (`0x0077F690`–`0x0077F6D3`); the nine-row budget is shared by the three sections of a list.
Hidden/off-screen deals remain in the retained vector and full counts.

Discriminators: E1 has 2 flag-zero deals, E3 has 1 active deal, E0 has none, E2 has 1 flag-zero deal,
E5/E4 empty. A count=5 (two headers plus three deals); B count=2. The active scalar list has only E3's
{cityID,E3}; the E1/E2 deals remain selectable. Ten E1 deals give countA=11 and at most eight initial
deal rows after the header. Three one-deal A sections give six rows, not three.

### 3.2 Combo-row ownership and selection edits

Each displayed combo row is an owner-built 24-byte record in +0x50..+0x54: its first word is the combo
control ID; +0x10 is a retained deal interface. The +8/+12 words are row/presentation metadata whose exact
values still need the bounded refresh/options audit. The append helper `0x0077E5A0` copies +0, +8, +12
and +16; it does not initialize +4 or +20 in its spare-capacity path. It
AddRefs +0x10, then advances by 24. Row construction first takes a temporary deal reference
(`0x0077F823`–`0x0077F850`) and later releases it. Erase helper `0x0077C3C0` releases the pointer at
entry +0x10 for each erased record (`0x0077C3EC`–`0x0077C3FB`). These references are separate from
the held-deal vector and cannot be replaced with a host-owned selected-deal result.

On combo change, traverse all 24-byte rows, comparing the first dword with the message's control ID;
process every match. Find the child by ID (window slot 35), QI IID `0x412CE496`, and only on success get
the selection (combo slot 30, +0x78). The row's retained deal at +0x10 is the edit target. Release the
temporary combo interface afterward. A failed QI makes no edit for that row. There is no null-child check
before QI. After traversal, always call updateNeighborBudget once and return 0
(`0x0078BD6B`–`0x0078BDFB`, `0x0078BFF2`–`0x0078C064`).

Selection 0: if deal flags bit 1 is clear, do nothing. If set, call deal slot 34 (+0x88), set combo selection
0 (slot 24/+0x60), replace its displayed item text through slot 27/+0x6C using LTEXT
`0x6A3FF01C:0xCA5793C0`, then notify through combo slot 28/+0x70 with (0,1) and deal slot 37/+0x94
with (1,1) (`0x0078BDFD`–`0x0078BE85`).

Nonzero selection S: first, if deal slot 39/+0x9C with mask 0x100 returns true, make no edit. Otherwise read
flags; when bit 1 is clear call deal slot 35/+0x8C first. Define the raw 32-bit accessor results as M=slot
18/+0x48, I=slot 16/+0x40, A=slot 10/+0x28; avoid guessing their semantic names from folded symbols.
Read M first. If M != 0, compare the **low 32 bits** of I*S to A, unsigned. When that product exceeds A,
compute k = trunc_signed(A/M) - trunc_signed(A/I) + S (32-bit differences/sum), call slot 11/+0x2C with
the low 32 bits of M*k, set combo selection 0, display LTEXT `0x6A3FF01C:0x2A5794A0`, then call
slot 17/+0x44 with a fresh M read. If M == 0 or the product does not exceed A, call slot 11 with low32(I*S),
set combo selection 0 and display the same LTEXT. The special path reads A separately for each division,
I separately for the divisor, then M again for its product; do not collapse public calls if inputs can change.
The ordinary path rereads I for its product; the special path also rereads M for its first divisor.
The signed IDIV and unsigned product comparison are intentional (`0x0078BE90`–`0x0078BFDA`).
For stable A=100,I=30,M=20,S=4: 120>100, k=5-3+4=6, set amount 120 then slot17(20). S=3 takes the
other path, sets amount 90 and does not call slot17. No floating-point funding conversion belongs here.

Raw deal primary table is at `0x00AB1DD8` (data references from `00AB1DD8` through `00AB1E74`). It is
absent from the named vtable query's results, so slot names above are behavioral descriptions, not guessed
names from folded getters. For example slot 10's `0x00A6CAB0` reads +0x34, slot 16's `0x0080BC00` reads
+0x38, slot 15's `0x006CE250` and slot 13's `0x007B62D0` are distinct setters, and slot 22's `0x006CE110`
returns +0x50/+0x54 as two words. The initializer must make the public calls, not inspect those object fields.

## 4. Panel balance-sheet computation

### 4.1 CalculateValues (`0x00767270`)

If cached budget interface +0x14 is null, obtain it through application slot 52 (+0xD0), acquire an owned
reference on a different nonnull result, replace the cached pointer and release the old pointer. If still
null, zero every dword from +0x28 through +0x14C inclusive and return (`0x00767278`–`0x007672B3`,
`0x00767805`–`0x00767981`). Do not leave stale rows on the no-budget path.

Otherwise perform the following reads/computation in table order. Each offset below holds a signed 64-bit
low/high word pair. Sum and product operations retain the low 64 bits, with no floating-point conversion,
rounding or saturation. Annual means multiply the monthly pair by 12 through `0x009F0D10`.

| Destination | Raw input / owner computation |
|---|---|
| +0x28 | budget slot 6/+0x18, total funds |
| +0x30 | budget slot 16/+0x40, monthly income |
| +0x38 | budget slot 11/+0x2C (unnamed in ABI table) |
| +0x40 | budget slot 14/+0x38, monthly expense |
| +0x48 | budget slot 13/+0x34 (unnamed in ABI table) |
| +0x50; +0x58 | sum budget slot 18/+0x48 GetTaxIncome(2), then (1), then (0); annual of that sum |
| +0x60; +0x68 | sum GetTaxIncome(5),(4),(3),(7),(6), in that order; annual |
| +0x70; +0x78 | sum GetTaxIncome(8),(10),(11),(9), in that order; annual |
| +0x80; +0x88 | zero, then ordinance income accumulation below; annual |
| +0x90; +0x98 | zero, then ordinance expense accumulation below; annual |
| +0xA0; +0xB0 | neighbours global `0x00B43D8C` slot 32/+0x80 monthly income; annual |
| +0xA8; +0xB8 | neighbours slot 33/+0x84 monthly expense; annual |

Read/call sites: `0x007672BE`–`0x00767411`, `0x007674D2`–`0x00767577`. Ordinances use the same available
ID count/array/borrowed GetOrdinanceByID providers as section 2. For each ID, call ordinance slot 5/+0x14,
**save only EAX**, then call IsOrdinanceOn(id). If off, skip it. If on, interpret the saved low dword as
signed i32: nonnegative values are sign-extended and added to +0x80; negative values are negated in 32
bits, then sign-extended and added to +0x90 (`0x00767450`–`0x007674C2`). Do not classify by
IsIncomeOrdinance here. `INT32_MIN` negates to itself and therefore adds a negative 64-bit amount to expense.
The allocated ID array is freed after traversal; no ordinance object reference is acquired or released.

Then call budget slot 29/+0x74 GetAllGroups(&temporaryVector), whose records are 40 bytes. Use the group
ID at record +0; the monthly **expense** at +24 (two words, `totalExpense`) and the monthly **income** at +32
(`totalIncome`; SDK `BudgetGroupInfo`, budget.md `GetAllGroups`; corrected 2026-10-22, re-budgetwin: the labels
were swapped, the offsets and destinations stand). Process records in returned
order, selecting destinations as follows; other IDs have no effect. Copy the chosen pair and compute its
annual pair. No clearing of these destinations precedes this loop on the successful-budget path: an absent
group retains its prior value; a later duplicate overwrites the earlier one. EACB/EA95 share one destination.

| Group ID | Input pair | Monthly; annual destinations |
|---|---|---|
| `0x0A5A72D1` | income | +0xC0; +0xC8 |
| `0xAA369059` | income, then expense | +0xD0;+0xD8 and +0xE0;+0xE8 |
| `0x4A357B40` | expense | +0xF0; +0xF8 |
| `0x6A357B7F` | expense | +0x100; +0x108 |
| `0x4A357EAF` | expense | +0x110; +0x118 |
| `0x6A357B96` | expense | +0x120; +0x128 |
| `0xEA357ECB` or `0xEA597195` | expense | +0x130; +0x138 |

Dispatch/copy sites: `0x007675C0`–`0x00767780`. Finally read budget slot 45/+0xB4 into +0x140 and compute
annual at +0x148; destroy temporary group strings and free the temporary vector (`0x00767786`–`0x00767804`).
The unnamed budget slots remain described by public slot/arguments; resolving their names is not permission
to replace CalculateValues with a host-produced balance-sheet object.

Discriminators: tax results [2→1,1→2,0→3] produce monthly 6, annual 72. An on ordinance returning
`0x00000001_FFFFFFFF` contributes expense 1, not income 8589934591; `0x00000001_00000000` contributes zero.
An income-classified ordinance with low dword -7 contributes expense 7 here but remains an A row in section 2.
Two returned groups EACB with expense 10 followed by EA95 with expense 20 leave +0x130=20, annual=240;
an update with neither ID retains that value. Missing budget resets it to zero.

### 4.2 UpdateBalanceSheetInfo (`0x00767990`) *[compat]*

*Rewritten 2026-10-22 (re-budgetwin): the control mapping is complete. Correction: `+0x198` is the **period** (0
monthly, otherwise yearly; set by the period buttons, §8), not a summary/detailed switch, and the byte `+0x194` is the
**tab** (budget-window.md §2.1), which picks the control set, not monthly versus annual (`0x007679E5`,
`0x00767A29`, `0x00767E5F`; the period buttons write `+0x198` at `0x00768B40` and `0x00768C51` through the `+8` view).*

1. Use panel window `+0x10` slot 3 and then the returned object's slot 11 (the root), with no null check
   (`0x00767998`–`0x007679A5`). Run `CalculateValues` (§4.1). Get two colours from the graphics system
   (`0x00913C1A`, slot 34): `neg` = (0x77, 0, 0), `pos` = (0x44, 0x55, 0x66).
2. With the period's income and expense, `balance := funds − expense + income` (64-bit, before formatting).
3. **Monthly** (`+0x198` = 0): funds go to both `0xAA3AC100` and `0xAA3AC10E`.
   - Tab 0 (`+0x194` = 0): `0xAA3AC100` and `0xAA3AC107` are coloured by the funds' sign (`pos` for funds ≥ 0, else
     `neg`); income `+0x30` → `0xAA3AC101`, expense `+0x40` → `0xAA3AC102`, balance → `0xAA3AC103`; `0xAA3AC103`
     and `0xAA3AC104` are coloured by the balance's sign. Nothing else is written.
   - Tab 1: `0xAA3AC10E` and `0xAA3AC10D` are coloured by the funds' sign; income → `0xAA3AC10F`, expense →
     `0xAA3AC110`, balance → `0xAA3AC111`; `0xAA3AC111` and `0xAA3AC10A` are coloured by the balance's sign; then
     the parts below (monthly column).
4. **Yearly** (`+0x198` ≠ 0); no colour is set, so the colours of the last monthly drawing stay *[quirk]*:
   - Tab 0: funds → `0xAA3AC100`, the estimated income `+0x38` → `0xAA3AC101`, the estimated expense `+0x48` →
     `0xAA3AC102`, balance → `0xAA3AC103`. Nothing else is written.
   - Tab 1: funds → `0xAA3AC10E`, `+0x38` → `0xAA3AC10F`, `+0x48` → `0xAA3AC110`, balance → `0xAA3AC111`; then the
     parts below (annual column).

| Control | Part | Monthly | Annual |
|---|---|---|---|
| `0xAA3AC112` | residential taxes | `+0x50` | `+0x58` |
| `0xAA3AC113` | commercial taxes | `+0x60` | `+0x68` |
| `0xAA3AC114` | industrial taxes | `+0x70` | `+0x78` |
| `0xAA3AC115` | ordinance income | `+0x80` | `+0x88` |
| `0xAA3AC11F` | ordinance expense | `+0x90` | `+0x98` |
| `0xAA3AC120` | neighbour deal income | `+0xA0` | `+0xB0` |
| `0xAA3AC121` | neighbour deal expense | `+0xA8` | `+0xB8` |
| `0xAA3AC122` | group `0x0A5A72D1` income | `+0xC0` | `+0xC8` |
| `0xAA3AC123` | group `0xAA369059` income | `+0xD0` | `+0xD8` |
| `0xAA3AC124` | group `0xAA369059` expense | `+0xE0` | `+0xE8` |
| `0xAA3AC125` | group `0x4A357B40` expense | `+0xF0` | `+0xF8` |
| `0xAA3AC126` | group `0x6A357B7F` expense | `+0x100` | `+0x108` |
| `0xAA3AC127` | group `0x4A357EAF` expense | `+0x110` | `+0x118` |
| `0xAA3AC128` | group `0x6A357B96` expense | `+0x120` | `+0x128` |
| `0xAA3AC129` | groups `0xEA357ECB` / `0xEA597195` expense | `+0x130` | `+0x138` |
| `0xAA3AC12A` | bond payments | `+0x140` | `+0x148` |

Every amount goes through `SetFormattedAmountInTextControl` (§4.3). A colour is set by finding the child with the
root's slot 37 (`(id, 0x212CDC1F, &text)`) and calling the text interface's slot 9 with `(0, colour)`; a missing
child is skipped and the reference is released. Sites: monthly `0x007679F1`–`0x00767E46`, yearly
`0x00767E5F`–`0x007680CC`.

### 4.3 The formatting helpers *[compat]*

*Added 2026-10-22 (re-budgetwin).*

- **`SetFormattedAmountInTextControl(id, amount)`** (`0x00765800`, int64 `amount`, `RET 0xC`): the root (as in §4.2;
  null → nothing) finds the child `id` (slot 35); null → nothing. Its text interface (QueryInterface `0x212CDC1F`)
  gets the string of `SC4MakeMoneyString(amount)` (`0x005F9AB0`) through its slot 4; without the interface nothing
  is written. The reference is released.
- **`PrintPercentageString(out, whole, tenths)`** (`0x00764D50`, `RET 0xC`): `out` := `"%d%s%d"` (`0x00AB4A68`) of
  `whole`, the locale's decimal separator (the locale service `0x00913D7A`, slot 38 with 0, then slot 15) and
  `tenths`.

## 5. Individual enumeration and restore acquisition

### 5.1 Raw list and viewport

Both `initIndividualBudget` and `updateIndividualBudget` obtain the budget simulator from global city
`0x00B43CEC` slot 52/+0xD0, then the department through budget slot 28/+0x70 with owner department ID
+0x24 (`0x007867E0`–`0x007867F9`, `0x00785818`–`0x00785843`). Call department slot 32/+0x80 with
(service kind +0x134, output list). Traverse the returned linked list in its returned order and count every
entry; store that count at +0x128 (`0x00786AC3`–`0x00786AEC`, `0x00785886`–`0x007858B2`). No extra
owner-side availability, occupancy or funding filter precedes this count. The service-kind mapping is the
main spec's section 5.5. Setup's zero-count branch uses SetEmptyWindow with body LTEXT
`0x6A3FF01C:0x4A5CB4B6`, sets byte +8 to 1, and leaves the populated path (`0x00786B6B`–`0x00786BAD`).

Refresh takes a window and a rebuild-restore byte (two stack arguments). It enumerates again rather than
reusing the setup list, resets signed-64-bit running amount +0x90/+0x94 to zero, and uses min(current
count,9) as its row quota. Every list entry is processed for service values. Rendering selects entries
whose zero-based list ordinal is at least +0x120 (unsigned comparison), until the displayed count reaches
that quota (`0x007858AF`–`0x007858EE`, `0x00785E6D`–`0x00785E85`). Thus a list of 12 entries at offset
10 displays two entries, not nine; setup count does not freeze the later list. No sort is performed here.

The service-specific amount is a 32-bit integer. If +0x28 equals 1, multiply it by 12 retaining low32
before sign-extending it into the two-word total. Every entry contributes, including off-screen entries;
addition uses ADD/ADC (`0x00785E59`–`0x00785E69`, `0x007860E0`–`0x0078612A`). Service-specific raw
query calls, rounding, and the initial amount itself remain unproved; this is an accumulation boundary,
not permission to source computed rows or totals from the original owner.

### 5.2 Restore vector references and refresh distinction

When rebuild-restore is nonzero, first clear the entire +0x44..+0x48 vector; when zero, leave it intact
(`0x007858D4`–`0x0078590A`). For **every** returned occupant, before testing viewport membership,
append one 12-byte record: the original list occupant interface pointer, followed by the two f32 funding
percentages used by RestoreStartSettings (main spec section 5.1). The pointer is not the result of the
per-entry QI for `0x26BB767E`. The two working values start as positive-zero bit patterns each iteration;
the service-kind branch can fill them before they are copied (`0x00785930`–`0x0078595A`,
`0x00785DD5`–`0x00785E59`). Exact per-service production of those floats remains an audit item.

The append takes a temporary AddRef, copies into the vector with another AddRef, then releases the
temporary: one net retained occupant reference per record. The capacity-growth path has the same retained
record, copies existing records with AddRef, then releases the old records (`0x00785DDE`–`0x00785E56`,
`0x0077D93A`–`0x0077D9A2`, copy helper `0x0077ACA0`, old-storage release/free `0x0077C480`). Clear releases each erased pointer and resets the
end without needing to free capacity (`0x00429800`–`0x00429826`). A false rebuild byte therefore preserves
the original cancel snapshot even while current service values and the viewport change. A true rebuild
replaces it with one record per current list entry, including off-screen entries. The temporary per-entry
QI is released separately at `0x007860FB`–`0x0078610B`; temporary list entries release their pointers at
`0x00786210`–`0x00786231`. Neither release consumes the vector's retained reference.

Discriminator: 12 returned occupants and rebuild=true produce 12 restore records at either offset 0 or 10.
Another refresh with rebuild=false and a changed 11-entry list leaves those 12 saved records untouched.
It recomputes the current count and total from the 11-entry list.

## 6. Group refresh (`UpdateValues`, `0x007898A0`), bounded A traversal proof

This section covers the raw provider boundary and A-side traversal/row reuse, not the complete 6265-byte
method. A means item slot 4/+0x10 returns 1; the later B pass tests 0 and still needs its own audit.

If byte +0x12C is already nonzero, return false without asking the budget provider. Otherwise set it to 1
(`0x007899DA`–`0x007899F4`, `0x00789A89`–`0x00789AB9`). Obtain the budget simulator as in section 5,
then call budget slot 29/+0x74 with an output vector of 40-byte group records. Search record word 0 for
selector +0x20; use index 0 if none matches. Then call budget slot 31/+0x7C with (chosen record word 0,
output department vector). No owner row provider supplies those results. Clear slider vector +0x2C..+0x30
on every admitted refresh (`0x00789AC1`–`0x00789B67`). No empty-vector guard appears before the selected
record dereference; do not invent a successful empty-group result for this path.
After both passes, normal cleanup releases each returned department interface separately and clears
the busy byte +0x12C (`0x0078AFDC`–`0x0078B013`). It does not retain that department vector as its next
refresh's raw input.

Only if stored A count +0x124 is nonzero, reset +0x90/+0x94, then traverse departments in provider order.
For each, obtain its item vector through department slot 21/+0x54. Traverse items in provider order, testing
item slot 4 == 1. At the first matching item, the main spec's section 4.1 header predicate applies: for
selectors `0x6A357B96`, `0xEA597195`, `0x0A5A72D1`, department slot 16/+0x40 must return a nonzero
two-word amount. Zero skips that header attempt but still proceeds to the item's rules. Otherwise admit
one header for that department (`0x00789B6C`–`0x00789C3F`).

Keep two separate counters, both initially zero: q is the traversal/scroll ordinal; p is the quota counter.
They are **not** interchangeable with a count of rows actually drawn:

| A event | Draw condition | Counter change |
|---|---|---|
| Admitted department header | q >= +0x11C unsigned, and p < 9 | q increments always; p only when drawn |
| Item with unsigned slot 3/+0x0C > 100 and zero two-word slot 14/+0x38 | Never; skip before flag check | Neither |
| Other matching item, slot 17/+0x44 bit 0 clear | Never | q only |
| Other matching item, slot 17 bit 0 set | q >= +0x11C unsigned, and p < 9 | Both q and p, even when skipped |

Header counters and tests: `0x00789C2D`–`0x00789C4A`, `0x00789DBD`–`0x00789E0D`; item tests and
counters: `0x00789E11`–`0x00789E63`, `0x0078A008`–`0x0078A01F`. Preserve the p increment for a flagged
item before the scroll offset; replacing it with a simple nine-row slice changes the refresh.

For an admitted header, add department slot 16's signed two-word amount multiplied by owner scale
+0xB0/+0xB4 (low64 product via `0x009F0D10`) to the two-word total, whether drawn or off-screen. A drawn
header reads that accessor separately for display and accumulation (`0x00789CD6`–`0x00789D2F`);
an off-screen header uses `0x00789DD0`–`0x00789E07`. Display uses SC4MakeMoneyString. Existing controls
are reused, starting at `0x0ABCDE03`: header name at current ID from department slot 4/+0x10, amount at
ID+1, and SetFlag(1,false) on ID+2; then advance control ID by 3 (`0x00789C50`–`0x00789DBD`). This
pass does not create a new child for that header. Item rendering and B-side slider/reference rebuilding
remain open; don't generalize the header rules to those paths.

Discriminator: offset 2, a department header followed by two ordinary bit-0-set A items. The header is
skipped (q=1,p=0), the first item is skipped (q=2,p=1), and the second item is eligible to draw (q=3,p=2
afterwards). An ordinal-only slice that resets p when it reaches the offset would have p=1 instead.

## 7. Bounded remainder / completion boundary

BW17 remains open, partially resolved by sections 1–6. Next bounded audits: (1) ordinance refresh, scale
initialization and B money formatting; neighbour combo options/set_comboBox, the +8/+12 metadata words,
temporary-list reference balance and updateNeighborBudget row reuse;
(2) individual service-query calls, float/amount derivation, remaining individual control reuse, malformed
initializer fragment `00786F62` and its true-rebuild call at `007875C5`; full group B refresh/slider rebuilding,
A item rendering and final stale-row cleanup; (3) popup/empty/setup
failure references and setup helpers. (The panel's loan, tax, formatting and balance-sheet rendering bodies are
§4.2–§4.3 and §8–§11, re-budgetwin 2026-10-22 and re-budgetui7 2026-10-23; `cSC4BudgetUIWinProc` is now
specified whole, and BW17's remaining audits concern `cSC4SubBudgetWindow` only.) The inventory
BW17's broad request is archived with a partial-closure answer; it does **not** mean whole-owner completion.
The remaining work is numbered separately in questions/budget-window.md: BW22 covers neighbour combo
options/set_comboBox, +8/+12 metadata, temporary-list references and updateNeighborBudget; BW23 covers
individual service-query calls, float/amount derivation (including the +0x44 saved floats), remaining
control reuse and the malformed `00786F62` initializer continuation/`007875C5` rebuild call; BW24 covers
group A item rendering, B refresh/slider rebuilding and final stale-row cleanup; BW25 covers
property replacement, popup/empty/setup and widget failure references and unclassified helpers.
The panel's loan, tax, formatting and balance-sheet rendering bodies are
§4.2–§4.3 and §8–§10, re-budgetwin 2026-10-22; their summary parts remain BW18. The inventory
above is the checklist; layout alone may stay presentation, but counts, selection, filtering, arithmetic and
owned references cannot be delegated as already computed results of these same owners.

## 8. Panel: construction, Init, notifications and the period buttons *[compat]*

*Added 2026-10-22 (re-budgetwin).* Offsets are the primary object's; `DoWinProcMessage` runs on the `+8` view, so
its instructions read these fields 8 lower (budget-window.md §2.1, BW4).

- **Construction** (`0x007658A0`): the window from the UI script (type 0, group `0x96A006B0`, instance `0xCBC3C2B9`)
  is made a child of the city view's window (generator `0x5386D516`, id `0xAA3AC000`), aligned with the city view's
  other children, given this proc, made to refuse the focus, and `InitBalanceSheet` (`0x00764DF0`) fills its labels
  (§11.1).
- **`Init`** (`0x00766E10`): the panels `0xAA3AC001`, `0xAA3AC002` (taxes), `0xAA3AC000` and `0xCA4C332D` (loans)
  are hidden; this proc is the notification target of every button below (§11.2).
- **`HideWindow`** (`0x00764AD0`): hides `0xAA3AC001`, `0xAA3AC000`, `0xAA3AC002` and `0xCA4C332D` (window flag 1
  := 0, slot 68) and clears the visible byte `+0x19D`.
- **Notifications** (`RegisterMessageNotification(on)`, `0x007648D0`): `on` and not yet subscribed (`+0x19C`):
  `+0x19C` := 1, the message target (`+0x0C`) subscribes on the message server (`0x00B43CCC`, slot 5) to
  `0x08DAC3E8`, `0x08DAC3E7` and `0x86AD10EE`, and joins the animation tick manager (`0x00602600`, slot 5) with
  priority `0xC0`; `on` false and subscribed: the reverse (slot 6 on both). `DoWinProcMessage` calls it with true on
  `0xA2BF8AD5` and with false on `0xA2BF8AD6` when `+0x19C` is set (`0x00769093`–`0x007690DD`). What the
  messages do is budget-window.md §7.
- **Period buttons**: `0x4A8879CC` and `0x4A8879DC` select **monthly** values (`+0x198` := 0); `0x4A8879CD` and
  `0x4A8879DD` **yearly** values (`+0x198` := 1). Each first updates the four toggles `0x4A8879CC`, `0x4A8879CD`,
  `0x4A8879DC`, `0x4A8879DD` (each found with root slot 37 and the check-box IID `0x8810`) and then updates the
  sheet (§4.2) (`0x00768A5A`–`0x00768C5B`). The period is also what the sub-budget dialog gets as its
  `+0x28` (budget-window.md §2 step 3).
- **Hide buttons**: `0x4A5BB1B5` and `0x6A5BB0FB` call the panel's slot 7, `HideWindow` (`0x00768C88`).

## 9. Panel: the taxes window *[compat]*

*Added 2026-10-22 (re-budgetwin).*

- **Open** (`DisplayTaxesWindow`, `0x00768130`; buttons `0xAA3AC400`, `0xAA3AC500`): for each tax type `t` = 0–11,
  its slider (`0x200 + t`, IID `0x612CE0C3`) := `round(10 × GetTaxRate(t))` (tenths of a percent; budget slot 36),
  kept as the start value (`+0x150 + 4t`); its edit field (`0x100 + t`, IID `0x231A1C57`) shows the value through
  `PrintPercentageString` (§4.3) with the whole part and the tenths. `SetTaxes` (`0x00766D00`) shows each type's tax
  income (`0x300 + t`, ×12 in the yearly view) and the totals `0x500` (types 0–2), `0x501` (3–7) and `0x502`
  (8–11). The panel `0xAA3AC002` is shown, the simulator (`0x00B43D5C`) **hidden-paused** (slot 10), the window
  location restored (`0x00B43CE0` slot 10 with `(panel, 1)`) and the panel run modally (window manager slot 41).
  After the loop, if the panel is still visible (flag 1, slot 67): the rates are restored (`RestoreTaxRates`,
  `0x00764850`), the panel hidden, its modal loop ended (window manager slot 9 with `(0xF, panel, 0)`), the
  simulator resumed (slot 13) and the sheet updated. Then the focus returns to the city view and the location is
  saved (`0x00B43CE0` slot 6) (`0x00768389`–`0x0076841A`).
- **A slider moves** (message `0x682876C9`, data1 `0x200`–`0x20C` inclusive, `t := data1 − 0x200`, data2 the value
  `v`): the edit field shows `v`, and the budget's `SetTaxRate(t, 0.1 × v)` (slot 37) applies at once
  (`0x0076884C`–`0x00768865`).
- **An edit field changes** (`0x9B1552D9`): `i := id − 0x100`. The range test rejects only `i < 0 and i ≥ 12`, which
  is never true, so every id passes *[quirk]* (`0x007690F6`–`0x00769101`). The text is parsed as a number `x`; a
  fully parsed text gives `v := round(10 × x)`; otherwise `v := round(10 × GetTaxRate(T[i]))` with
  `T` = (0, 3, 9, 1, 4, 10, 2, 5, 11) at `0x00B0D9E4` *[quirk]*: the fallback takes another type's rate, and for
  `i` = 9–11 it reads the pointers that follow the table as tax types, for which `GetTaxRate` returns 0
  (budget.md). Both roundings add 1.5 · 2²³ (`0x00AB4A18`) to the float, so a half rounds to even. The slider is set
  to `v` and the text rewritten. **Enter** (`0x9B1552D5` with key 13) moves the focus to the edit field
  `(id − 0xFF) mod 12 + 0x100`.
- **OK** (`0xAA4C353B`) and **Cancel** (`0xCA4C352F`): Cancel first restores every type's start rate
  (`SetTaxRate(t, 0.1 × start)`, `RestoreTaxRates`); both then run `CloseTaxesWindow` (`0x007680D0`): hide the panel,
  end its modal loop, the simulator's hidden resume (slot 13), update the sheet.

## 10. Panel: the loan window *[compat]*

*Added 2026-10-22 (re-budgetwin).* The loan list is a vector of `LoanInfo` records (SDK: months, years, total amount,
remaining payment, monthly payment; 0x20 bytes) at `+0x1C`.

- **Open** (`DisplayLoanWindow`, `0x00765C30`; buttons `0xAA3AC40D`, `0xAA3AC50D`): the scroll position `+0x180`,
  the amount added `+0x18C` and the chosen amount `+0x190` are cleared. The loan amount slider (`0x8A4C34D4`, IID
  `0x612CE0C3`) gets its range from the bond increment (budget slot 39) and the limits: when the total borrowed
  (slot 41) equals the current borrowing limit (slot 42) nothing more can be borrowed; otherwise the maximum is the
  smaller of the remaining borrowing room and the bond limit (slot 43) (§11.3). The texts `0xEA4C34A6`,
  `0xCA4C349D`, the monthly payment `0xCA4C34E2` (slot 48 `GetMonthlyPaymentForLoan`, ×12 in the yearly view) and
  the loan length (slot 47, months, shown in years) are set; the existing loans (slot 46 `GetAllLoans`) fill the
  list, and the count is kept twice (`+0x184` and, as the count at opening, `+0x188`). The simulator is
  hidden-paused, the location restored and the panel `0xCA4C332D` run modally.
- **The list** (`UpdateLoanDisplay`, `0x00766680`): four rows from the scroll position (ids `0x4A4C33AD`–`0x4A4C33B0`,
  `0x8A4C33D0`–`0x8A4C33D3`, `0x0A4C3405`–`0x0A4C3408`, `0xCA4C3426`–`0xCA4C3429`) (§11.4).
- **The slider moves** (`0x682876C9`, data1 `0x8A4C34D4`): the amount `0x0A4C3409` and the payment `0xCA4C34E2`
  (slot 48, ×12 yearly) are shown; the amount is kept (`+0x190`).
- **Scroll**: `0x2A4C33A6` moves the list up two rows when the position is above 0; `0xAA4C339C` down two rows while
  the position is below the count − 4; both redraw the list.
- **Accept** (`0xAA4C33E6`, `0x00768EAD`–`0x00768FA3`): a `LoanInfo` record {months mod 12 and months / 12 of the loan
  length (slot 47), the amount twice, the monthly payment `GetFullCostOfLoan(amount) / months` (slot 49; the low 32
  bits divided, unsigned)} is appended; `+0x18C` += the amount, the count `+0x184` += 1; the list is redrawn;
  `IssueNewLoans` (`0x00764BE0`) issues every record from the count at opening to the count (`IssueBond(amount)`,
  slot 40); then the window closes (`RemoveLoanWindow`).
- **Close** (`0x0A4C3354`; `RemoveLoanWindow`, `0x00764CF0`): hide the panel `0xCA4C332D`, end its modal loop
  (window manager slot 9 with `(0xF, panel, 0)`), clear the list, the simulator's hidden resume (slot 13).

## 11. Panel: the remaining bodies (BW18) *[compat]*

*Added 2026-10-23 (re-budgetui7).* Read on the instruction bytes; the export of these bodies is stack-mangled.
Offsets are the primary object's. "Root" is the panel window's root (`+0x10` slot 3, then slot 11); a control is
found by id with root slot 35 (`+0x8C`) or, with an interface, root slot 37 (`+0x94`); a caption is set with slot 74
(`+0x128`); a window flag with slot 68 (`+0x110`, `(mask, on)`). A localized string `S(i)` is the LTEXT record
(group `0x6A3FF01C`, instance `i`). The amount slider is a spinner (IID `0x612CE0C3`): slot 10 (`+0x28`) sets its
range `(min, max)`, slot 9 (`+0x24`) its value, slot 8 (`+0x20`) reads it *[inferred from use]*.

### 11.1 Construction (`0x007658A0`) and `InitBalanceSheet` (`0x00764DF0`)

1. Every field is cleared (the period `+0x198` := 0, monthly).
2. The parent is the city view's window (`0x00B43DC4` slot 3). Its current child set is recorded
   (`nSC4UI::GetWindowChildrenSet`), any window at `+0x10` is released, and the panel window is created from the UI
   script (type 0, group `0x96A006B0`, instance `0xCBC3C2B9`) as a child of the parent, with id `0xAA3AC000` and
   interface `0x5386D516`, into `+0x10`.
3. Only when that succeeds:
   - the windows the creation added to the parent are found by comparing its child set with the recorded one
     (`0x0078F3C0`) *[inferred]* and aligned to the parent (`nSC4UI::AlignWindowSetToWindow(set, 0)`);
   - the panel window's proc is set to this object's `+0x08` interface (window slot 7);
   - the accept-focus flag is cleared on the panel's whole tree;
   - `InitBalanceSheet` runs.

   A failed creation leaves `+0x10` null and sets no labels.
4. The twelve tax start values `+0x150`–`+0x17C` := 0, and the recorded set is freed.

`InitBalanceSheet` sets the caption of each label below to `S(i)`. The first fourteen are found without a null test.
The last four are set only when `0xAA3AD200` exists; otherwise it returns after `0xAA3AD123`.

| Labels | `i` when `+0x198` = 0 (monthly) | `i` otherwise (yearly) |
|---|---|---|
| `0xAA3AC108`, `0xAA3AC109` | `0x0A569E48` | `0x6A569FFD` |
| `0xAA3AC104`, `0xAA3AC10A` | `0x4A569EF8` | `0x4A569EF8` |
| `0xAA3AC105`, `0xAA3AC10B`, `0xAA3AD120`, `0xAA3AD200` | `0xAA5690B0` | `0xAA569DCD` |
| `0xAA3AC106`, `0xAA3AC10C`, `0xAA3AD122`, `0xAA3AD202` | `0x4A569F44` | `0xAA56A0F8` |
| `0xAA3AC107`, `0xAA3AC10D` | `0xCA569FA8` | `0x4A56A15B` |
| `0xAA3AD121`, `0xAA3AD123`, `0xAA3AD201`, `0xAA3AD203` | `0xCA569B75` | `0x0A569C81` |

*[quirk]* Construction is the only caller (`0x00765B70`), and it runs while the period is still monthly, so the
yearly column is never used. Switching to the yearly view (§8) changes the values (§4.2) but not these labels.

### 11.2 `Init` (`0x00766E10`), the rest

1. Hide (flag 1 := 0) `0xAA3AC001`, `0xAA3AC000`, `0xAA3AC002` and `0xCA4C332D`.
2. Make this object's `+0x08` proc the notification target (`nsGZWinUtils::SetWindowNotificationTarget(root, id,
   proc)`) of the controls `0xAA3AC40E`, `0xAA3AC50E`, `0x4A8879CB`, `0xAA3AC40F`, `0xAA3AC50F`, `0x4A8879DB`,
   `0x4A8879CC`, `0x4A8879CD`, `0x4A8879DC`, `0x4A8879DD`, `0xAA3AC400`–`0xAA3AC40D`, `0xAA3AC500`–`0xAA3AC50D`,
   `0x6A5BB0FB`, `0x4A5BB1B5`, `0xAA4C353B`, `0xCA4C352F`. Then, for each tax type `t` = 0–11, of `0x200 + t` and
   `0x100 + t`. Last come `0x0A4C3354`, `0xAA4C33E6`, `0x8A4C34D4`, `0x2A4C33A6` and `0xAA4C339C`.
3. The period toggles (button interface `0x8810`):
   - `0x4A8879CC` and `0x4A8879DC` get button slot 6 (`+0x18`);
   - `0x4A8879CD` and `0x4A8879DD` get slot 7 (`+0x1C`);
   - so the monthly pair starts selected and the yearly pair clear *[inferred]*;
   - each found reference is released.
4. `+0x198` := 0 (monthly).
5. If the amount slider `0x8A4C34D4` is found, the amount text `0x0A4C3409` shows its value (§4.3 formatting,
   sign-extended).

### 11.3 `DisplayLoanWindow` (`0x00765C30`): titles and the slider's limits

Before the list:
- the budget simulator `+0x14` is fetched when null (`0x00B43CEC` slot 52), with the usual reference swap;
- the title `0x0A4C34BE` := `S(0x8A569BD2)` in the monthly view, `S(0x4A569CE4)` in the yearly one;
- the sixteen list cells (§11.4) are set to the empty string.

Then, if the slider is found. `B` = total borrowed (slot 41), `L` = current borrowing limit (slot 42), `C` = bond limit
(slot 43), all 64-bit; `inc` = bond increment (slot 39).

| Case (64-bit compares) | Range `(min, max)` | Value | Accept `0xAA4C33E6` (flag 2) |
|---|---|---|---|
| `L − B = 0` | `(0, 0)` | 0 | off |
| `L − B < C` (signed) | `(inc, low32(L) − low32(B))` | `inc` | on |
| otherwise | `(inc, low32(C))` | `inc` | on |

A borrowed total above the limit (`L − B` negative) falls in the second row and gives a negative maximum
*[quirk]*.

Then:
- `0xEA4C34A6` := `L` and `0xCA4C349D` := `B` (§4.3 money formatting);
- the chosen amount `+0x190` := `inc`;
- the payment `0xCA4C34E2`, the loan length `0x0A4C34DB` := `S(0xEA569D4B)` formatted with (length / 12,
  length mod 12) of slot 47, and the list follow as in §10;
- the panel is shown (flag 1 := 1), its flag `0x800` set, raised (slot 23), flag `0x800` cleared, and centred in
  its parent: slot 56 moves it to ((parent slot 41 − slot 41) / 2, (parent slot 42 − slot 42) / 2), slots 41 and 42
  being width and height *[inferred]*, each half a signed division rounded toward zero (`0x00766465`–`0x0076650F`).

When the number of loans equals the maximum loan count (slot 44, a 64-bit count whose high word must be 0), the
slider is reset as in the first row (range `(0, 0)`, value 0, Accept off) before the modal loop.

### 11.4 `UpdateLoanDisplay` (`0x00766680`): the rows

Let `m` = 12 in the yearly view, else 1. The simulator is fetched as in §11.3 and the sixteen cells are emptied.
Then the slider again (`A` = the amount added this session, `+0x18C`, unsigned):

| Case | Range | Value | Accept |
|---|---|---|---|
| `L − A = B` (64-bit) | `(0, 0)` | 0 | off |
| otherwise | `(inc, low32(L) − low32(B) − A)` | `+0x190` | on |

*[quirk]* This path does not apply the bond limit `C`. Redrawing the list (scrolling, §10) therefore lifts the cap
that opening set, up to the whole remaining room.

Next:
- `0xCA4C349D` := `B + A`;
- the count `+0x184` := the number of loan records;
- for each record `k`, in order, with `r` the number of rows drawn so far: when `k ≥` the scroll position `+0x180`
  and `r < 4`, row `r` is drawn:

| Cell | Shows |
|---|---|
| `0x4A4C33AD + r` | `S(0xEA569D4B)` formatted (`Sprintf`) with record words 1 then 0 (years, then months) |
| `0x8A4C33D0 + r` | the total amount (record `+0x08`, 64-bit) |
| `0x0A4C3405 + r` | the remaining payment (`+0x10`, 64-bit) |
| `0xCA4C3426 + r` | the monthly payment (`+0x18`, 64-bit) × `m` |

- Every record, drawn or not, adds `low32(+0x18) × m` (32-bit) to a total. `0x6A4C3494` shows that total,
  sign-extended.
- Scroll-up `0x2A4C33A6` is enabled (flag 1) when the position is not 0.
- Scroll-down `0xAA4C339C` is enabled when the position is below count − 1 **and** the count is above 4 (both
  unsigned). This is not the same test the scroll handler uses (§10).
- The maximum-loan-count reset of §11.3 is applied last.

### 11.5 The small slots

`IsVisible` (slot 9, `0x00764820`) returns the visible byte `+0x19D`. `Update` (slot 8, `0x00764830`) sets the dirty
byte `+0x19E` when the panel is visible. That byte is the one `DoMessage` tests (budget-window.md §7): `DoMessage`
runs on the `+0x0C` interface, so its `+0x192` is object `+0x19E` (`0x00768641`, `lea ecx,[esi−0xC]` before
`UpdateBalanceSheetInfo`). The deleting destructor (slot 3, `0x00764CD0`) destroys the loan vector `+0x1C` and frees
the object when its flag bit 0 is set.
