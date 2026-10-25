-- Each lot's return is preserved, including failures; do not stop at the first.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
local lots = dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/lot-sweep-catalog.lua')

function fc4.lot_sweep(name, flatten)
    fc4.start(name)
    fc4.try_city('SetCityEstablished')
    fc4.city('SimSpeedPause')
    local width, depth = fc4.city('GetCitySize')
    if width ~= 256 or depth ~= 256 then error('lot sweep requires a large 256-cell tile') end
    if flatten then
        fc4.try_city('SetCellAltitude', {0, 0, 255, 255, 280})
        fc4.try_city('GetCellAltitude', {32, 32})
        -- Single-cell read-back distinguishes command acceptance from terrain effect.
        fc4.try_city('SetCellAltitude', {32, 32, 280})
        fc4.try_city('GetCellAltitude', {32, 32})
    end
    fc4.call('app', 'ExecuteCheat', {"you don't deserve it"}, true)
    fc4.call('app', 'ExecuteCheat', {'YouDontDeserveIt'}, true)
    local i = 1
    while lots[i] do
        local lot = lots[i]
        local x, z, w, d = lot[2], lot[3], lot[4], lot[5]
        fc4.log('lot-begin iid=' .. lot[1] .. ' category=' .. lot[6] .. ' name=' .. lot[7])
        -- Clear disjoint footprints on the grown city, retain the rest of it.
        fc4.try_city('Demolish', {x, z, x + w - 1, z + d - 1, 'None'})
        local status = fc4.try_city('PlaceLot', {fc4.iid(lot[1]), x, z, 0, false, false})
        fc4.log('lot-result iid=' .. lot[1] .. ' category=' .. lot[6] .. ' status=' .. fc4.text(status))
        i = i + 1
    end
    -- Landfill is a zone, not a catalog ploppable lot.
    fc4.try_city('PlaceZone', {14, 220, 220, 231, 231})
    fc4.city('SaveCityQuick', {true})
    fc4.log('placement-complete')
    fc4.city('SimSpeedFast')
    fc4.delay(300)
    fc4.log('settled-frames 300')
    -- Demolish part of the placed catalog with all three documented flags.
    i = 1
    local flags = {'None', 'Sim', 'Explode'}
    while i <= 3 do
        local lot = lots[i]
        local x, z, w, d = lot[2], lot[3], lot[4], lot[5]
        fc4.log('demolish-pass iid=' .. lot[1] .. ' flag=' .. flags[i])
        fc4.try_city('Demolish', {x, z, x + w - 1, z + d - 1, flags[i]})
        i = i + 1
    end
    fc4.city('SaveCityQuick', {true})
    fc4.log('scenario-complete')
end
