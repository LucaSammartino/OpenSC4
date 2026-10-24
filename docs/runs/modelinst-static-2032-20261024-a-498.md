tag: modelinst-static-2032-20261024-a-498
run: 498
date: 2026-10-24
lane: glue-modelinst-shadow11
dll: FreeCity4-glue-modelinst-shadow11 (engine source d230f041b)
pacing: turbo
verdict: none
switches: train193 house switches plus modelinst_shadow=1; city=Konradshohe; end_date=2032-01-01; seconds=600; timeout=1200
result: Harness no-load: no city load within 1200 s (region view? needs a click); wall 2717 s including hung end screenshot. Coordinator confirmed harness failure, not code. No modelinst measurement; city never ran. Reference warnings 0, exceptions 0, panics 0. Retried with unique label modelinst-static-2032-20261024-b (run 499). Scope is isolated-static-transform-bounds only (MI14), not live or whole-owner coverage.
