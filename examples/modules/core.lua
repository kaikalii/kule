local function tprint(tbl, indent)
    if not indent then indent = 0 end
    for k, v in pairs(tbl) do
        local formatting = string.rep("  ", indent) .. tostring(k) .. ": "
        if type(v) == "table" then
            print(formatting)
            tprint(v, indent + 1)
        else
            print(formatting .. tostring(v))
        end
    end
end

local core = {tracker = {}}

function core:print_tracker() tprint(self.tracker, 0) end
function core:fps() return self.tracker.fps end
function core:event(event)
    tprint(event)
    print()
end

return core
