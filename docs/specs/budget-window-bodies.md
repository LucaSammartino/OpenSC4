# Budget-window native bodies (BW17)

Companion to [budget-window.md](budget-window.md), sections 1, 4, 5.2, 5.6 and 8.
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
| GetBudgetSegment | `0077A390` | Raw-input selection helper; body remains to specify |
| SetupWindow; SetupWindowBig; SetupWindowIndiv; SetEmptyWindow | `0077A480`; `0077A6F0`; `0077A960`; `0077BEC0` | Setup/failure body gaps |
| init/updateOrdinanceBudget | `0077C660`; `0077ADA0` | Section 2 population; refresh audit remains |
| init/updateNeighborBudget | `0077E600`; `00781C90` | Section 3; remaining rendering/refresh audit recorded below |
| init/updateIndividualBudget | `00786690`; `00785740` | Occupant enumeration/restore acquisition/refresh body gaps |
| UpdateValues | `007898A0` | Group refresh and scroll reuse gap; existing visibility predicate still applies |
| RestoreStartSettings; SetOccupantFundingLevel | `0077C5E0`; `0077BC90` | Existing sections 5.1/5.5 |
| DoIndividualBudgetWindow; end_dialog | `00786530`; `00779850` | Existing nested-dialog and modal/modeless contracts |
| ApplyDealEdits (unlabelled); non-deleting destructor | `0077AB40`; `0077C4C0` | Existing apply/ownership contracts |

| Panel methods | Entries | Contract coverage |
|---|---|---|
| constructor; deleting destructor | `007658A0`; `00764CD0` | Construction/base cleanup still needs complete audit |
| Init; RegisterMessageNotification | `00766E10`; `007648D0` | Initialization/subscriptions gap |
| IsVisible; Update; ShowWindow; HideWindow | `00764820`; `00764830`; `00768670`; `00764AD0` | Visibility/tick routing, existing spec; ShowWindow invokes owner refresh |
| DoMessage; DoWinProcMessage; DoSubBudgetWindow | `00768620`; `00768750`; `00768460` | Existing command/modal/dirty routing |
| InitBalanceSheet; CalculateValues; UpdateBalanceSheetInfo | `00764DF0`; `00767270`; `00767990` | Section 4 computation; setup/rendering remainder explicit |
| SetFormattedAmountInTextControl; PrintPercentageString | `00765800`; `00764D50` | Formatting/helper audit needed |
| DisplayLoanWindow; UpdateLoanDisplay; RemoveLoanWindow | `00765C30`; `00766680`; `00764CF0` | Loan display/body gaps beyond existing take-loan routing |
| SetTaxes; DisplayTaxesWindow | `00766D00`; `00768130` | Tax edit/window bodies gap |

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
disabled rows display an empty string (`0x0077CC2E`–`0x0077CCB0`). This scale is owner state, not a refreshed
total supplied externally. Its initialization and B's visible money formatting remain in the refresh audit.

Discriminators: available IDs [10 expense/on, 20 income/off, 30 income/on] produce counts A=2, B=1 and map
[1,2,0], including the off ordinance and all zero amounts. With ten income ordinances, only indices 0..8
have initial controls/map entries, but index 9 contributes to the total if edited/on. A provider that changes
IsOrdinanceOn between the two initialization calls can produce original != edited before user interaction.

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
ID at record +0; monthly income at +24 (two words); monthly expense at +32. Process records in returned
order, selecting destinations as follows; other IDs have no effect. Copy the chosen pair and compute its
annual pair. No clearing of these destinations precedes this loop on the successful-budget path: an absent
group retains its prior value; a later duplicate overwrites the earlier one. EACB/EA95 share one destination.

| Group ID | Input pair | Monthly; annual destinations |
|---|---|---|
| `0x0A5A72D1` | expense | +0xC0; +0xC8 |
| `0xAA369059` | expense, then income | +0xD0;+0xD8 and +0xE0;+0xE8 |
| `0x4A357B40` | income | +0xF0; +0xF8 |
| `0x6A357B7F` | income | +0x100; +0x108 |
| `0x4A357EAF` | income | +0x110; +0x118 |
| `0x6A357B96` | income | +0x120; +0x128 |
| `0xEA357ECB` or `0xEA597195` | income | +0x130; +0x138 |

Dispatch/copy sites: `0x007675C0`–`0x00767780`. Finally read budget slot 45/+0xB4 into +0x140 and compute
annual at +0x148; destroy temporary group strings and free the temporary vector (`0x00767786`–`0x00767804`).
The unnamed budget slots remain described by public slot/arguments; resolving their names is not permission
to replace CalculateValues with a host-produced balance-sheet object.

Discriminators: tax results [2→1,1→2,0→3] produce monthly 6, annual 72. An on ordinance returning
`0x00000001_FFFFFFFF` contributes expense 1, not income 8589934591; `0x00000001_00000000` contributes zero.
An income-classified ordinance with low dword -7 contributes expense 7 here but remains an A row in section 2.
Two returned groups EACB with income 10 followed by EA95 with income 20 leave +0x130=20, annual=240;
an update with neither ID retains that value. Missing budget resets it to zero.

### 4.2 UpdateBalanceSheetInfo (`0x00767990`), proved computation boundary

First use panel window +0x10 slot 3/+0x0C and then returned object's slot 11/+0x2C, with no null check
(`0x00767998`–`0x007679A5`). Then always CalculateValues, obtain the two
GS colors (0x77,0,0) and (0x44,0x55,0x66), then test the dword +0x198. Zero selects the summary rendering
path, nonzero the detailed path (`0x00767990`–`0x007679EB`). The summary writes funds to controls
`0xAA3AC100` and `0xAA3AC10E`; color is the second color for signed funds >=0 and the first for negative.
Byte +0x194 selects monthly versus annual controls. Monthly projection is funds - monthly expense + monthly
income, using 64-bit subtraction/addition before money formatting (control `0xAA3AC103`,
`0x00767AC2`–`0x00767ADE`); its sign selects the same positive/negative colors. The detailed control
mapping and the remainder of summary rendering are still pending; they cannot be represented as an external
refresh callback. SetFormattedAmountInTextControl remains an owner helper requiring its separate widget audit.

## 5. Bounded remainder / completion boundary

BW17 remains open, partially resolved by sections 1–4. Next bounded audits: (1) ordinance refresh, scale
initialization and B money formatting; neighbour combo options/set_comboBox, the +8/+12 metadata words,
temporary-list reference balance and updateNeighborBudget row reuse;
(2) individual enumeration/+0x44 restore acquisition and group/individual refresh reuse; (3) popup/empty/setup
failure references, loan-display/tax/setup helpers and remaining balance-sheet control rendering. The inventory
above is the checklist; layout alone may stay presentation, but counts, selection, filtering, arithmetic and
owned references cannot be delegated as already computed results of these same owners.
