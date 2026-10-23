# view3d-window: open questions
<!-- Questions about [view3d-window.md](../view3d-window.md). Answered entries move to answered/view3d-window.md. -->








<!-- from: core-view3dwin5 (2026-10-23, round 3) -->
- **VW18 - §§1.1, 5.1, 6, 6f, 6j–6l remaining opaque contracts:** how is mission activation data1 converted
  into UpdateMissionProgress's first argument? What payload fields select the advice-link subpanel/graph/budget,
  and what is the terrain debug-string contract (only its slot name is given)? Do renderer/model-maker and
  accelerator getters AddRef, or return borrowed handles? (Filter and input-control getters specify this.)
  Which background mode is selected on zoom refresh? What are the coverage-helper recalculation arguments,
  visibility/clear guards and spatial set operations beyond the two effect IDs/radius 32? The remaining §6l
  constructors are named with addresses but no owner-visible setup/teardown contracts; which extra view actions
  do they require beyond the already specified GUI lifetime? [raw MissionActivation retained without guessing a
  derived value; advice, debug/info and opaque helper operations stay foreign/open; background mode is supplied
  by the host, first-valid-picture selection and reference lifetime are implemented]
