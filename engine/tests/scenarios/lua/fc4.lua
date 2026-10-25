-- Maxis test-script helpers. Interface facts: 0xC0000054's public command docs.
-- These candidates still require calibration on 1.1.641; see docs/ops/lua-commands.md.
-- The launcher sets FC4_ROOT to the private slot's LuaScripts directory.
fc4 = { sequence = 0, scenario = 'startup', log_backend = 'markers' }
fc4.unpack = unpack or (table and table.unpack)
fc4.gsub = gsub or (string and string.gsub)
fc4.sub = strsub or (string and string.sub)

function fc4.text(value)
    if value == nil then return 'nil' end
    return tostring(value)
end

function fc4.log(message)
    fc4.sequence = fc4.sequence + 1
    local line = fc4.text(fc4.sequence) .. ' ' .. fc4.scenario .. ' ' .. message
    if io and io.open then
        local file = io.open(FC4_ROOT .. '/fc4-lua.log', 'a')
        if file then
            file:write(line .. '\n')
            file:close()
            fc4.log_backend = 'io.open'
            return
        end
    elseif openfile and write and closefile then
        local file = openfile(FC4_ROOT .. '/fc4-lua.log', 'a')
        if file then
            write(file, line .. '\n')
            closefile(file)
            fc4.log_backend = 'openfile'
            return
        end
    end
    -- Retail Lua may have no file I/O or print. The documented CreateGZLog
    -- command creates an empty file; the filename preserves this bounded line.
    if sc4app and sc4app.CreateGZLog and fc4.gsub and fc4.sub then
        local safe = fc4.gsub(line, '[^%w_.-]', '_')
        sc4app.CreateGZLog(FC4_ROOT .. '/' .. fc4.sub(safe, 1, 150) .. '.marker', '', false)
    end
end

function fc4.delay(frames)
    sc4app.GameDelay('frames', frames or 1)
end

function fc4.require_state(expected)
    local actual = sc4app.GetAppState()
    if actual ~= expected then
        fc4.log('wrong-state expected=' .. expected .. ' actual=' .. fc4.text(actual))
        return false
    end
    return true
end

function fc4.call(scope, command, args)
    if scope == 'city' and not fc4.require_state(2) then return nil end
    if scope == 'region' and not fc4.require_state(1) then return nil end
    if not sc4app[command] then
        fc4.log('unavailable ' .. command)
        return nil
    end
    args = args or {}
    local detail = ''
    local index = 1
    while args[index] ~= nil do
        detail = detail .. ' ' .. fc4.text(args[index])
        index = index + 1
    end
    -- A Before without a corresponding Return is an incomplete command, not
    -- proof of a crash or a failure. Inspect the game log and process verdict.
    fc4.log('Before ' .. command .. detail)
    local a, b, c, d, e = sc4app[command](fc4.unpack(args))
    fc4.log('Return ' .. command .. ' ' .. fc4.text(a) .. ' ' .. fc4.text(b) .. ' ' ..
            fc4.text(c) .. ' ' .. fc4.text(d) .. ' ' .. fc4.text(e))
    return a, b, c, d, e
end

function fc4.app(command, args) return fc4.call('app', command, args) end
function fc4.city(command, args) return fc4.call('city', command, args) end
function fc4.region(command, args) return fc4.call('region', command, args) end

function fc4.wait_state(expected, max_polls)
    fc4.log('wait-state ' .. expected)
    local poll = 0
    while sc4app.GetAppState() ~= expected do
        if poll >= (max_polls or 12000) then
            fc4.log('timeout wait-state ' .. expected)
            error('fc4: application state wait timed out')
        end
        fc4.delay(1)
        poll = poll + 1
    end
    fc4.log('reached-state ' .. expected)
end

function fc4.start(name)
    fc4.scenario = name
    fc4.log('startup backend=' .. fc4.log_backend)
    if sc4app.GetAppState() ~= 2 then
        -- 0xCBB5BB45, written in decimal for the retail Lua number parser.
        local message = 3417684805
        local token = 'fc4-' .. name .. '-post-region'
        local state = sc4app.WaitForMessage2(message, token)
        sc4app.SetScriptAutoYield(-1, true)
        local polls = 0
        while state == 1 do
            polls = polls + 1
            if polls > 12000 then
                sc4app.CancelWaitForMessage2(message, token)
                error('fc4: post-region wait timed out')
            end
            state = sc4app.WaitForMessage2(message, token)
        end
        sc4app.CancelWaitForMessage2(message, token)
    end
    sc4app.SetScriptAutoYield(-1, true)
    fc4.wait_state(2)
    fc4.app('SetPopupModalDialogsEnabled', {false})
    fc4.city('SimSpeedFast')
    fc4.city('GetCityName')
    fc4.city('GetCitySize')
    fc4.city('GetSimulationDate', {'year', 'month', 'day of month'})
end

function fc4.wait_date(year, month, day, max_polls)
    fc4.log('wait-date ' .. year .. '-' .. month .. '-' .. day)
    local target = year * 10000 + month * 100 + day
    local polls = 0
    while fc4.require_state(2) do
        local y, m, d = sc4app.GetSimulationDate('year', 'month', 'day of month')
        if type(y) ~= 'number' or type(m) ~= 'number' or type(d) ~= 'number' then
            fc4.log('invalid-date-return ' .. fc4.text(y) .. ' ' .. fc4.text(m) .. ' ' .. fc4.text(d))
            error('fc4: GetSimulationDate did not return three numbers')
        end
        if y * 10000 + m * 100 + d >= target then
            fc4.log('reached-date ' .. y .. '-' .. m .. '-' .. d)
            return true
        end
        if polls >= (max_polls or 200000) then
            fc4.log('timeout wait-date')
            error('fc4: date wait timed out')
        end
        fc4.delay(1)
        polls = polls + 1
    end
    error('fc4: left city while waiting for date')
end

function fc4.iid(hex)
    -- Pass a number, not a Lua hexadecimal literal or an uncalibrated ID string.
    return tonumber(hex, 16)
end

function fc4.lot(hex, x, z, facing)
    return fc4.city('PlaceLot', {fc4.iid(hex), x, z, facing or 0, false, false})
end

function fc4.finish()
    fc4.wait_date(2028, 1, 1)
    fc4.city('SaveCityQuick', {true})
    fc4.log('scenario-complete')
end
