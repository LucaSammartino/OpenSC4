-- Candidate calibration; no coordinate/return convention is yet verified.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-calibrate')
local width, depth = fc4.city('GetCitySize')
fc4.app('GetDateAndTime', {'date', 'time'})
fc4.app('GetRandomInteger', {1, 10})
fc4.city('GetSeaLevel')
local altitude = fc4.city('GetCellAltitude', {8, 8})
if type(altitude) == 'number' then
    fc4.city('SetCellAltitude', {8, 8, altitude + 1})
    fc4.city('GetCellAltitude', {8, 8})
    fc4.city('SetCellAltitude', {8, 8, altitude})
end
if type(width) == 'number' and type(depth) == 'number' and width > 100 and depth > 100 then
    -- Small, disjoint rectangles; compare these against the month DIGEST/save.
    fc4.city('PlaceZone', {1, 32, 32, 35, 35})
    fc4.city('PlaceNetwork', {0, 32, 38, 40, 38})
    fc4.lot('03000001', 40, 40, 0)
    fc4.city('PlaceLot', {'0x03000001', 44, 40, 1})
    fc4.city('PlaceBuilding', {fc4.iid('03110000'), 80, 80})
    fc4.city('PlaceBuilding', {'0x03110000', 96, 80})
end
local x, y, z = fc4.app('GetViewTarget')
if type(x) == 'number' and type(y) == 'number' and type(z) == 'number' then
    fc4.app('SetViewTarget', {x, y, z})
end
fc4.app('TakeSnapshot', {FC4_ROOT .. '/lua-calibrate.png'})
fc4.finish()
