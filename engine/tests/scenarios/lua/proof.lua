-- Small Lua-entry/city-command probe; not a suite scenario.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('proof')
local name = fc4.city('GetCityName')
fc4.city('SetCityName', {'FC4 Lua proof'})
local changed = fc4.city('GetCityName')
if changed ~= 'FC4 Lua proof' then error('city name did not change') end
fc4.log('city-name-effect verified')
fc4.city('SetCityName', {name})
fc4.city('SaveCityQuick', {true})
fc4.log('proof-complete')
