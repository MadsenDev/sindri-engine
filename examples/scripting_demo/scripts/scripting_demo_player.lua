-- Simple player controller used by the scripting demo.
-- Horizontal movement uses WASD/arrow keys; Space provides a small hop.

-- Defaults tuned for the Sindri scripting demo scale.
local SPEED = params.speed or 200.0
local JUMP = params.jump or 75.0

function on_start(self)
    print("[player script] loaded scripting_demo_player.lua")
    print("[player script] params: SPEED=" .. tostring(SPEED) .. ", JUMP=" .. tostring(JUMP))
    local sprite = self:sprite()
    if sprite ~= nil then
        sprite:set_tint({0.35, 0.9, 1.0, 1.0})
    end
end

-- Track grounded state, wall contact, and jump cooldown
local is_grounded = true -- Assume we start on ground
local is_on_wall = false
local wall_direction = 0 -- -1 for left wall, 1 for right wall
local jump_cooldown = 0.0
local wall_jump_cooldown = 0.0

function on_collision_enter(self, other_entity)
    -- When we collide with something, check if we're on ground or wall
    local physics = self:physics()
    if physics then
        local vel = physics:velocity()
        local pos = self:position()
        
        -- Check if we're on ground (vertical velocity near zero)
        if vel.y >= -5.0 and vel.y <= 10.0 then
            is_grounded = true
        end
        
        -- Check if we're on a wall (horizontal collision while in air)
        -- We're on a wall if we're not grounded and have horizontal velocity near zero
        if not is_grounded and vel.y < 0.0 then -- Falling
            -- Check if we're touching a wall by looking at our position
            -- Left wall would be around x < 100, right wall around x > 860
            if pos.x < 100.0 then
                is_on_wall = true
                wall_direction = -1 -- Left wall, jump right
            elseif pos.x > 860.0 then
                is_on_wall = true
                wall_direction = 1 -- Right wall, jump left
            end
        end
    end
end

function on_collision_exit(self, other_entity)
    -- When we leave a collision, we might be in the air
    local physics = self:physics()
    if physics then
        local vel = physics:velocity()
        -- If we're moving up, definitely not grounded
        if vel.y < -10.0 then
            is_grounded = false
        end
        -- Reset wall contact when leaving collision
        is_on_wall = false
    end
end

function on_fixed_update(self, fixed_dt)
    local physics = self:physics()
    if not physics then return end

    -- Update cooldowns
    if jump_cooldown > 0.0 then
        jump_cooldown = jump_cooldown - fixed_dt
    end
    if wall_jump_cooldown > 0.0 then
        wall_jump_cooldown = wall_jump_cooldown - fixed_dt
    end

    local input = self:input()
    local move_x = 0.0
    if input:is_key_down("A") or input:is_key_down("Left") then
        move_x = move_x - 1.0
    end
    if input:is_key_down("D") or input:is_key_down("Right") then
        move_x = move_x + 1.0
    end

    -- Get current velocity and preserve Y component
    local current_vel = physics:velocity()
    local vel_y = current_vel.y or 0.0
    local pos = self:position()
    
    -- Update grounded state based on velocity
    -- If we're moving up significantly, we're definitely not grounded
    if vel_y < -15.0 then
        is_grounded = false
    elseif vel_y >= -2.0 and vel_y <= 5.0 then
        -- Very small velocity range - likely on ground
        -- (collision events will confirm)
    end
    
    -- Update wall contact based on position and velocity
    -- Only consider on wall if falling (not rising) and near wall position
    if not is_grounded and vel_y >= -50.0 then -- In air and not rising too fast
        -- Check if we're near a wall (left wall around x < 100, right wall around x > 860)
        -- Also check if we're moving toward the wall (for sticking)
        local moving_toward_wall = false
        if pos.x < 100.0 then
            -- Left wall - check if moving left
            if move_x < 0 then
                is_on_wall = true
                wall_direction = -1 -- Left wall, jump right
                moving_toward_wall = true
            else
                -- Not moving toward wall, don't stick
                is_on_wall = false
            end
        elseif pos.x > 860.0 then
            -- Right wall - check if moving right
            if move_x > 0 then
                is_on_wall = true
                wall_direction = 1 -- Right wall, jump left
                moving_toward_wall = true
            else
                -- Not moving toward wall, don't stick
                is_on_wall = false
            end
        else
            is_on_wall = false
        end
    else
        is_on_wall = false
    end
    
    -- Apply wall slide friction when on wall (only if moving toward wall)
    if is_on_wall and vel_y > 0.0 then
        -- Reduce downward velocity for wall sliding effect
        local slide_vel = vel_y * 0.5 -- 50% reduction for wall slide
        -- When on wall and moving toward it, allow some horizontal movement to stick
        physics:set_velocity(vec2(move_x * SPEED * 0.3, slide_vel))
    else
        -- Set velocity with horizontal movement and preserved Y
        physics:set_velocity(vec2(move_x * SPEED, vel_y))
    end

    -- Handle jumping: ground jump or wall jump
    if input:is_key_pressed("Space") then
        -- Wall jump: jump off wall when touching it
        if is_on_wall and wall_jump_cooldown <= 0.0 then
            -- Jump away from wall with VERY strong horizontal push and upward impulse
            local wall_jump_horizontal = wall_direction * SPEED * 3.5 -- Much stronger push away from wall
            local wall_jump_vertical = -JUMP * 1.2 -- Strong vertical jump
            physics:apply_impulse(vec2(wall_jump_horizontal, wall_jump_vertical))
            is_on_wall = false
            is_grounded = false
            wall_jump_cooldown = 0.15 -- Short cooldown for chaining
            jump_cooldown = 0.1 -- Small cooldown for ground jump too
        -- Ground jump: normal jump from ground
        elseif jump_cooldown <= 0.0 then
            -- Very strict check: only allow jump if velocity is very close to zero
            if vel_y >= -1.0 and vel_y <= 5.0 then
                physics:apply_impulse(vec2(0.0, -JUMP))
                is_grounded = false -- Immediately mark as not grounded after jumping
                jump_cooldown = 0.2 -- 0.2 second cooldown to prevent spam
            end
        end
    end
end

