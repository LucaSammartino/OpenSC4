-- Each lot's return is preserved, including failures; do not stop at the first.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')

function fc4.lot_sweep(name, flatten, qualified)
    fc4.start(name)
    fc4.city('SimSpeedPause')
    local width, depth = fc4.city('GetCitySize')
    if width ~= depth or (width ~= 128 and width ~= 256) then error('lot sweep requires 128 or 256 cells') end
    if flatten and width ~= 256 then error('empty lot sweep requires a large tile') end
    local suffix = ''
    if width == 128 then suffix = '-128' end
    local lots = dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/lot-sweep-catalog' .. suffix .. '.lua')
    if qualified then
        if flatten or width ~= 128 then error('qualified sites require West Weburg') end
        local sites = dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/lot-sweep-grown-sites.lua')
        local j = 1
        while sites[j] do
            local k = 1
            while lots[k] do
                if lots[k][1] == sites[j][1] then
                    lots[k][2], lots[k][3] = sites[j][2], sites[j][3]
                end
                k = k + 1
            end
            j = j + 1
        end
    end
    fc4.try_city('SetCityEstablished')
    if flatten then
        -- The companion dated plugin script prepares terrain. Check its effect
        -- before plopping: Maxis SetCellAltitude accepted calls without changing it.
        fc4.city('SimSpeedFast')
        local altitude = fc4.city('GetCellAltitude', {32, 32})
        local polls = 0
        while altitude ~= 280 and polls < 120 do
            fc4.delay(1)
            altitude = fc4.city('GetCellAltitude', {32, 32})
            polls = polls + 1
        end
        if altitude ~= 280 then error('flat terrain companion did not run') end
        if fc4.city('GetCellAltitude', {8, 8}) ~= 280 or
           fc4.city('GetCellAltitude', {240, 240}) ~= 280 then error('flat terrain read-back failed') end
        fc4.log('flat-terrain verified 280m')
        fc4.city('SimSpeedPause')
    end
    local i = 1
    local batch = 0
    while lots[i] do
        local lot = lots[i]
        local x, z, w, d = lot[2], lot[3], lot[4], lot[5]
        fc4.log('lot-begin iid=' .. lot[1] .. ' category=' .. lot[6] .. ' name=' .. lot[7])
        -- Clear disjoint footprints on the grown city, retain the rest of it.
        local pad = 0
        if qualified then pad = 2 end
        local x1, z1, x2, z2 = x - pad, z - pad, x + w - 1 + pad, z + d - 1 + pad
        if x1 < 0 then x1 = 0 end
        if z1 < 0 then z1 = 0 end
        if x2 >= width then x2 = width - 1 end
        if z2 >= depth then z2 = depth - 1 end
        fc4.try_city('Demolish', {x1, z1, x2, z2, 'None'})
        local status = fc4.try_city('PlaceLot', {fc4.iid(lot[1]), x, z, 0, false, false})
        fc4.log('lot-result iid=' .. lot[1] .. ' category=' .. lot[6] .. ' status=' .. fc4.text(status))
        batch = batch + 1
        if batch == 8 then
            -- GameDelay(frames) stalled in paused West Weburg after exactly
            -- three batches. Advance a running frame, then pause construction.
            if not flatten then fc4.city('SimSpeedFast') end
            fc4.delay(1)
            if not flatten then fc4.city('SimSpeedPause') end
            batch = 0
        end
        i = i + 1
    end
    -- Landfill is a zone, not a catalog ploppable lot.
    fc4.try_city('PlaceZone', {14, width - 6, depth - 6, width - 2, depth - 2})
    fc4.city('SaveCityQuick', {true})
    fc4.log('placement-complete')
    if not flatten then fc4.city('SimSpeedFast') end
    fc4.delay(10)
    if not flatten then fc4.city('SimSpeedPause') end
    fc4.log('settled-frames 10')
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
    fc4.city('SimSpeedFast')
end
