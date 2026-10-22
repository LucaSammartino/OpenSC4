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

The checkbox/control IDs start at 300 and info IDs at 500 and follow the source index (the button handlers
use id-300/id-500 to index +0xA4/+0xA8, not the category-relative visible row). The edited state controls the
checkbox and amount inclusion; the original state is only the apply-change comparator. Enabled items beyond
the ninth still contribute to category totals: the off-screen A branch is `0x0077CCFB`–`0x0077CD36`; B is
`0x0077D209`–`0x0077D24E`. A adds the helper's two-word amount; B subtracts it. The helper is the signed
64-bit arithmetic entry `0x009F0D10` (its imported template name is misleading); exact scaling, displayed
amount formatting and refreshed map reuse are reserved below until that helper/caller audit is complete.

Discriminators: available IDs [10 expense/on, 20 income/off, 30 income/on] produce counts A=2, B=1 and map
[1,2,0], including the off ordinance and all zero amounts. With ten income ordinances, only indices 0..8
have initial controls/map entries, but index 9 contributes to the total if edited/on. A provider that changes
IsOrdinanceOn between the two initialization calls can produce original != edited before user interaction.

## 3. Neighbour population

Body proof in progress; no populated list, selected-row vector or count returned by a same-owner host
callback is a valid external input. Native entries are listed above.

## 4. Panel balance-sheet computation

Body proof in progress. CalculateValues and UpdateBalanceSheetInfo are owner operations, not raw simulator
inputs. In particular the former uses full 64-bit amounts; its decompilation has corrupted stack variables.

## 5. Bounded remainder / completion boundary

BW17 remains open. Next body audits: neighbour enumeration/selection and balance-sheet arithmetic first;
then ordinance refresh/scaling, individual enumeration and +0x44 restore acquisition, group/individual/neighbour
refresh and row reuse, popup/empty/setup failure references, and loan-display/tax/setup helpers. The inventory
above is the checklist; layout alone may stay presentation, but counts, selection, filtering, arithmetic and
owned references cannot be delegated as already computed results of these same owners.
