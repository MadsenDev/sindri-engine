local screen_w = params.screen_w or 1280.0
local screen_h = params.screen_h or 720.0

local rotation_speed = params.rotation_speed or 5.0
local thrust = params.thrust or 260.0
local friction = params.friction or 0.985
local max_speed = params.max_speed or 340.0

local vx = 0.0
local vy = 0.0
local rotation = 0.0

function on_start(self)
print("[player.lua] started")

local sprite = self:sprite()
if sprite ~= nil then
    sprite:set_tint({0.9, 0.95, 1.0, 1.0})
    end
    end

    function on_update(self, dt)
    local input = self:input()
    local transform = self:transform()

    if transform == nil then
        return
        end

        if input:is_key_down("A") or input:is_key_down("Left") then
            rotation = rotation - rotation_speed * dt
            end

            if input:is_key_down("D") or input:is_key_down("Right") then
                rotation = rotation + rotation_speed * dt
                end

                if input:is_key_down("W") or input:is_key_down("Up") then
                    vx = vx + math.cos(rotation) * thrust * dt
                    vy = vy + math.sin(rotation) * thrust * dt
                    end

                    vx = vx * friction
                    vy = vy * friction

                    local speed = math.sqrt(vx * vx + vy * vy)
                    if speed > max_speed then
                        vx = (vx / speed) * max_speed
                        vy = (vy / speed) * max_speed
                        end

                        local pos = transform:position()
                        local x = pos.x + vx * dt
                        local y = pos.y + vy * dt

                        if x < 0.0 then x = screen_w end
                            if x > screen_w then x = 0.0 end
                                if y < 0.0 then y = screen_h end
                                    if y > screen_h then y = 0.0 end

                                        transform:set_position(vec2(x, y))
                                        transform:set_rotation(rotation)
                                        end
