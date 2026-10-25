dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-demolish')
local modes = {'Sim', 'None', 'Explode'}
local i = 1
while modes[i] do
    local z = 64 + i * 16
    fc4.city('Demolish', {48, z, 76, z + 10, 'None'})
    fc4.city('PlaceZone', {1, 48, z, 51, z + 3})
    fc4.city('PlaceNetwork', {0, 54, z, 62, z})
    fc4.lot('6BB942D7', 66, z, 0)
    fc4.wait_date(2026, 3 + i * 2, 1)
    fc4.city('Demolish', {48, z, 76, z + 10, modes[i]})
    i = i + 1
end
fc4.finish()
