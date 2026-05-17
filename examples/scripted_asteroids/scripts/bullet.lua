local screen_w = params.screen_w or 1280.0
local screen_h = params.screen_h or 720.0
local speed = params.speed or 520.0
local lifetime = params.lifetime or 1.35

local vx = 0.0
local vy = 0.0
local initialized = false

function on_start(self)
local transform = self:transform()
if transform == nil then
    return
    end

    local rotation = transform:rotation()
    vx = math.cos(rotation) * speed
    vy = math.sin(rotation) * speed

    initialized = true
    end

    function on_update(self, dt)
    if not initialized then
        on_start(self)
        end

        lifetime = lifetime - dt

        local transform = self:transform()
        if transform == nil then
            return
            end

            local pos = transform:position()
            local x = pos.x + vx * dt
            local y = pos.y + vy * dt

            if x < 0.0 then x = screen_w end
                if x > screen_w then x = 0.0 end
                    if y < 0.0 then y = screen_h end
                        if y > screen_h then y = 0.0 end

                            transform:set_position(vec2(x, y))

                            if lifetime <= 0.0 then
                                self:world():despawn(self:entity())
                                end
                                end
