-- Establish only the isolated slot city; its companion dated script owns actions.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('mayor-setup')
fc4.city('SetCityEstablished')
fc4.city('SimSpeedFast')
fc4.log('scenario-complete')
