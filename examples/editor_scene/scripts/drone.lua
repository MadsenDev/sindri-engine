local angle = 0.0
local radius = 150.0

function on_update(self, dt)
    angle = angle + dt * 1.25

    local transform = self:transform()
    if transform == nil then
        return
    end

    local x = 640.0 + math.cos(angle) * radius
    local y = 360.0 + math.sin(angle) * 70.0
    transform:set_position(vec2(x, y))
    transform:set_rotation(angle * 2.0)

    local sprite = self:sprite()
    if sprite ~= nil then
        sprite:set_tint({0.34, 0.54 + math.sin(angle) * 0.18, 1.0, 1.0})
    end
end
