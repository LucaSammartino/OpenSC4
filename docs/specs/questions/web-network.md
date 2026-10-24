# Open questions: web-network.md

## Q12 (re-www-ww5, 2026-10-23) — command-generator wire grammar and execution

Section 7 specifies message/poll/accept boundaries. Settle address-helper `0081e7d0` and exact
bind address/port; ReadCommandInput `0081ef00` and line reader `0081ed50` framing/limits;
malformed percent escapes in `0081ebd0`; ExecuteCommand `0081eb30` service/result mapping;
and exact response concatenation plus zero-progress/error behavior in `0081f180`. Pseudocode
for CheckForCommands incorrectly removes the accepted-command branch; instructions retain it.

