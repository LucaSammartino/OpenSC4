# UI window driver questions

- **UI29-TREE (reach-ui29, 2026-10-26):** please qualify the UI-script
  service slot-4 key/parent/id contract, partial-creation cleanup, and the
  prerequisites for initializing the import template tree. A test using
  key `{0,96A006B0,8A5AB1CD}`, the main parent and ID FC402901 returned null
  yet left a visible child 00001000. Clicking it and sending Down entered
  GZOnKeyDown (009D14BF) and raised an access violation at 009D1676.
  Evidence: shared game-results/reach-ui29-p7.log lines 6472, 6624, 6739;
  corrupt/ref_warnings=0, no completed reach trace. The candidate driver was
  removed; there is no qualified template/import driver or tree reach claim.
