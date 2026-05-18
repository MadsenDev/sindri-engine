local time = 0.0

function on_update(self, dt)
    time = time + dt

    local transform = self:transform()
    if transform ~= nil then
        transform:set_rotation(time * 0.65)
        local pulse = 1.0 + math.sin(time * 2.5) * 0.08
        transform:set_scale(vec2(pulse, pulse))
    end

    local sprite = self:sprite()
    if sprite ~= nil then
        local glow = 0.72 + math.sin(time * 3.0) * 0.2
        sprite:set_tint({1.0, glow, 0.22, 1.0})
    end
end
