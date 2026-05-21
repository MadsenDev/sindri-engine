speed = 200
jump_speed = 500

function on_update(self, dt)
    local input = self:input()
    local transform = self:transform()
    local physics = self:physics()
    if input == nil or transform == nil or physics == nil then return end

    local anim = self:animated_sprite()

    local pos = transform:position()
    if grounded_y == nil then grounded_y = pos.y end

    local velocity = physics:velocity()
    local horizontal = input:axis("A", "D")
    local grounded = pos.y >= grounded_y - 2.0 and velocity.y >= -1.0

    physics:set_velocity(vec2(horizontal * speed, velocity.y))

    if grounded and input:is_key_pressed("Space") then
        physics:set_velocity(vec2(horizontal * speed, -jump_speed))
    end

    if anim ~= nil then
        if horizontal > 0.1 then
            anim:play("walk_right")
        elseif horizontal < -0.1 then
            anim:play("walk_left")
        else
            anim:play("idle")
        end
    end
end
