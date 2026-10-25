dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-terrain')
local altitude = fc4.city('GetCellAltitude', {12, 12})
local sea = fc4.city('GetSeaLevel')
if type(altitude) == 'number' then
    fc4.city('SetCellAltitude', {12, 12, 15, 15, altitude + 5})
    fc4.city('GetCellAltitude', {12, 12})
    fc4.city('SetCellAltitude', {12, 12, 15, 15, altitude})
end
if type(sea) == 'number' then
    fc4.city('SetSeaLevel', {sea + 1})
    fc4.city('GetSeaLevel')
    fc4.wait_date(2026, 6, 1)
    fc4.city('SetSeaLevel', {sea})
end
fc4.finish()
