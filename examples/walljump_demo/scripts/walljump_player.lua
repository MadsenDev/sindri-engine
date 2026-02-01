local move_dir = 0.0
local jump_pressed = false
local jump_buffer = 0.0
local coyote_timer = 0.0
local wall_jump_lock = 0.0
local wall_stamina = 1.0
local wall_slide_timer = 0.0
local prev_pos = vec2(0.0, 0.0)
local was_grounded = false
local last_fall_speed = 0.0
local one_way_ground_timer = 0.0
local drop_through_timer = 0.0
local jump_hold_timer = 0.0
local jump_hold_active = false
local hang_timer = 0.0
local jump_grace_timer = 0.0
local last_jump_wall = false

local ground_contacts = 0
local wall_left_contacts = 0
local wall_right_contacts = 0
local contact_types = {}

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

local function classify_contact(ctx, other_id)
    local world = ctx:world()
    local player_pos = world:position(ctx:entity())
    local other_pos = world:position(other_id)
    local size = world:collider_size(other_id)
    if size.x == 0 and size.y == 0 then
        return nil
    end

    local half_x = size.x * 0.5
    local half_y = size.y * 0.5
    local dx = player_pos.x - other_pos.x
    local dy = player_pos.y - other_pos.y

    if dy < -half_y * 0.6 then
        return "ground"
    elseif dx > half_x * 0.6 then
        return "wall_left"
    elseif dx < -half_x * 0.6 then
        return "wall_right"
    end
    return nil
end

local function add_contact(kind)
    if kind == "ground" then
        ground_contacts = ground_contacts + 1
    elseif kind == "wall_left" then
        wall_left_contacts = wall_left_contacts + 1
    elseif kind == "wall_right" then
        wall_right_contacts = wall_right_contacts + 1
    end
end

local function remove_contact(kind)
    if kind == "ground" then
        ground_contacts = math.max(0, ground_contacts - 1)
    elseif kind == "wall_left" then
        wall_left_contacts = math.max(0, wall_left_contacts - 1)
    elseif kind == "wall_right" then
        wall_right_contacts = math.max(0, wall_right_contacts - 1)
    end
end

function on_start(ctx)
    local world = ctx:world()
    if world ~= nil then
        prev_pos = world:position(ctx:entity())
    end
    wall_stamina = param("wall_stamina_max", 1.0)
end

function on_collision_enter(ctx, other_id)
    local world = ctx:world()
    if world ~= nil then
        local tag = world:tag_of(other_id)
        if tag == "checkpoint" then
            local size = world:collider_size(other_id)
            if size.x > 0 or size.y > 0 then
                local other_pos = world:position(other_id)
                local top = other_pos.y - size.y * 0.5
                local half_height = param("half_height", 28.0)
                set_respawn_point(other_pos.x, top - half_height)
            end
        end
    end

    local kind = classify_contact(ctx, other_id)
    if kind ~= nil then
        contact_types[other_id] = kind
        add_contact(kind)
    end
end

function on_collision_exit(ctx, other_id)
    local kind = contact_types[other_id]
    if kind ~= nil then
        contact_types[other_id] = nil
        remove_contact(kind)
    end
end

function on_trigger_enter(ctx, other_id)
    on_collision_enter(ctx, other_id)
end

function on_trigger_exit(ctx, other_id)
    on_collision_exit(ctx, other_id)
end

function on_update(ctx, dt)
    local input = ctx:input()
    move_dir = 0.0
    if input:is_key_down("A") or input:is_key_down("Left") then
        move_dir = move_dir - 1.0
    end
    if input:is_key_down("D") or input:is_key_down("Right") then
        move_dir = move_dir + 1.0
    end

    if input:is_key_pressed("Space") or input:is_key_pressed("W") then
        jump_pressed = true
        jump_buffer = param("jump_buffer", 0.15)
    end

    if input:is_key_released("Space") or input:is_key_released("W") then
        jump_hold_active = false
    end

    if (input:is_key_down("S") or input:is_key_down("Down"))
        and (input:is_key_pressed("Space") or input:is_key_pressed("W"))
    then
        drop_through_timer = param("drop_through_time", 0.18)
        jump_pressed = false
        jump_buffer = 0.0
    end
end

function on_fixed_update(ctx, dt)
    local physics = ctx:physics()
    if physics == nil then
        return
    end

    local move_speed = param("move_speed", 260.0)
    local jump_impulse = param("jump_impulse", 360.0)
    local wall_jump_impulse = param("wall_jump_impulse", jump_impulse)
    local jump_cut = param("jump_cut", 0.45)
    local jump_max_hold = param("jump_max_hold", 0.16)
    local jump_hold_boost = param("jump_hold_boost", 0.45)
    local wall_jump_hold_boost = param("wall_jump_hold_boost", 0.35)
    local hang_time = param("hang_time", 0.06)
    local hang_speed = param("hang_speed", 26.0)
    local fall_speed = param("fall_speed", 1200.0)
    local wall_jump_x = param("wall_jump_x", 320.0)
    local wall_slide_speed = param("wall_slide_speed", 120.0)
    local wall_slide_interval = param("wall_slide_interval", 0.06)
    local wall_stamina_max = param("wall_stamina_max", 1.0)
    local coyote_time = param("coyote_time", 0.12)
    local half_height = param("half_height", 28.0)
    local respawn_y = param("respawn_y", 740.0)

    if one_way_ground_timer > 0.0 then
        one_way_ground_timer = math.max(0.0, one_way_ground_timer - dt)
    end
    if drop_through_timer > 0.0 then
        drop_through_timer = math.max(0.0, drop_through_timer - dt)
    end
    if jump_grace_timer > 0.0 then
        jump_grace_timer = math.max(0.0, jump_grace_timer - dt)
    end

    local on_ground = (ground_contacts > 0 or (one_way_ground_timer > 0.0 and drop_through_timer <= 0.0))
        and jump_grace_timer <= 0.0
    local on_left_wall = wall_left_contacts > 0
    local on_right_wall = wall_right_contacts > 0

    local vel = physics:velocity()
    last_fall_speed = math.max(0.0, vel.y)

    if wall_jump_lock > 0.0 then
        wall_jump_lock = math.max(0.0, wall_jump_lock - dt)
    else
        local target = move_dir * move_speed
        local accel = on_ground and 40.0 or 18.0
        vel.x = vel.x + (target - vel.x) * math.min(accel * dt, 1.0)
    end

    local pressing_left = move_dir < -0.1
    local pressing_right = move_dir > 0.1
    local wall_slide = (on_left_wall and pressing_left) or (on_right_wall and pressing_right)
    if not wall_slide and (on_left_wall or on_right_wall) and wall_stamina > 0.0 then
        wall_slide = true
    end
    if (not on_ground) and wall_slide and wall_stamina > 0.0 and vel.y > wall_slide_speed then
        vel.y = wall_slide_speed
    end

    if not on_ground and vel.y > 0.0 then
        vel.y = math.min(vel.y + fall_speed * dt, fall_speed)
    end

    if not on_ground and vel.y > -hang_speed and vel.y < hang_speed and hang_timer > 0.0 then
        vel.y = 0.0
        hang_timer = math.max(0.0, hang_timer - dt)
    end

    physics:set_velocity(vel)

    if wall_slide and wall_stamina > 0.0 and not on_ground then
        wall_slide_timer = wall_slide_timer + dt
        if wall_slide_timer >= wall_slide_interval then
            local world = ctx:world()
            if world ~= nil then
                local pos = world:position(ctx:entity())
                local offset_x = on_left_wall and -12.0 or 12.0
                emit_effect("wall_slide", pos.x + offset_x, pos.y + 10.0)
            end
            wall_slide_timer = 0.0
        end
    else
        wall_slide_timer = 0.0
    end

    if jump_buffer > 0.0 then
        jump_buffer = math.max(0.0, jump_buffer - dt)
    end

    if jump_pressed then
        local can_jump = on_ground or coyote_timer > 0.0
        local jump_x = 0.0
        local wall_jump = false
        if not can_jump then
            if on_left_wall and pressing_left and wall_stamina > 0.0 then
                can_jump = true
                wall_jump = true
                jump_x = wall_jump_x
            elseif on_right_wall and pressing_right and wall_stamina > 0.0 then
                can_jump = true
                wall_jump = true
                jump_x = -wall_jump_x
            end
        end

        if can_jump then
            vel = physics:velocity()
            vel.y = 0.0
            if wall_jump then
                vel.x = jump_x
                wall_jump_lock = 0.12
            end
            physics:set_velocity(vel)
            local impulse = wall_jump and wall_jump_impulse or jump_impulse
            physics:apply_impulse(vec2(0.0, -impulse))
            if wall_jump then
                local world = ctx:world()
                if world ~= nil then
                    local pos = world:position(ctx:entity())
                    local offset_x = jump_x > 0 and -16.0 or 16.0
                    emit_effect("wall_jump", pos.x + offset_x, pos.y)
                end
            end
            jump_hold_active = true
            jump_hold_timer = jump_max_hold
            hang_timer = hang_time
            jump_grace_timer = 0.08
            coyote_timer = 0.0
            last_jump_wall = wall_jump
            jump_pressed = false
            jump_buffer = 0.0
        end
    end

    if jump_hold_active and jump_hold_timer > 0.0 then
        local hold_boost = (last_jump_wall and wall_jump_impulse or jump_impulse)
            * (last_jump_wall and wall_jump_hold_boost or jump_hold_boost)
        physics:apply_impulse(vec2(0.0, -hold_boost * dt))
        jump_hold_timer = math.max(0.0, jump_hold_timer - dt)
    end

    if not jump_hold_active and vel.y < 0.0 then
        vel.y = vel.y * (1.0 - jump_cut)
        physics:set_velocity(vel)
    end

    if on_ground then
        coyote_timer = coyote_time
        wall_stamina = math.min(wall_stamina_max, wall_stamina + dt * 1.6)
        if jump_grace_timer <= 0.0 then
            jump_hold_active = false
            jump_hold_timer = 0.0
            hang_timer = 0.0
            last_jump_wall = false
        end
    else
        coyote_timer = math.max(0.0, coyote_timer - dt)
        if (on_left_wall or on_right_wall) and wall_stamina > 0.0 then
            wall_stamina = math.max(0.0, wall_stamina - dt * 0.5)
        end
    end

    if jump_buffer > 0.0 and (on_ground or coyote_timer > 0.0) then
        jump_pressed = true
    end

    if not was_grounded and on_ground then
        local impact = math.max(0.0, last_fall_speed - 320.0)
        if impact > 0.0 then
            local world = ctx:world()
            if world ~= nil then
                local pos = world:position(ctx:entity())
                emit_effect("landing", pos.x, pos.y)
                local intensity = math.min(1.0, impact / 500.0) * 6.0
                shake_camera(intensity, 0.18)
            end
        end
    end
    was_grounded = on_ground

    local world = ctx:world()
    if world ~= nil then
        prev_pos = world:position(ctx:entity())
        if prev_pos.y > respawn_y then
            request_respawn()
        end
    end
end

function on_post_physics(ctx, dt)
    local physics = ctx:physics()
    if physics == nil then
        return
    end
    local world = ctx:world()
    if world == nil then
        return
    end

    local half_height = param("half_height", 28.0)
    local respawn_y = param("respawn_y", 740.0)
    local pos = world:position(ctx:entity())
    local vel = physics:velocity()
    last_fall_speed = math.max(0.0, vel.y)

    local on_ground = ground_contacts > 0 or (one_way_ground_timer > 0.0 and drop_through_timer <= 0.0)

    local transform = ctx:transform()
    if transform ~= nil then
        local tables = world:find_all_by_tag("one_way")
        local falling = vel.y > 10.0
        for _, entity in ipairs(tables) do
            local size = world:collider_size(entity)
            if size.x > 0 or size.y > 0 then
                local block_pos = world:position(entity)
                local top = block_pos.y - size.y * 0.5
                local bottom_prev = prev_pos.y + half_height
                local bottom_now = pos.y + half_height
                local within_x = math.abs(pos.x - block_pos.x) <= size.x * 0.5 - 4.0
                if drop_through_timer <= 0.0 and falling and within_x and bottom_prev <= top and bottom_now >= top then
                    pos.y = top - half_height
                    vel.y = 0.0
                    transform:set_position(pos)
                    physics:set_velocity(vel)
                    on_ground = true
                    one_way_ground_timer = 0.12
                    break
                end
            end
        end
    end

    if drop_through_timer > 0.0 and vel.y < 120.0 then
        vel.y = 120.0
        physics:set_velocity(vel)
    end

    if not was_grounded and on_ground then
        local impact = math.max(0.0, last_fall_speed - 320.0)
        if impact > 0.0 then
            emit_effect("landing", pos.x, pos.y)
            local intensity = math.min(1.0, impact / 500.0) * 6.0
            shake_camera(intensity, 0.18)
        end
    end
    was_grounded = on_ground

    if pos.y > respawn_y then
        request_respawn()
    end
end
