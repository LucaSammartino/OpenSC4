-- Catalog IDs from installed DBPF game data. Locations/returns await calibration.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-civics')
fc4.app('ExecuteCheat', {"you don't deserve it"})
fc4.app('ExecuteCheat', {'weaknesspays'})
fc4.city('Demolish', {24, 24, 112, 48, 'None'})
fc4.lot('8A32E4CB', 24, 32, 0) -- wind power / utilities
fc4.lot('03000001', 32, 32, 1) -- water tower / utilities
fc4.lot('04000002', 40, 32, 2) -- recycling / utilities
fc4.lot('6BB942D7', 48, 32, 3) -- police kiosk
fc4.lot('A9DD1E4C', 56, 32, 0) -- fire station
fc4.lot('2A2735A0', 64, 32, 1) -- clinic
fc4.lot('0A272F52', 72, 32, 2) -- library / education
fc4.lot('0A635A90', 80, 32, 3) -- park
fc4.lot('4A57BF3A', 88, 32, 0) -- Big Ben / landmark
fc4.lot('2A53CC9B', 96, 32, 1) -- mayor statue / reward
fc4.lot('05000001', 104, 32, 2) -- bus stop / transit
fc4.city('PlaceNetwork', {0, 24, 40, 112, 40})
fc4.finish()
