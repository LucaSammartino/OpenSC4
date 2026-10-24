tag: overlays-shadow5-r2-2032b-372
run: 372
date: 2026-10-24
lane: glue-overlays-shadow5
dll: FreeCity4-glue-overlays-shadow5 sha256 055c5720
pacing: turbo
verdict: clean
switches: overlays_shadow=1, observe=1, popup_dialogs=0; Konradshohe through 2032-01-01
result: Clean through 2032-01-02; ref_warnings=0 and every harness problem count zero; 60.437 real seconds, 74 seconds wall, slot 1. overlays contract shadow: 73/73 lines exact mismatches=0 fields=7300 scope=sentinels. overlays records shadow: 292/292 lines exact mismatches=0 incomplete=0 checks=0 overlays=0 handles=0 groups=0 cells=2392064 entries=0 bindings=0 sources=0 unknown_bindings=0 observed_words=2398196 image_bytes=0 data=unavailable replay=unavailable(OV12) scope=observation-invariants. All four managers sampled on load and 72 months. This proves sentinel getters and empty arena/static-index observation only: no live-record getter checks, cell entries, resource metadata or image bytes were exercised. Full independent creation-history replay remains unavailable under OV12/OV13; no full-owner coverage claim.
summary: OV12 observer 292/292 exact on empty state; getters 73/73 exact
