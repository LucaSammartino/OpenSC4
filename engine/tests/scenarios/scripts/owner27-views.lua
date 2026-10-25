-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
sc4app.Zoom0()
sc4app.GameDelay('frames', 30)
sc4app.Zoom1()
sc4app.GameDelay('frames', 30)
sc4app.Zoom2()
sc4app.GameDelay('frames', 30)
sc4app.Zoom3()
sc4app.GameDelay('frames', 30)
sc4app.Zoom4()
sc4app.GameDelay('frames', 30)
sc4app.Zoom5()
sc4app.ScrollRightOnce()
sc4app.ScrollDownOnce()
sc4app.EnableNightOnly()
sc4app.GameDelay('frames', 120)
sc4app.EnableDayOnly()
sc4app.ScrollLeftOnce()
sc4app.ScrollUpOnce()
sc4app.EnableDayAndNight()
-- Data-map selection, legend controls and rotation require input calibration.
sc4app.GameDelay('frames', 60)
