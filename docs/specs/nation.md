# Nation (`cSC4Nation`): short-form spec

Status: round 1 (re-specs14a, 2026-10-15). Written from the Windows 1.1.641 executable (code `0x00497220`–`0x00498CF0`,
vtable `0x00A8EF00`). It settles what the nation adds beyond neighbors.md, foundations §11 and demand.md §8.2.

Tags: *[compat]* observable, must match; *[quirk]* original behaviour that a replacement must reproduce.

## 1. Role

The nation belongs to the application (`cISC4App::GetNation()`), not to a city. It is **not** a city component and
writes **no** save record. It holds:
- the list of regions found on disk (the `Regions` folders and the downloaded regions), the active region, and the
  region directories;
- the national economy curves and the difficulty modifiers (exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F6115`,
  demand.md §11.2), read once per process.

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
