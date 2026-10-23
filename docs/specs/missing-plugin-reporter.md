# Missing-plugin data reporter

## Scope and evidence

This is the first dedicated contract for the original Windows `cSC4MissingPluginDataReporter`, limited to `DoPostCityInit` (`0x0048C6D0`) and `DoMessage` (`0x0048C8F0`). Export sizes 532 and 33 bytes identify the bodies; their sum is not a coverage or execution claim. Foreign resource, property, allocator, container and dialog operations remain primitives. No native execution or replacement implementation supplied the results below.

## Message decision contract

`DoMessage` receives the message-listener subobject pointer, asks the message at its first argument for its selector through vtable byte offset `+0x10`, and compares the full 32-bit result with `0xEA8AE29A` (`0x0048C8F9–0x0048C901`). On equality it calls post-init using the listener address minus eight (`0x0048C903–0x0048C906`). Other selectors do not call post-init. Both returning paths set **AL = 1** and return while removing the one stack argument (`0x0048C90B–0x0048C90E`); post-init's boolean does not propagate to the message result. This body has no once-only latch, registration mutation or message payload read.

## Post-init decision contract (normal returning paths)

The invocation starts with a true byte result and a fresh temporary unsigned-word set. After the resource traversal it tests that set's count as a full 32-bit value (`0x0048C810`). Empty means skip the popup-permission query and dialog. Nonempty means call `SC4GetPopupModalDialogsEnabled` and test **AL**, not the whole return register (`0x0048C817–0x0048C81E`). A false permission result skips the dialog and retains the initial true result. A true permission result reaches the foreign `cSC4MissingPluginPacksDialog::DoDialog` call with the temporary set (`0x0048C844–0x0048C84B`); its **AL** replaces the invocation's saved byte result (`0x0048C852`). Normal cleanup follows even when this byte is zero. A missing global resource-service pointer at `0x00B43D0C` instead destroys the temporary set and returns **AL = 0** (`0x0048C712–0x0048C73E`). These branches do not establish that every missing-content event displays a dialog or that a dialog decline changes persistent reporter state.

The allocation-null edge explicitly rejoins the dialog call with a null receiver (`0x0048C82A–0x0048C82C`, `0x0048C8DD–0x0048C8DF`). Its successful return is outside the normal nonnull-receiver domain; it is not a proved graceful allocation-failure result.
