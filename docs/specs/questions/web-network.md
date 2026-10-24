# Open questions: web-network.md

## Q12 (re-www-ww5, 2026-10-23) — command-generator wire grammar and execution

Section 7 specifies message/poll/accept boundaries. Settle address-helper `0081e7d0` and exact
bind address/port; ReadCommandInput `0081ef00` and line reader `0081ed50` framing/limits;
malformed percent escapes in `0081ebd0`; ExecuteCommand `0081eb30` service/result mapping;
and exact response concatenation plus zero-progress/error behavior in `0081f180`. Pseudocode
for CheckForCommands incorrectly removes the accepted-command branch; instructions retain it.

<!-- from: core-www-close15 r2 (2026-10-24) -->
- **Q19 - FTP initialization and conflicting listing helper descriptions:** after the correction of `00813d70` to scheduler construction in FTP-control 9.1, FTP-control 6 still requires initializing a fresh session before login, but section 7 specifies only construction/shutdown/destruction. Which actual entry and effects implement that initialization? Also confirm that parent 4.15's close-listener `00811f00` / close-data `00811f90` replace FTP-control 3's older receive-options labels. [Fresh-session initialization stays Unsupported; listing uses the later parent 4.15 close contracts literally. No added options or thread join.]

<!-- from: core-www-close15 r2 (2026-10-24) -->
- **Q20 - HTTP response parsing and worker record/resource transition:** wire 1 specifies header delimiter/status-zero handling but does not qualify status/version token parsing, proxy connect destination selection, or how the admission record `[ID,1,0,1,0,request,0,custom]` transitions into the completion record: lifetime 2 calls word +0x14 the internal resource, whereas admission puts the request there. Please specify the dequeue/response-resource construction and field transition before completion, and malformed status-line behavior. [Portable transaction accepts a dequeued timestamp, parses literal `HTTP/major.minor decimal-status`, returns body/header outcomes, and reuses the specified retry/finalization helpers. Native resource publication and proxy endpoint selection remain unconnected; no invented request-to-resource cast.]

