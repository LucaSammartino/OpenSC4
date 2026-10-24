tag: overlays-shadow5-r2-2032a-349
run: 349
date: 2026-10-24
lane: glue-overlays-shadow5
dll: FreeCity4-glue-overlays-shadow5 sha256 83ac52da
pacing: turbo
verdict: clean
switches: overlays_shadow=1, observe=1, popup_dialogs=0; Konradshohe through 2032-01-01
result: Clean through 2032-01-09; ref_warnings=0, all harness problem counts zero; 47.719 real seconds, 66 seconds wall, slot 1. overlays contract shadow: 73/73 lines exact mismatches=0 fields=7300 scope=sentinels. overlays records shadow: 146/292 lines exact mismatches=146 incomplete=146 checks=146 overlays=0 handles=0 groups=0 cells=2392064 entries=0 bindings=0 sources=0 unknown_bindings=0 observed_words=2395130 image_bytes=0 data=unavailable replay=unavailable(OV12) scope=observation-invariants. Every dynamic sample failed at rust-invariants; static samples complete but contain no live overlays. Diagnostic evidence retained; validator incorrectly constrains nonapplicable dynamic static-grid fields (OV14).
summary: OV12 diagnostic run; getter exact, dynamic observer rejected
