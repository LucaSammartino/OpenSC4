-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
-- Before this driver, open the compact budget panel in the slot copy.
-- Refuse an absent panel rather than treating a missing button as an action.
local button = 2855977987
local present = sc4app.GZWinIsWindowPresent(button, 0)
if present ~= true and present ~= 1 then
    error('Budget panel precondition is unmet')
end
sc4app.GZWinClickButton(button, 0)
sc4app.GameDelay('frames', 30)
sc4app.Cancel()
-- Allocation changes, ordinances, deals and modal closing need calibration.
sc4app.GameDelay('frames', 60)
