local function init(self)
  self.blinkTimer = 0
  self.brightness = 0.5
end

local function update(self, dt)
  self.blinkTimer = (self.blinkTimer + dt) % 2
  local sprite = self.entity.components.Sprite
  if sprite then
    sprite.color = {
      1.0,
      0.5950231 + math.sin(self.blinkTimer * 3) * 0.2,
      0.22,
      1.0
    }
  end
end

return { init = init, update = update }