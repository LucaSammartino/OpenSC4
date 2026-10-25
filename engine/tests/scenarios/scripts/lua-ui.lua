-- Closing via Cancel and the button ID below are probes, not verified UI IDs.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-ui')
local dialogs = {'OpenAudioOptionsDialog', 'OpenCheatCodeDialog', 'OpenGraphicOptionsDialog',
                 'OpenPlayOptionsDialog', 'OpenPhotoAlbumDialog', 'OpenSnapshotDialog',
                 'OpenObliterateCityDialog'}
local i = 1
while dialogs[i] do
    fc4.city(dialogs[i])
    fc4.delay(2)
    fc4.app('TakeSnapshot', {FC4_ROOT .. '/' .. dialogs[i] .. '.png'})
    fc4.city('Cancel') -- must be calibrated before treating the next dialog as reached
    i = i + 1
end
local toggles = {'ToggleGodPanel', 'ToggleMayorPanel', 'ToggleMySimPanel', 'ToggleOptionsPanel',
                 'ToggleGridVisibility', 'ToggleUIVisibility'}
i = 1
while toggles[i] do
    fc4.city(toggles[i])
    fc4.delay(2)
    fc4.city(toggles[i])
    i = i + 1
end
fc4.city('QueryTool')
fc4.city('Cancel')
fc4.city('TrafficQueryTool')
fc4.city('Cancel')
fc4.app('GZWinIsWindowPresent', {0, 0})
fc4.app('GZWinClickButton', {1, 0}) -- negative/ID-format probe; no known button claimed
fc4.app('GZWinEnableUserInput', {true})
i = 0
while i <= 5 do
    fc4.city('Zoom' .. i)
    i = i + 1
end
fc4.city('ScrollRightOnce')
fc4.city('ScrollLeftOnce')
fc4.city('ScrollUpOnce')
fc4.city('ScrollDownOnce')
fc4.city('EnableDayOnly')
fc4.city('EnableNightOnly')
fc4.city('EnableDayAndNight')
fc4.finish()
