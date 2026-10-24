# Nation (`cSC4Nation`): short-form spec

Status: NQ1–NQ4 closure (re-nation-nq3 / re-nation-nq4, 2026-10-24). Written from the Windows 1.1.641 executable (code `0x00497220`–`0x00499D3F`,
vtable `0x00A8EF00`). It settles what the nation adds beyond neighbors.md, foundations §11 and demand.md §8.2.

Tags: *[compat]* observable, must match; *[quirk]* original behaviour that a replacement must reproduce.

## 1. Role

The nation belongs to the application (`cISC4App::GetNation()`), not to a city. It is **not** a city component and
writes **no** save record. It holds:
- the list of regions found on disk (the `Regions` folders and the downloaded regions), the active region, and the
  region directories;
- the national economy curves and the difficulty modifiers (exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F6115`,
  demand.md §11.2), read on each transition from shut down to initialized (§6).

Everything except `GetEconomyModifier` is region-view and file-system work (loading, adding, deleting and copying
region folders, choosing the region from the preferences). No simulator reads it. The region and city records
themselves are covered by neighbors.md and foundations §11.

## 2. `GetEconomyModifier(demandID)` (slot 16, `0x00497570`, pops 4, returns x87) *[compat]*

demand.md §8.2 gives the formula. The bytes add two details:

1. `months := (simYear − startYear) × 12 − startMonth + simMonth`, as a 32-bit integer, is converted to float as an
   **unsigned** value: a negative month count (a simulation date before the start date) becomes `months + 2^32`, and
   the curve is evaluated far past its last point *[quirk]*.
2. The curve value is rounded to float. The difficulty modifier `difficulty[City.GetDifficultyLevel()]` is added
   **only when a city exists** (the app's city pointer is not null); with no city the result is the curve value
   alone.

The curve lookup is the demand ID, and ID 0 when the demand ID has no curve (demand.md §11.2 has the failure cases).

### 2.1 Malformed tuning: defined replacement outcomes (NQ3)

*[choice]* An explicitly present **empty ID list** stays empty; it does not request the missing-property
defaults in demand.md §11.2. If neither the requested ID nor ID 0 exists, return `MissingCurve`, with no
numeric result. A short (including empty) explicitly supplied difficulty vector keeps its supplied length:
with a city, an index outside that length returns `InvalidDifficulty`, with no numeric result. With no city,
do not inspect that vector or request a difficulty index. Missing-curve failure takes precedence when both
conditions apply. These choices match core-nation10's proposed errors; do not synthesize ID 0, pad an explicit
vector, clamp an index, or return an arbitrary float. They deliberately replace undefined original reads:
`0x004975EA–0x0049760D` uses the fallback iterator without a second end check; `0x00497639–0x00497648` reads the
indexed float without checking the vector length. Missing exemplar/properties still use demand.md §11.2's
explicit FreeCity4 defaults; an existing empty curve evaluates to zero under that existing choice.

## 3. Messages *[compat]*

`Init` subscribes to `0x26D31EC1` (PostCityInit), `0x26D31EC2` (PreCityShutdown) and `0xAC3B3FA6`. Only `0xAC3B3FA6`
is handled: it is posted by the game preferences window, and it calls `ResetTutorialCities` (slot 14). The two city
messages are subscribed and ignored. `Shutdown` unsubscribes all three.

## 4. Slot table (`cISC4Nation`, vtable `0x00A8EF00`) *[compat]*

| Slot | Function | Pops |
|---|---|---|
| 0 | QueryInterface | 8 |
| 1 | AddRef | 0 |
| 2 | Release | 0 |
| 3 | Init (`0x00498750`) | 0 |
| 4 | Shutdown (`0x00497A80`) | 0 |
| 5 | GetRegionDirectory | 0 |
| 6 | GetRegionCount (`(end − begin) / 4`) | 0 |
| 7 | GetActiveRegionIndex (`+0x20`) | 0 |
| 8 | GetActiveRegion (the region at the active index) | 0 |
| 9 | SetActiveRegion(index) (`0x004972F0`): deactivates the old region, activates the new one (region slots 9 and 8) | 4 |
| 10 | GetIndexOfCurrentRegionSettingInPreferences | 4 |
| 11 | GetRegion(index) | 4 |
| 12 | DeleteRegion | 4 |
| 13 | AddNewRegion | 4 |
| 14 | ResetTutorialCities | 0 |
| 15 | GetRegionDownloadDirectory | 0 |
| 16 | GetEconomyModifier(demandID) | 4 |
| 17 | deleting destructor | 4 |

The message target (`DoMessage` `0x00497290`) is the sub-object at `+4`.

## 5. Conformance tests

1. With the shipped tuning, `GetEconomyModifier(any)` with a city at difficulty 0, 1, 2 returns 0.06, 0.03, 0.01;
   with no city it returns 0.
2. A simulation date one month before the start date evaluates the curve at `2^32 − 1` (the last curve point's
   value with the shipped flat curve, so still 0 + difficulty).

### 5.1 Owner-contract vectors

Callbacks below are an ordered trace; results of region activation/shutdown may be forced false. A catalog
fixture supplies the directory enumerator's order rather than relying on a host filesystem's enumeration order.

| Input/state | Expected result and effects |
|---|---|
| Constructed nation | initialized false, empty catalog, active index 0; this is not a usable selected region. |
| Catalog `[A,B]`, active 0; select 0 | No callbacks, index stays 0. |
| Catalog `[A,B]`, active 0; select 1, both callbacks return false | `A.Shutdown`, then index becomes 1, then `B.Init`; no rollback or retry. |
| Catalog `[A,B]`, active 0; get/select index 2 | Replacement `InvalidRegionIndex`, no callback or mutation (§7). |
| Root entries in order: directory B, regular file X, directory dOwNlOaDs, directory A | Catalog `[B,A]`; region setters/read run in that order. No recursive discovery of Downloads. |
| Regions named `Alpha`, `Beta`; preference bytes `Beta` followed by NUL and junk | Preference method returns true and writes index 1. |
| Same catalog; preference `beta`, or `BetaX` | False and output index 0: comparison is case sensitive and length sensitive. |
| No preference match; first region fails Init, second has no tutorial city | Initial candidate remains index 0, not the first region whose Init succeeds; final Init failure requests tutorial reset. |
| Already initialized; call Init again | True, no new discovery, tuning read, subscriptions, activation, or reset. |
| Initialized with two regions; Shutdown twice | First call unsubscribes three IDs, shuts down active region, releases both catalog entries, deletes all curves; second call has no effects. Both return true. |
| Shutdown then Init with changed tuning | Catalog rediscovered and tuning reread; directory StaticInit does not repeat. |
| `[A,B,C]`, active A; delete B | Select B (`A.Shutdown`, `B.Init`), `B.Delete`, switch to C (`B.Shutdown`, `C.Init`), remove B; catalog `[A,C]`, active 1. Disk-removal failure does not restore B. |
| `[A,B,C]`, active A; delete C | Select C, delete it, switch to B; catalog `[A,B]`, active 1. |
| `[A]`; delete A | Replacement `LastRegion`, no effects; original attempts selection of −1 (§7). |
| Add a name whose factory returns null to catalog of length 2 | Original appends null, returns 2, leaves active index unchanged. Replacement reports `RegionCreationFailed` before catalog mutation (§7). |
| Explicit empty ID list, any demand, city absent or present | `MissingCurve`. |
| Only ID 7 has constant curve 2; request ID 8 | `MissingCurve`, even with no city. Request ID 7 with no city returns 2. |
| ID 0 constant 2, explicit difficulty `[0.25]`, city difficulty 1 | `InvalidDifficulty`; no city returns 2; city difficulty 0 returns 2.25. |
| ID 0 empty curve, difficulty `[0.25]`, city difficulty 0 | 0.25, under the existing empty-curve choice. |
| Reset with no app, or with a live city | No filesystem, registry, catalog, or screen effects. |
| Registered tutorial folder exists with both `*.sc4` and `region.ini` | Keep that region; skip template-directory copying; export tutorial records and update registry active flags (§8). |
| Registered folder exists but has no `region.ini` | Delete region, recreate it, copy template files, select replacement, then export tutorial records; failures do not roll back earlier effects. |

### 5.2 Directory case-map vectors (NQ4)

`Lower` below means §6.4's directory-name transform. Text is UTF-8; hex lists specify literal bytes.
Pin the CRT locale in the fixture: `C` means its invariant C locale; `en-US` means the Windows English
(United States) LC_CTYPE locale (LCID `0x0409`, code page 1252). These are static conformance expectations,
not measurements from a game run. Do not substitute the test runner's unspecified default locale.

| Locale and input/state | Expected result |
|---|---|
| C or en-US; `Lower("TuToRiAl")` | `tutorial`. |
| C; `Lower("ÉCOLE")`, bytes `C3 89 43 4F 4C 45` | `École`, bytes `C3 89 63 6F 6C 65`: accented capital is retained. |
| en-US; same input | `école`, bytes `C3 A9 63 6F 6C 65`: non-ASCII bytes are not simply preserved. |
| en-US; `Lower("É")` versus `Lower("É")` (the latter is `45 CC 81`) | `C3 A9` versus `65 CC 81`, unequal: no normalization. |
| C; `Lower` of bytes `41 E9 42` | Only `61` remains. The code-page-1252 byte E9 is not a complete UTF-8 sequence; neither it nor the suffix is preserved. |
| C; `Lower` of bytes `F0 9F 98 80` (U+1F600) | `EF 98 80` (U+F600), from the converter's 16-bit truncation, not a surrogate pair. |
| No preference match; catalog `[Other, ÉCOLE]`, both probe successfully and contain tutorial cities; supplied registry default bytes are `école` | en-US selects index 1; C keeps provisional index 0. The registry result itself is not folded by Init. |
| Reset; registry directory key `école`; catalog `[ÉCOLE, école]`, each folder valid under §8 | en-US retains the first region for that key; C retains the second. Reset folds catalog names, then takes the first byte-equal match. |
| en-US; raw registry default supplied as `ÉCOLE`; catalog directory `ÉCOLE` | Init's comparison fails (`école` versus verbatim `ÉCOLE`). Ordinary registry initialization avoids this by folding its stored names. |
| en-US; display name `ÉCOLE`, preference `école` | Preference lookup is false with output 0. Directory folding never applies to preference display names. |

## 6. Initialization, discovery and shutdown (NQ1) *[compat]*

### 6.1 Directory setup and catalog

The constructor starts uninitialized, with an empty catalog and active index **0** (`0x00497698–0x004976A7`).
The region directory and download directory are shared process strings, returned by reference by slots 5 and
15 (`0x004976F0`, `0x00497700`); they are not a list of additional roots. `StaticInit` (`0x00498160`) has a
process-wide guard, set **before** checking the app pointer. With an app it obtains the user regions root
through app slot 60, creates it if absent, derives `<root>Downloads\`, and creates that if absent. Directory
creation failures are ignored; the static routine reports true and will not retry on later calls. Calling it
without an app consumes the guard too *[quirk]*.

`LoadRegions` (`0x00497CF0`) enumerates the immediate root entries in directory-provider order. It accepts
directory entries (entry kind 4), excludes `Downloads` by a **case-insensitive, equal-length** string comparison
(flag 0), and does no further nation-level sorting. Each accepted entry gets a new region,
appended to the catalog **before** its directory name and initial display name are set from the entry name;
then region slot 7 (`ReadRegionINIFile`) runs. Its result is ignored. The load result is whether at least one
entry was accepted, not whether all region INI reads succeeded. No second scan of Downloads is performed.
This exclusion lowercases both strings before comparing bytes (`cRZString::IsEqual`, `0x0090EC57`);
it does not make the separate preference comparison case insensitive.

If discovery accepts nothing, `CopyNewUserRegions` (`0x004985A0`) tries installation templates, then repeats
discovery. It returns false without copying if the root string is empty or the root directory does not exist.
Otherwise it copies `<DataDirectory>Regions\`, then, when a language manager exists,
`<DataDirectory><language-directory>\Regions\`, into the user root and returns the repeated discovery result.
`CopyRegionFiles(destination, source)` (`0x00498270`) walks source files recursively, preserves each relative
path, creates a missing destination parent directory, copies with **overwrite enabled**, and makes the
destination writable. It ignores directory-creation, copy and attribute results and continues through the
file list; it neither rolls back files nor returns an aggregate success. Overwrite is verified by
`cRZFile::Copy`'s inversion of its true argument at `0x0091A32A–0x0091A335`; writable clears the Windows
read-only bit (`0x0091A0AB–0x0091A0C0`). Empty source directories need not be reproduced.

### 6.2 Init and preference selection

An already initialized nation returns true without work. Otherwise `Init` sets its initialized flag first,
runs StaticInit, **rereads tuning**, clears/releases any existing catalog entries, and tries discovery,
template copying/discovery, then `CreateStarterRegion` if both fail. The starter factory's result is appended
without a null check. No previous catalog is restored after a failure. These calls are visible at
`0x00498759–0x004987A0`; the apparent `vector::size` export label at `0x00527180` is actually range erasure
with releases and end-pointer adjustment, not a count query.

`GetIndexOfCurrentRegionSettingInPreferences(outIndex)` (`0x00497820`) is a bool operation with an output
32-bit index. It initializes the output to 0, takes the preference's fixed 64-byte current-region name
(preference offsets `+0xEBC…+0xEFC`), truncates at its first NUL if present, then finds the first catalog
**display name** whose first at most 64 bytes compare equal to it. The comparison is bytewise, case sensitive,
with lengths significant, not a path comparison, directory-name comparison, or case-folded match. It returns
true and writes that index on a match; false with output 0 otherwise (`0x00497962–0x00497976`, helper
`0x00497420`). A name longer than 64 bytes can match its 64-byte preference prefix.

Init sets active index −1 before this preference lookup. A match supplies the selected index directly.
Without a match, Init uses the first catalog index as its provisional candidate, even if that region later
fails activation. It probes regions in catalog order with `region.Init`; successful probes inspect their
city list for any tutorial ID other than −1 and then call `region.Shutdown`. Failed probes are not shut down
by this loop. Among those containing tutorial cities it prefers the first whose lowercased **directory
name** equals the tutorial registry's default directory name for the current language. Otherwise the
provisional first index remains selected. Evidence: `0x00498843–0x00498965`, `0x004989D6–0x00498AF5`.

Init calls the selected region's Init once more (not SetActiveRegion, so no old-index access). It subscribes
the three messages in §3 when the message server exists. It requests `ResetTutorialCities` if selection is
negative or that final activation fails. Otherwise, when no catalog directory matches the current-language
default tutorial directory (after lowercasing the catalog directory), it requests reset only if app slot 50
returns true (`0x00498B36–0x00498CCC`). It returns true regardless of these bool results and keeps its flag
set; it is not an all-or-nothing initialization operation.

### 6.3 Shutdown and reinitialization

`Shutdown` (`0x00497A80`) always returns true. If not initialized it has no effects. Otherwise it clears the
flag first, unsubscribes the three IDs if a message server exists, calls the active region's Shutdown,
releases/erases every catalog entry, destroys every response curve and clears the curve map. It does not
delete region files, reset the active index, clear the difficulty vector, or reset StaticInit's process guard.
Callback return values are ignored. A subsequent Init rediscovers the catalog and rereads tuning; the earlier
"read once per process" description meant the ordinary single application lifetime, not a reinit guarantee.
Difficulty resize-to-three retains surviving elements before new tuning is read; shutdown has not zeroed them.

### 6.4 Directory-name encoding and case mapping (NQ4) *[compat]*

Tutorial directory names use the `cRZString::MakeLower` transform (`0x0090EAC5`), whose string encoding is
**UTF-8**, internal encoding ID 8, rather than the Windows ANSI code page. Its case map is the **active CRT
LC_CTYPE locale**, not a fixed ASCII-only table or a locale-independent Unicode case fold. For an initial
run of bytes below `0x7F`, each uppercase byte is replaced by the CRT narrow lowercase result. If any byte
is `0x7F` or greater, the whole string, including the already transformed prefix, is converted from encoding
8 to encoding 16; each resulting 16-bit character receives CRT `towlower`, and the result is encoded back
to 8. Thus ordinary BMP UTF-8 names receive locale-dependent lowercase, without normalization or
multi-character case expansion. In the C locale, wide lowercase changes only U+0041–U+005A; other
characters remain unchanged. In a non-C locale it uses the CRT's Windows lowercase mapping, retaining
the input character if mapping fails. The language manager can set the CRT locale from its selected
language's locale name; a failed locale change leaves the previous map in force. Merely selecting a
tutorial default name does not define a new case map. Evidence: threshold/narrow path
`0x0090EAE4–0x0090EB0D`, fixed encoding conversions and word-sized map
`0x0090EB3E–0x0090EB72`, encoding-8 decoder `0x00910EDF–0x00910F69`, C/non-C wide mapping
`0x009F4C59–0x009F4C78` / `0x009F4BD4–0x009F4C32`, and language locale change
`0x00928058–0x0092807F`.

*[quirk]* This converter is older than a conforming UTF-8/UTF-16 round trip: four-byte characters become
one word holding the low 16 bits, rather than a surrogate pair. Invalid or incomplete input stops decoding
at that sequence and keeps only the converted prefix; MakeLower ignores the conversion status and
replaces the string with that prefix. Do not preserve arbitrary high bytes as a fallback. The word store,
prefix length, and unconditional replacement are at `0x0091112C–0x00911145`,
`0x0090EB46–0x0090EB72` and `0x009113CB–0x009113DE`; the four-byte calculation is at
`0x00910FD3–0x0091103E`. No nation-specific validity filter repairs these names.

Init compares the transformed catalog directory to the registry default **verbatim**, including length.
Reset likewise compares each transformed catalog directory to the registry's supplied directory key,
verbatim, taking the first match (`0x00498FE6–0x00499015`). In the ordinary registry, both the
language-default names and tutorial `region_dir` keys were already transformed with MakeLower when
loaded (`0x004B7994–0x004B79A9`, `0x004B7BAE–0x004B7BCA`). This explains the apparent folding of
both sides in §8: do not add another transformation to the registry result at the nation boundary.
If the locale changes between loading the registry and comparing catalog names, the stored key is not
automatically remapped. All final comparisons are byte/length equality, not filesystem case comparison.
The preference display-name comparison in §6.2 remains case sensitive. §5.2 pins locale, encoding and
comparison boundary in conformance vectors.

## 7. Catalog getters, selection, creation and deletion (NQ1/NQ2)

`GetRegionCount` returns the catalog length. `GetRegion(index)` returns its borrowed pointer without AddRef;
`GetActiveRegion` is the equivalent indexed lookup using the active index. Original lookups have **no**
bounds checks (`0x00497740–0x0049774C`, `0x00497770`). Selection has a void contract: an equal index is an
absolute no-op; otherwise it shuts down the old indexed region, stores the new 32-bit index, and initializes
the new indexed region. Both callback results are ignored, with no retry or rollback (`0x004972F3–0x00497318`).
An invalid old or new index, null entry, or getter on an empty catalog can access invalid memory; there is no
original recoverable error or special "none selected" handling.

`AddNewRegion(name)` (`0x00497EE0`) takes a string reference, invokes the static region factory described in
region.md §6, appends the returned region, and returns the new last index (32 bits). It does not select,
initialize, deduplicate, or inspect the new region. Even a null factory result is appended and its index
returned *[quirk]*. Lower-level filesystem failures are those of the factory, not transactional nation errors.

`DeleteRegion(index)` (`0x00497F60`) is void and takes a catalog index, not a region pointer or path. It first
**selects the region being deleted**, even if another was active, then invokes that region's Delete (region.md
§6). Since the target is now active, it selects its next catalog neighbor, or its previous neighbor when the
target is last. It attempts to remove `<root><target-directory-name>` with removal flag 0, erases/releases the
target entry while preserving remaining order, and decrements the active index if the erased index was
smaller. It ignores disk and region results: catalog removal is not undone if files remain. The final-neighbor
selection precedes catalog compaction (`0x00497F6C–0x00497FA4`, `0x00498033–0x00498076`). Deleting the sole
region attempts selection −1 and is invalid original behavior, not a successful transition to an empty nation.

*[choice]* For safe replacement calls, reject invalid indices (including a missing current region for a
nontrivial selection) as `InvalidRegionIndex` before effects. Reject deletion of the last region as
`LastRegion`, and reject a null factory result as `RegionCreationFailed` before appending it. A valid
same-index selection has no callbacks. These protections apply only to invalid original accesses; do not
turn ordinary callback/file failures into rollback. Void original operations need not manufacture bool
success values, and any replacement status must distinguish its safety error from an original return value.

## 8. Tutorial reset (NQ1) *[compat]*

`ResetTutorialCities` (`0x00498CF0`) is a **void** operation. With no application, or while the app has a live
city, it does nothing (`0x00498CFD–0x00498D17`). Otherwise it obtains the tutorial registry's ordered region
names, moves the current-language default tutorial name to the front if present, and processes that list.
If the current screen reports class ID `0xEA659793`, it destroys that screen before processing and recreates
the region screen at the ordinary end (`0x00498D33–0x00498E93`, `0x00499C7C–0x00499C84`).

For each tutorial directory key supplied by the registry, it looks for the first catalog region whose
**lowercased directory name** equals that key byte for byte (§6.4 defines the transform and the registry's
earlier folding). A found region is retained only when its user directory exists and contains both a `*.sc4` match and
a `region.ini` match. A missing directory or either missing pattern causes DeleteRegion and recreation. An
unlisted directory at that root/name is removed with recursive flag 1 before creation. Creation uses
AddNewRegion(name), then GetRegion(returnedIndex); a null region causes an early return without undoing prior
effects or taking the normal screen-recreation path (`0x00498F21–0x00499547`). It does not restore the region
that had been active before reset.

Only newly created/recreated tutorial regions receive a recursive copy from
`<DataDirectory>Regions\<tutorial-name>\` into their user region directory. Each source-relative suffix is
preserved, copy enables overwrite, and the destination is made writable; the copy/attribute results are
ignored (`0x00499604–0x004998C5`). A retained region skips this template copy, but still receives the tutorial
city exports below. This validity check does not verify that every tutorial file is intact.

Each processed region is selected through SetActiveRegion. The registry supplies that region's tutorial
entries. Each entry ID exports the resource `0x6A5B7BF5 / 0x6A5B7B57 / id` to
`<user-region-directory>Tutorial-City-%08x.sc4` (eight lowercase hexadecimal digits). Export result −1 or a
failed regional-city read marks that tutorial **inactive** and processing continues. On successful export
and read, the city receives the saved path, its position is obtained, it is installed with the region's
MoveCity operation, then shut down/released, and that tutorial is marked **active**. The MoveCity and city
shutdown results are not checked before setting active (`0x0049995A–0x00499B99`). Thus reset can leave a
partially reset catalog/files/registry; it has no aggregate success return and no rollback. The last processed
tutorial region remains selected, subject to the explicit same-index no-op rule.
