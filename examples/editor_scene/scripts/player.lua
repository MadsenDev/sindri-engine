local speed = 220.0
local jump_speed = 420.0
local grounded_y = nil
local was_grounded = false

function on_update(self, dt)
  local input = self:input()
  local transform = self:transform()
  local physics = self:physics()
  if input == nil or transform == nil or physics == nil then
    return
  end

  local pos = transform:position()
  if grounded_y == nil then
    grounded_y = pos.y
  end

  local velocity = physics:velocity()
  local horizontal = input:axis("A", "D")
  local grounded = pos.y >= grounded_y - 2.0 and velocity.y >= -1.0

  physics:set_velocity(vec2(horizontal * speed, velocity.y))

  if grounded and input:is_key_pressed("Space") then
    physics:set_velocity(vec2(horizontal * speed, -jump_speed))
    was_grounded = false
  else
    was_grounded = grounded
  end
end
