dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-networks')
fc4.city('Demolish', {24, 24, 112, 104, 'None'})
local kind = 0
while kind <= 12 do
    fc4.city('PlaceNetwork', {kind, 24, 24 + kind * 6, 104, 24 + kind * 6})
    kind = kind + 1
end
fc4.lot('05000001', 40, 26, 0) -- bus stop
fc4.lot('05000002', 48, 68, 1) -- subway
fc4.lot('4BAFBA9F', 56, 74, 2) -- elevated rail station
fc4.lot('6A3855D4', 64, 32, 3) -- freight rail
fc4.lot('6A8A358F', 76, 32, 0) -- passenger rail
local tools = {'RoadTool', 'StreetTool', 'AvenueTool', 'HighwayTool', 'RailTool',
               'LightRailTool', 'SubwayTool', 'PowerLinesTool', 'PipesTool',
               'PlaceBusStopTool', 'PlaceSubwayStationTool', 'PlaceLightRailStationTool',
               'PlaceFerryTerminalTool', 'PlacePassengerDepotTool', 'PlaceFreightDepotTool'}
local i = 1
while tools[i] do
    fc4.city(tools[i])
    fc4.city('Cancel')
    i = i + 1
end
fc4.finish()
