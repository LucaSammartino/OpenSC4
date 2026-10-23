tag: commands-controls4-r3-off-train108-20261023-a
run: commands-controls4-r3-off-train108-20261023-a
date: 2026-10-23
lane: verify-command-system-controls4
dll: train108 SHA256 45de4459b606931d384a166726a0b1df8870465af8f84fff89cb58cf781fcd36
pacing: turbo
verdict: clean
switches: enable empty; disable budget; commands_verify=0; refaudit=1; popup_dialogs=0; autotest_end=quit; Konradshohe; seconds420; timeout600; end2032-01-01; turbo1; script=build-runs/commands-controls-save.txt (2031-12-01 save; 2031-12-15 mark commands-controls-fullsave-complete)
result: OFF lifecycle only, no command-comparison credit. Actual enabled subsystem exactly simulator; budget disabled, optional owners native. Queued02:28:04BST, started02:28:08 slot1 after4s, wall56s; clean date2032-01-07, real_seconds44.141. Full save ok1 at02:28:59.036 (run on2031-12-19),11000ms; mark same date/time; all2actions completed. RequestQuit returned1; simulator Shutdown02:28:59.472, PreAppShutdown02:28:59.486, PostAppShutdown02:28:59.512, PostSystemServiceShutdown02:28:59.585. foreign_exceptions0/watchdog_stalls0/rtc0/overruns0/ref_warnings0; summary Rust panics0/structured exceptions0/INCOMPLETE0. One retained warning: unknown config key commands_verify (separate SettingInt reader in unchanged verifier). Pair hashes/sizes verified before and after; PDB3bf57b3ab7e88eda24b5c43415cb0071280f8bf86356108f3ebe5a9cdd8464ad,171528192B; DLL18282496B. No numeric run ID assigned by harness; unique label used. Raw request game-queue/done/commands-controls4-r3-off-train108-20261023-a.req, log/verdict game-results same label, all under this worktree build-runs. Train108 is separate from unavailable historical17115a0c DLL; old disable-all stalls retained, no all-original simulator/current40/reload credit.
