-- Candidate scenario; only the isolated game-slot copy may change.
-- Requires the opt-in lua_script harness key. No command is calibrated here.
sc4app.SetScriptAutoYield(-1, true)
while sc4app.GetAppState() ~= 2 do
    sc4app.GameDelay('frames', 1)
end
sc4app.SetPopupModalDialogsEnabled(false)
-- Preflight: identify occupied civic, utility and transport lots.
sc4app.QueryTool()
sc4app.GameDelay('frames', 120)
-- A calibrated map click must occur here; selection alone is not a query.
sc4app.Cancel()
sc4app.TrafficQueryTool()
sc4app.GameDelay('frames', 120)
sc4app.Cancel()
sc4app.GameDelay('frames', 60)
