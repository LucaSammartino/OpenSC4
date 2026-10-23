| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-radio: the disk radio, its stations and the playlist reader (new radio.md), and the intro video player window (new intro-video-player.md) (RE spec lane) | `re-batch6` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4f059a7`, after re-freshness) | **ready** (docs and RE tables only). radio.md §1–§7 (the event channel, Init/Shutdown, settings and options, station building from `Res\Music\Radio\`, the station programme and its 4 s gap, item choice, shuffles, M3U/EXTM3U/PLS); intro-video-player.md §1–§4; questions RA1–RA4; 18 names (10 new); `cRZRadio`, `cRZDiskRadioStation`, `cGZPlayList`, `cSC4MadCowPlayerWin` claimed: specified +18,204 bytes (+146 functions) | RA1–RA4; the MAD decoder |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`),
the Ghidra export of the four owners and the SDK headers. Written on 2026-10-22.

### What changed

- New docs/specs/radio.md. Byte-checked: the three radio vtables (`0x00ACC3C0`, `0x00ACC310`, `0x00ACC2FC`) and the
  station's (`0x00ACC290`); the folder and pattern strings (`0x00B16040`–`0x00B16064`); the six event ids;
  `GetOption`'s three indices (`0x008D2BA3`) against the station's four option tests (`0x008D2EB0`, the `push 2`
  that makes jingles unreachable); the playlist separators and keywords.
- New docs/specs/intro-video-player.md for the 16 functions owned by `cSC4MadCowPlayerWin` (the decoder stays out).
- audio.md §2: the radio row points at radio.md.
- docs/specs/questions/radio.md: RA1–RA4 (prefix new); QUESTIONS.md regenerated.

### Names and coverage

- re/symbols/radio.tsv: 18 names (10 new: the radio's idle tick, station select, volume setter, file start and
  current-song setter; the MadCow window's QI, surface, activation handler, shutdown and stop helper; 8 confirm auto
  names). Delta (−8 +18) identical to `merge_names.py`'s.
- tools/coverage-map.tsv: the four rows cite radio.md / intro-video-player.md. Specified bytes 3,545,325 → 3,563,529
  (63.5% → 63.8%, `cov_local.py`); audio 50.2%.

### Rebase onto `15c9ce9e` (2026-10-23)

Upstream's docs/specs/sound-system.md §9 now owns `cGZPlayList` by coverage row; that row stays there and radio.md §4
links to it (the two agree on the format choice and the `#EXTINF:` pairing). `cRZRadio`, `cRZDiskRadioStation` and
`cSC4MadCowPlayerWin` were still unclaimed upstream and stay on radio.md / intro-video-player.md. The numbers above
were measured on `a4f059a7`.

### Questions

- New: RA1 (event `0x4AB186E9`'s effect), RA2 (the shuffle-strategy bit), RA3 (two flags), RA4 (a game check of
  the jingle quirk).

### Not covered

The MAD video decoder and frame loop; the language manager's folder names.
