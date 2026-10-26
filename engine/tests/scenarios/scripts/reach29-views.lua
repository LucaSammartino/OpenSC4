-- Use actual command statuses; setup and actions are confined to the slot.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('reach29-views')
local zoom = 0
while zoom <= 5 do
    fc4.city('Zoom' .. zoom)
    fc4.delay(12)
    zoom = zoom + 1
end
fc4.city('ScrollRightOnce')
fc4.city('ScrollDownOnce')
fc4.delay(24)
fc4.delay(24)
fc4.city('ScrollLeftOnce')
fc4.city('ScrollUpOnce')
fc4.city('Zoom3')
fc4.city('SaveCityQuick', {true})
fc4.log('scenario-complete')
