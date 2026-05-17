local screen_w = params.screen_w or 1280.0
local screen_h = params.screen_h or 720.0

local velocity = params.velocity or vec2(60.0, 20.0)
local rotation_speed = params.rotation_speed or 0.5

function on_start(self)
local sprite = self:sprite()
if sprite ~= nil then
    sprite:set_tint({0.65, 0.68, 0.72, 1.0})
    end
    end

    function on_update(self, dt)
    local transform = self:transform()
    if transform == nil then
        return
        end

        local pos = transform:position()
        local x = pos.x + velocity.x * dt
        local y = pos.y + velocity.y * dt

        if x < 0.0 then x = screen_w end
            if x > screen_w then x = 0.0 end
                if y < 0.0 then y = screen_h end
                    if y > screen_h then y = 0.0 end

                        local rotation = transform:rotation() + rotation_speed * dt

                        transform:set_position(vec2(x, y))
                        transform:set_rotation(rotation)
                        end
