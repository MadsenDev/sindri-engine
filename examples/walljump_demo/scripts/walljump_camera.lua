local function param(name, fallback)
    if params == nil then
        return fallback
    end
    local value = params[name]
    if value == nil then
        return fallback
    end
    return value
end

function on_start(ctx)
    local cam = ctx:camera()
    if cam ~= nil then
        cam:set_zoom(param("zoom", 1.1))
        cam:set_offset(vec2(0.0, 0.0))
    end
end

function on_update(ctx, dt)
    local dead_x = param("dead_zone_x", 36.0)
    local dead_y = param("dead_zone_y", 18.0)
    local world = ctx:world()
    if world == nil then
        return
    end

    local player_id = world:find_by_tag("player")
    if player_id == nil then
        return
    end

    local player_pos = world:position(player_id)
    local transform = ctx:transform()
    if transform == nil then
        return
    end

    local cam_pos = transform:position()
    local delta = vec2(player_pos.x - cam_pos.x, player_pos.y - cam_pos.y)

    local desired = cam_pos
    if math.abs(delta.x) > dead_x then
        desired.x = player_pos.x - math.sign(delta.x) * dead_x
    end
    if math.abs(delta.y) > dead_y then
        desired.y = player_pos.y - math.sign(delta.y) * dead_y
    end

    local smooth = param("smooth", 0.12)
    local t = math.min(1.0, smooth * 60.0 * dt)
    local new_pos = vec2(
        cam_pos.x + (desired.x - cam_pos.x) * t,
        cam_pos.y + (desired.y - cam_pos.y) * t
    )
    transform:set_position(new_pos)
end

function math.sign(value)
    if value > 0 then
        return 1.0
    elseif value < 0 then
        return -1.0
    end
    return 0.0
end
