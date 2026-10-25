-- Berlin's large empty tile; only its isolated slot copy is changed.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/lot-sweep.lua')
fc4.lot_sweep('lua-lots-empty', true)
