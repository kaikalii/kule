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

local function table_eq(a, b)
    if type(a) == "table" then
        for a_key, a_val in pairs(a) do
            if not table_eq(a_val, b[a_key]) then return false end
        end
        return true
    else
        return a == b
    end
end

local function has_value(tab, val)
    for _, value in pairs(tab) do
        if table_eq(value, val) then return true end
    end
    return false
end

local core = {rot = 1.0, tracker = {}}

function core:print_tracker() tprint(self.tracker, 0) end
function core:fps() return self.tracker.fps end
-- function core:event(event)
--     tprint(event)
--     print()
-- end
function core:update(dt)
    if has_value(self.tracker.keys, "R") then self.rot = self.rot + dt end
end

return core
