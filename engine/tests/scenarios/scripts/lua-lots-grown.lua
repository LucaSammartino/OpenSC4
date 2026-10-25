-- Medium grown Maxisland city; clear only the individual planned lot footprints.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/lot-sweep.lua')
fc4.lot_sweep('lua-lots-grown', false)
