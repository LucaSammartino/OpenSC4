| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fwmisc7c: small framework owners 3 (record copies, small constructors, type IDs, allocator and lock helpers) plus rows for five owners other specs already specify, framework-misc3.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-gzcomfactories7) | **ready** (docs only). 29 owners claimed: specified +2,800 bytes (+61 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The 24 owners of the new spec were read in the decompile and, for the small bodies, the disassembly. For the five
row-only owners the existing text was read first: renderer-gdrivers-director.md and app-directors.md describe every
`cRZCOMDllDirector` slot and helper; widget-toolkit.md §14.7 the three `cGZFadeTmpWin` functions; lua-host.md the
CRC-24 of `nRZHash`; framework-gz-services.md §4 the `cRZThreadSignal` constructor; gz-framework-core.md the two
`cGZCommandSystemSupport` bodies. `cGZScreenBuffer::Init` was left out (gz-graphics.md GZG3, screen buffer defaults,
is open upstream). Written on 2026-10-24.

### What changed

- New docs/specs/framework-misc3.md.

### Quirks found

- The scoped lock's constructor is exported as `cRZLock::~cRZLock`; `cRZCellMap<bool>::Fill(true)` also sets the
  padding bits of each row's last word.

### Coverage

- tools/coverage-map.tsv: 24 rows after `cRZExceptionNotification`; the `cCompletionObj`, `SignalPlatformData`,
  `ibasic_string`, `RZFileFindData` and `FileChangeNotification` rows of the attributed-owner block filled in place.
- Specified bytes 4,930,484 → 4,933,284 (+2,800, 88.3%, `cov_local.py`).
