-- Strings from IGN's public PC Cheats page; see docs/ops/lua-commands.md.
dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')
fc4.start('lua-cheats')
local cheats = {'weaknesspays', 'fightthepower', 'howdryiam', "you don't deserve it",
                'zoneria', 'tastyzots', 'dollyllama', 'gol', 'sizeof 2', 'whatimeizit 12',
                'hellomynameis FC4Lua', 'whererufrom Konradshohe'}
local i = 1
while cheats[i] do
    fc4.app('ExecuteCheat', {cheats[i]})
    fc4.delay(2)
    i = i + 1
end
fc4.finish()
